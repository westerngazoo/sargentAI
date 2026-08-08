# SPEC-0041 — Authored program persistence & serving (`api::authored`)

- **Status:** Draft
- **Realizes:** R-0041
- **Author:** Claude (main session)
- **Created:** 2026-07-29
- **Depends on:** SPEC-0039 (`AuthoredProgram`, `materialize`, `AuthorError`),
  SPEC-0038 (load math, `E1rmMap`), SPEC-0015 (`LiftSummary.current_e1rm`).
- **Module(s):** `backend/crates/api/src/authored/{mod,handlers}.rs` (new);
  `backend/migrations/00009_authored_programs.sql` (new); a one-line visibility
  widening in `backend/crates/core/src/periodize.rs`.

## 1. Motivation

Realizes [R-0041](../requirements/0041-authored-program-persistence.md).
`core::authoring` landed in #76 as pure domain logic — complete, tested, and
unreachable. This spec gives it storage and four read/write endpoints so a client
can save an authored program and read it back materialized against the user's own
e1RM.

## 2. Design

### 2.1 Storage (AC1)

`backend/migrations/00009_authored_programs.sql`:

```sql
CREATE TABLE authored_programs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    program     JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- `id` is part of the ordering, not decoration: `created_at DESC` alone is not
-- a total order, so two programs created in the same tick could swap places
-- between requests. It also makes the index keyset-paginatable later.
CREATE INDEX idx_authored_programs_user_created
    ON authored_programs (user_id, created_at DESC, id DESC);
```

`program` holds the whole serialized `AuthoredProgram`. `name` is duplicated out
of the JSONB purely so the list query needs no deserialization (OQ-2). No
uniqueness constraint on `name` — the id is the identity (OQ-3). The index
matches AC4's `(owner, newest first)` access path exactly.

### 2.2 Endpoints

| Method | Path | Purpose | AC |
|---|---|---|---|
| `POST` | `/authored-programs` | validate + store, `201` | AC2, AC3 |
| `GET` | `/authored-programs` | caller's list, newest first | AC4 |
| `GET` | `/authored-programs/{id}` | one full program | AC5, AC6 |
| `GET` | `/authored-programs/{id}/materialized` | concrete cycle vs caller's e1RM | AC7 |

All four take `AuthenticatedUser`, so an absent/invalid JWT is `401` before any
handler body runs (AC8).

### 2.3 Ownership as absence (AC6)

Every read is `WHERE id = $1 AND user_id = $2`. A row that exists but belongs to
someone else produces zero rows and therefore `ApiError::NotFound` → `404`, on
the same code path as a genuinely missing id. There is no branch that could
return `403`, so id existence is not disclosed even by timing of a distinct
error.

### 2.4 Validation (AC3)

`POST` deserializes into `AuthoredProgram` and calls a **public domain
validator** before touching the database.

`core::authoring` is refactored so validation is a first-class entry point
rather than a side effect of materialization:

```rust
/// Every program-level check, independent of any e1RM or plate increment.
///
/// # Errors
/// [`AuthorError`] for a blank or duplicated exercise name, an empty or
/// unreferenced schedule, or an invalid work-set line.
pub fn validate(program: &AuthoredProgram) -> Result<(), AuthorError> {
    index(program).map(|_| ())
}

/// key → exercise, rejecting blank/duplicate names, then the schedule and
/// prescription checks. The single traversal `materialize` also needs.
fn index(program: &AuthoredProgram)
    -> Result<BTreeMap<String, &AuthoredExercise>, AuthorError>;
```

`materialize` becomes `let by_key = index(program)?; check_plate(plate_kg)?; …`
— one traversal, no duplicated logic, no wasted allocation.

This refactor is required, not cosmetic: the existing private `validate` takes
an index map *and* `plate_kg`, and `BlankExercise`/`DuplicateExercise` are
raised in `materialize`'s index loop rather than inside `validate`. Exporting it
unchanged would silently lose two of the eleven tokens.

Consequence: **`BadPlate` moves out of program validation** into
materialization, so it is reachable from `/materialized` only — ten tokens from
`POST`, one from `/materialized`.

A core unit test pins the two entry points together so they cannot drift:
`validate(p).is_ok() == materialize(p, &E1rmMap::new(), 2.5).is_ok()`.

**Program name** is *request-level* validation, not a domain invariant: blank,
whitespace-only, or over 120 chars → `ApiError::Validation { field: "name" }` →
`400`. Deliberately not a new `AuthorError` variant, which would push an API
concern into the domain model.

