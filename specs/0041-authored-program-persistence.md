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

CREATE INDEX authored_programs_user_created_idx
    ON authored_programs (user_id, created_at DESC);
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

`POST` deserializes into `AuthoredProgram`, then calls the domain validator
before touching the database. `materialize` is the only public entry point that
validates, so v1 validates by calling `materialize` with an **empty** `E1rmMap`
and a default plate: it exercises every `AuthorError` check while the `None`-load
path makes the e1RM irrelevant, and the result is discarded.

> **Note for the architect:** this is deliberate but slightly indirect — it
> validates via a function whose primary job is materialization. The alternative
> is exporting `authoring::validate` publicly from `core`. I lean toward
> exporting `validate` as the honest interface; flagging it for your call.

Each `AuthorError` variant maps to a fixed `&'static str` token on
`ApiError::Unprocessable { reason }` → `422`:

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

```rust
let summary = /* R-0015 path: sessions + measurements + target → summarize(...) */;
let e1rm: E1rmMap = summary
    .lifts
    .iter()
    .map(|l| (lift_key(&l.name), l.current_e1rm))
    .collect();
let cycle = materialize(&program, &e1rm, plate_kg)?;
```

`LiftSummary.current_e1rm` is a plain `f64`, so every lift the caller has trained
contributes a value. A lift the caller has **never** trained is simply absent from
the map, and `materialize` already emits `target_load_kg: None` for it — AC7's
null-load path falls out of the existing domain behaviour with no special casing
and no fabricated number.

Reuses `summary`'s existing input-fetching rather than a second e1RM
implementation, so there is one e1RM definition in the system (R-0015's).

`plate_kg` is an optional query parameter defaulting to `2.5` server-side
(OQ-1), validated by `materialize` itself (`BadPlate` → `422`).

### 2.6 Shared keying — one core change

`periodize::lift_key` is currently `pub(crate)`, so the api crate cannot reuse
it. Inlining `name.trim().to_lowercase()` in the api crate would recreate exactly
the keying drift SPEC-0039 §2.5 set out to prevent. Therefore: widen `lift_key`
to `pub` and re-export it from `lib.rs`. Behaviour is unchanged; this is a
visibility change only, and it keeps one keying rule across `aggregate`,
`periodize`, `authoring` and now the api crate.

### 2.7 DTOs

```rust
#[derive(Deserialize)]
struct CreateRequest { program: AuthoredProgram }

#[derive(Serialize)]
struct ProgramResponse {           // fetch-one (AC5) + create (AC2)
    id: Uuid,
    name: String,
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
pub(crate) async fn create(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateRequest>,
) -> ApiResult<(StatusCode, Json<ProgramResponse>)> {
    validate_program(&body.program).map_err(to_unprocessable)?;   // §2.4, before any write
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
    let program = load_owned(&state.pool, id, user.user_id).await?;   // 404 on non-owner
    let e1rm = caller_e1rm(&state.pool, user).await?;                 // §2.5
    let cycle = materialize(&program, &e1rm, q.plate_kg.unwrap_or(DEFAULT_PLATE_KG))
        .map_err(to_unprocessable)?;
    Ok(Json(cycle))
}
```

`load_owned` is the single ownership-scoped read used by both `fetch_one` and
`materialized`, so AC6 cannot be satisfied in one place and missed in the other.

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

1. each endpoint without a JWT → `401` (AC8)
2. create valid program → `201`, id returned (AC2)
3. create with a duplicate exercise name → `422 duplicate_exercise` (AC3)
4. create with `load_pct = 1.5` → `422 bad_intensity` (AC3)
5. create with an empty schedule → `422 no_schedule_days` (AC3)
6. create with a schedule entry naming an unknown exercise → `422
   unknown_exercise` (AC3)
7. list returns only the caller's rows, newest first (AC4)
8. fetch-one returns the stored program (AC5)
9. second user fetching the first user's id → `404` (AC6)
10. materialized after logging a Squat session → plate-rounded `target_load_kg`
    off the caller's e1RM (AC7)
11. materialized for a program whose lift was never trained → `target_load_kg:
    null` (AC7)
12. create → fetch-one round-trip deserializes to an `AuthoredProgram` equal to
    the one sent (AC9)

## 8. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-29 | Whole `AuthoredProgram` in one JSONB column | The domain model is already the serialization contract; a relational shred would duplicate it and drift. |
| 2026-07-29 | Denormalize `name` onto the row | The list endpoint stays a pure SQL projection with no JSONB deserialization. |
| 2026-07-29 | Non-owner → `404` through a single `load_owned` | Ownership as absence, enforced in one place so it cannot be half-applied. |
| 2026-07-29 | `AuthorError` → fixed `&'static str` tokens | CLAUDE.md §6 forbids stringly-typed errors; accepts losing the dynamic name, which the client already has. |
| 2026-07-29 | Build `E1rmMap` from R-0015's `summarize` | One e1RM definition in the system; no second implementation to drift. |
| 2026-07-29 | Widen `periodize::lift_key` to `pub` | One keying rule across core and api; inlining `trim().to_lowercase()` would recreate the drift SPEC-0039 §2.5 prevented. |

## 9. Architect review — ACCEPT-WITH-CHANGES (2026-07-29)

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