**Malformed bodies** surface as axum's `JsonRejection`: `400` for syntax, `422`
for shape, both with a plain-text body and **no `reason` field**. A client
switching on `reason` must tolerate its absence on a 422.

Each `AuthorError` variant maps to a fixed `&'static str` token on
`ApiError::Unprocessable { reason }` → `422`, via an **exhaustive match with no
`_` arm**, so a twelfth variant becomes a compile error rather than a silently
wrong token:

| `AuthorError` | `reason` token |
|---|---|
| `NoExercises` | `no_exercises` |
| `BlankExercise` | `blank_exercise` |
| `DuplicateExercise(_)` | `duplicate_exercise` |
| `NoScheduleDays` | `no_schedule_days` |
| `NoScheduledEntries` | `no_scheduled_entries` |
| `UnknownExercise(_)` | `unknown_exercise` |
| `EmptyWorkLines` | `empty_work_lines` |
| `BadIntensity` | `bad_intensity` |
| `BadReps` | `bad_reps` |
| `BadSets` | `bad_sets` |
| `BadPlate` | `bad_plate` |

Tokens are fixed strings, never free text (CLAUDE.md §6). The two variants
carrying a `String` lose that detail: `reason` is `&'static str`, and widening the
shared error type to carry a detail field is out of scope here. The offending
name is recoverable client-side — the client holds the program it just sent.

### 2.5 Materialization (AC7)

The e1RM derivation lives in **core**, not in the api crate:

```rust
// core/src/aggregate.rs
/// Per-lift current e1RM over the window, keyed for
/// [`materialize`](crate::authoring::materialize) lookup.
#[must_use]
pub fn current_e1rm(today: NaiveDate, window_weeks: u32, sessions: &[WorkoutSession]) -> E1rmMap;

/// The window every consumer of "what can this user currently lift" shares.
pub const DEFAULT_WINDOW_WEEKS: u32 = 8;
```

so the handler is:

```rust
let sessions = db::find_sessions_by_user(&state.pool, user.user_id).await?;
let e1rm = current_e1rm(today, DEFAULT_WINDOW_WEEKS, &sessions);
let cycle = materialize(&program, &e1rm, plate_kg)?;
```

**Deliberately not** reusing `summary::handlers::fetch_inputs`, for three
reasons: it is private (reuse would mean widening a sibling api module's
internals — a lateral dependency, the opposite of pointing inward); it also
fetches and deserializes the active *generated* program, so a corrupt
`user_programs` row would produce a **500 from an authored-programs endpoint
that has nothing to do with it**; and it computes muscle volume, adherence and
body trend, all of which this endpoint discards. `db::find_sessions_by_user` is
already `pub`.

**AC7's null-load path is windowed.** `current_e1rm` considers only the last
`DEFAULT_WINDOW_WEEKS`, so "a lift the caller has never trained" is really "has
not trained in 8 weeks" — a lift last trained 9 weeks ago yields
`target_load_kg: null`. That is the correct behaviour (a two-month-stale e1RM is
a bad load anchor) but it is a decision, not an accident, and the AC7 mapping
says so. The constant lives in one place; `summary/handlers.rs` switches to it
rather than keeping a second `8`.

`plate_kg` is an optional query parameter defaulting to `2.5` server-side
(OQ-1), validated by `materialize` itself (`BadPlate` → `422`).

### 2.6 Keying stays in core

`periodize::lift_key` stays `pub(crate)`. Putting `current_e1rm` in core (§2.5)
means the api crate never needs to key a lift at all, so there is nothing to
widen and no second copy of the rule to drift.

Note for the record: SPEC-0039 §2.5's "one keying rule" claim is **not true
today** — `aggregate::per_lift` inlines `trim()` + `to_lowercase()` rather than
calling `lift_key`. Making `lift_key` public would have advertised a shared rule
that isn't shared. Reconciling `per_lift` to call `lift_key` is worth doing but
is out of scope here; this spec simply avoids adding a third copy.

### 2.7 DTOs

```rust
#[derive(Deserialize)]
struct CreateRequest { program: AuthoredProgram }

#[derive(Serialize)]
struct ProgramResponse {           // fetch-one (AC5) + create (AC2)
    id: Uuid,
    // No `name` field: it is already inside `program`, and carrying it twice in
    // one payload is two sources for one fact that will diverge the day update
    // lands. The denormalized column exists for the list query (below), not for
    // this response.
    program: AuthoredProgram,      // full round-trip value (AC9)
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct ProgramListItem {           // list (AC4) — no deserialization needed
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

`ProgramResponse.program` is the `AuthoredProgram` itself, not a re-modelled
copy, which is what makes AC9 structural: the type that went in is the type that
comes out.

## 3. Code outline

```rust
// authored/mod.rs — self-contained module, mirroring measurements/synthetic.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/authored-programs", post(handlers::create).get(handlers::list))
        // `:id`, not `{id}` — axum 0.7 (matchit 0.7) treats braces as a literal
        // segment, so `{id}` would never match a real UUID. Brace captures are
        // an axum 0.8 feature. Matches the convention in `archetype`/`photo`.
        .route("/authored-programs/:id", get(handlers::fetch_one))
        .route("/authored-programs/:id/materialized", get(handlers::materialized))
}

// authored/handlers.rs
// Extractor order is an invariant, not a style choice: axum runs extractors
// left to right, so `AuthenticatedUser` must precede `Path`/`Query`/`Json` in
// every signature. Reversed, `GET /authored-programs/not-a-uuid` with no token
// would return axum's 400 instead of AC8's 401.
pub(crate) async fn create(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateRequest>,
) -> ApiResult<(StatusCode, Json<ProgramResponse>)> {
    check_name(&body.program.name)?;                              // §2.4, 400
    validate(&body.program).map_err(to_unprocessable)?;           // §2.4, before any write
    let row: ProgramRow = sqlx::query_as(
        "INSERT INTO authored_programs (user_id, name, program) VALUES ($1, $2, $3) \
         RETURNING id, name, program, created_at, updated_at",
    )
    .bind(user.user_id.0)
    .bind(&body.program.name)
    .bind(serde_json::to_value(&body.program).map_err(internal)?)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(row.try_into()?)))
}

pub(crate) async fn materialized(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Query(q): Query<MaterializeQuery>,       // plate_kg: Option<f64>
) -> ApiResult<Json<MaterializedCycle>> {
    let row = load_owned(&state.pool, id, user.user_id).await?;       // 404 on non-owner
    let program = row.program()?;
    let sessions = db::find_sessions_by_user(&state.pool, user.user_id).await?;
    let e1rm = current_e1rm(today, DEFAULT_WINDOW_WEEKS, &sessions);  // §2.5
    let cycle = materialize(&program, &e1rm, q.plate_kg.unwrap_or(DEFAULT_PLATE_KG))
        .map_err(to_unprocessable)?;
    Ok(Json(cycle))
}

/// The one ownership-scoped read. Returns the **row**, not the program, so
/// `fetch_one` can build its response from the same query rather than issuing a
/// second one — otherwise the single-enforcement-point claim is false in
/// exactly the place it matters.
async fn load_owned(pool: &PgPool, id: Uuid, user_id: UserId) -> ApiResult<ProgramRow> {
    sqlx::query_as(
        "SELECT id, name, program, created_at, updated_at \
         FROM authored_programs WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id.0)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)
}
```

`ProgramRow::program(&self) -> ApiResult<AuthoredProgram>` deserializes the
JSONB (`Internal` on failure), and `TryFrom<ProgramRow> for ProgramResponse`
builds the response. Both `fetch_one` and `materialized` go through
`load_owned`, so AC6 cannot be satisfied in one and missed in the other.

## 4. Non-goals

Per R-0041 §4: no builder UI, no update/delete, no roles or assignment, no
community/subscriptions, no meal plans, no weekday calendars, and no behavioural
change to `core::authoring` (§2.6 widens one helper's visibility only).

## 5. Open questions

- **OQ-1 (resolved):** `plate_kg` is an optional query parameter on
  `/materialized`, defaulted to `2.5` server-side — a kg/lb gym is a client
  concern, not a migration.
- **OQ-2 (resolved):** `name` is denormalized onto the row so the list endpoint
  never deserializes JSONB; the full program is returned only by fetch-one.
- **OQ-3 (resolved):** duplicate program names are allowed; the id is identity.

## 6. Acceptance criteria

Maps R-0041 AC1–AC10: table + index (AC1); `POST` validates-then-stores, `201`
(AC2); `AuthorError` → fixed-token `422` (AC3); owner-scoped list newest-first
(AC4); fetch-one (AC5); non-owner `404` via `load_owned` (AC6); materialize off
R-0015 e1RM with the absent-lift `null` path (AC7); `AuthenticatedUser` on all
four routes (AC8); `AuthoredProgram` in/out as the same type (AC9); the
integration suite in §7 (AC10).

## 7. Test plan

`#[sqlx::test(migrations = "../../migrations")]` integration tests:

1. each endpoint without a JWT → `401` (AC8), including a **malformed** id with
   no token → still `401`, pinning the extractor-order invariant in §3
2. create valid program → `201`, id returned (AC2)
3. **table-driven rejection sweep (AC3, AC10).** One `(mutation, expected_token)`
   array covering **all ten** program-level tokens, one `POST` each:
   `no_exercises`, `blank_exercise`, `duplicate_exercise`, `no_schedule_days`,
   `no_scheduled_entries`, `unknown_exercise`, `empty_work_lines`,
   `bad_intensity`, `bad_reps`, `bad_sets` — each asserting `422` **and** the
   exact `reason`
4. `/materialized?plate_kg=0` → `422 bad_plate` — the only path that reaches the
   eleventh token after §2.4's refactor
5. create with a blank / whitespace-only / >120-char name → `400` with
   `field: "name"` (§2.4, request-level)
6. list returns only the caller's rows, newest first (AC4)
7. list with zero rows → `200 []`, not `404`
8. fetch-one returns the stored program (AC5)
9. second user fetching the first user's id → `404` (AC6)
10. materialized after logging a Squat session → plate-rounded `target_load_kg`
    off the caller's e1RM (AC7)
11. materialized for a program whose lift was never trained → `target_load_kg:
    null` (AC7)
12. materialized uses **the caller's own** e1RM — seed sessions for user B and
    assert user A's cycle is unaffected (the cross-user leak AC6 does not cover)
13. create → fetch-one round-trip deserializes to an `AuthoredProgram` equal to
    the one sent (AC9). Asserted on the **deserialized value** via the derived
    `PartialEq`, never on JSON text: Postgres normalizes JSONB key order, so
    byte-for-byte equality is not the guarantee and testing it would be flaky

Core unit tests (no database):

14. `validate` rejects each `AuthorError` variant it owns
15. `validate(p).is_ok() == materialize(p, &E1rmMap::new(), 2.5).is_ok()` — pins
    the two entry points together (§2.4)
16. `to_unprocessable` maps all eleven variants to eleven **distinct** tokens

## 8. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-29 | Whole `AuthoredProgram` in one JSONB column | The domain model is already the serialization contract; a relational shred would duplicate it and drift. |
| 2026-07-29 | Denormalize `name` onto the row | The list endpoint stays a pure SQL projection with no JSONB deserialization. |
| 2026-07-29 | Non-owner → `404` through a single `load_owned` | Ownership as absence, enforced in one place so it cannot be half-applied. |
| 2026-07-29 | `AuthorError` → fixed `&'static str` tokens | CLAUDE.md §6 forbids stringly-typed errors; accepts losing the dynamic name, which the client already has. |
| 2026-07-29 | Build `E1rmMap` from R-0015's `summarize` | One e1RM definition in the system; no second implementation to drift. |
| 2026-08-04 | `lift_key` stays `pub(crate)`; `current_e1rm` moves into `core::aggregate` | Removes the api crate's need to key a lift at all. Supersedes the 2026-07-29 entry below, whose premise was wrong — `aggregate::per_lift` already inlines the rule, so `lift_key` was never the single shared path it claimed to be. |
| 2026-08-04 | Public `authoring::validate` over a shared `index()` | Validation coverage must not be a side effect of `materialize`'s implementation; a future short-circuit there would silently weaken `POST`. |
| 2026-08-04 | e1RM from `db::find_sessions_by_user`, not `summary`'s fetch path | Avoids a lateral api→api dependency on a private helper, and stops a corrupt generated-program row 500ing an unrelated endpoint. |
| 2026-08-04 | AC7's "never trained" means "not in the last 8 weeks" | A two-month-stale e1RM is a bad load anchor; recorded so the windowing is a decision rather than a surprise. |
| ~~2026-07-29~~ | ~~Widen `periodize::lift_key` to `pub`~~ | ~~One keying rule across core and api.~~ **Superseded** — see 2026-08-04 above. |

## 9. Architect review — ACCEPT-WITH-CHANGES (2026-07-29), all applied 2026-08-04

Every required change below has been folded into the sections above. Summary of
what moved, and why it mattered:

| # | Change | Where |
|---|---|---|
| 1 | Public `validate` over a shared `index()`, not validation-by-`materialize` | §2.4 |
| 2 | `db::find_sessions_by_user` + `core::aggregate::current_e1rm`, not `summary`'s private fetch | §2.5 |
| 3 | `lift_key` stays `pub(crate)` | §2.6 |
| 4 | `load_owned` returns the row | §3 |
| 5 | AC7 is windowed; constant hoisted to core | §2.5, §6 |
| 6 | `to_unprocessable` exhaustive, no `_` arm | §2.4 |
| 7 | Program-name validation at request level → 400 | §2.4 |
| 8 | `name` dropped from `ProgramResponse` | §2.7 |
| 9 | Test plan table-driven over all ten tokens + `bad_plate` | §7 |
| 10 | `ORDER BY created_at DESC, id DESC` + matching index | §2.1 |

Two knock-on corrections worth stating plainly, since both contradict what this
spec originally claimed:

- **`BadPlate` is no longer reachable from `POST`.** Splitting validation from
  materialization means plate validation belongs to the latter. The §2.4 table
  is therefore ten-plus-one, not eleven.
- **R-0041 §4's "no change to `core::authoring`" is now false**, and the
  requirement has been amended. The change is a refactor that extracts existing
  behaviour behind a public entry point — no behavioural change — but the
  non-goal as written did not survive, and pretending otherwise would make the
  requirement lie about its own scope.

### Original review record

Reviewed pre-implementation. The shape was accepted (self-contained module,
whole-value JSONB, ownership-as-absence, materialization delegated to pure
core). **Applied so far:** the axum route syntax fix in §3 (`:id`, not `{id}` —
under matchit 0.7 the brace form is a literal segment and would never match).

**Still to apply before implementation starts:**

1. **Export a real `validate` from `core::authoring`** rather than validating by
   calling `materialize` and discarding the result (§2.4). The proposed
   alternative does not work as-is: `validate` currently takes an index map and
   `plate_kg`, and `BlankExercise`/`DuplicateExercise` are raised in
   `materialize`'s index loop, not in `validate` — so exporting it unchanged
   would silently lose two of the eleven tokens. Refactor to
   `pub fn validate(&AuthoredProgram) -> Result<(), AuthorError>` over a shared
   `index()` helper. Consequence: `BadPlate` becomes reachable only from
   `/materialized`, so the §2.4 table splits ten/one.
2. **Do not reuse `summary::handlers::fetch_inputs`** (§2.5) — it is private, it
   drags in an unrelated `ApiError::Internal` path (a corrupt stored *generated*
   program would 500 this endpoint), and it computes three discarded aggregates.
   Use the already-`pub` `db::find_sessions_by_user` plus a new
   `core::aggregate::current_e1rm(today, window, &sessions) -> E1rmMap`.
3. **Do not widen `lift_key` to `pub`** (§2.6). `aggregate::per_lift` already
   inlines the rule, so the "one keying rule" claim is not true today; putting
   `current_e1rm` in core removes the need for api-side keying entirely.
4. **`load_owned` must return the row, not the program** (§3) — otherwise
   `fetch_one` needs a second query, which is exactly the half-applied
   enforcement §2.3 claims to prevent.
5. **AC7 is windowed.** e1RM comes from the last 8 weeks, so "a lift never
   trained" is really "not trained in 8 weeks". State it, and hoist the window
   constant to one place in `core::aggregate`.
6. **`to_unprocessable` must be an exhaustive match with no `_` arm**, so a
   twelfth `AuthorError` variant is a compile error rather than a wrong token.
7. **Validate the program `name`** (blank/whitespace/length) as request-level
   validation → 400, not a new `AuthorError` variant.
8. **Drop the duplicated `name` from `ProgramResponse`** — it appears both as the
   denormalized column and inside the nested `program`.
9. **§7 does not satisfy AC10** — it covers 4 of 11 rejections. Make it
   table-driven over all ten program-level tokens plus `?plate_kg=0`.
10. **List ordering needs a tiebreak** — `ORDER BY created_at DESC, id DESC`,
    with the index widened to match.

R-0041 §4 and §4 above must also be amended: "no change to `core::authoring`"
becomes false once item 1 lands.

## Changelog

- _2026-07-29 — created (Draft); architect review recorded (§9), route syntax
  corrected._
