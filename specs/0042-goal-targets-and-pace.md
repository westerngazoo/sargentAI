# SPEC-0042 — Goal targets & pace tracking (`core::goals` + `api::goals`)

- **Status:** Draft
- **Realizes:** R-0042
- **Author:** Claude (main session)
- **Created:** 2026-08-08
- **Depends on:** SPEC-0015 (`LiftSummary`, `BodyTrend`, `Adherence` — the
  measured signal), SPEC-0041 (the persistence + ownership pattern this
  mirrors, and `current_e1rm`'s windowing precedent).
- **Module(s):** `backend/crates/core/src/goals.rs` (new — pure);
  `backend/crates/api/src/goals/{mod,handlers}.rs` (new);
  `backend/migrations/00010_goals.sql` (new).

## 1. Motivation

Realizes [R-0042](../requirements/0042-goal-targets-and-pace.md): a dated
target plus the answer to "am I on pace?". The signal already exists —
R-0015's `summarize` produces per-lift e1RM trends, a body trend, and
adherence. This spec adds the *target* to compare it against and the pure math
that turns (baseline, target, trend, today) into a typed status.

## 2. Design

### 2.1 Domain model (pure, `core::goals`)

```rust
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum GoalKind {
    Strength { lift: String, target_e1rm_kg: f64 },
    Body { target_weight_kg: Option<f64>, target_body_fat_pct: Option<f64> },
    Consistency { sessions_per_week: u32 },
}

/// What was true when the goal was set — captured server-side (AC1), never
/// client-supplied. For Consistency the baseline is trivially zero-valued and
/// present only for shape uniformity.
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Baseline {
    Strength { e1rm_kg: Option<f64> },          // None: lift untrained at creation
    Body { weight_kg: Option<f64>, body_fat_pct: Option<f64> },
    Consistency,
}

pub struct Goal {
    pub kind: GoalKind,
    pub baseline: Baseline,
    pub set_on: NaiveDate,
    pub target_date: NaiveDate,
}
```

All model types derive `Clone, Debug, PartialEq, Serialize, Deserialize`.

### 2.2 Pace assessment (AC4–AC7)

One pure entry point:

```rust
/// # Errors
/// [`GoalError::KindMismatch`] when `signal` does not match `goal.kind`.
pub fn assess(goal: &Goal, signal: &PaceSignal, today: NaiveDate)
    -> Result<PaceReport, GoalError>;

/// The already-aggregated observations `assess` consumes. The api layer builds
/// this from R-0015's `summarize` output; core never touches a database.
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PaceSignal {
    Strength { current_e1rm_kg: Option<f64>, slope_kg_per_week: Option<f64>, sessions: u32 },
    Body { current_weight_kg: Option<f64>, weight_slope_kg_per_week: Option<f64>,
           current_body_fat_pct: Option<f64>, body_fat_slope: Option<f64>,
           measurements: u32, span_days: u32 },
    Consistency { weekly_days: Vec<(NaiveDate, u32)> },
}

pub struct PaceReport {
    pub status: PaceStatus,
    pub baseline: Option<f64>,
    pub current: Option<f64>,
    pub target: f64,
    pub required_per_week: Option<f64>,   // None once terminal
    pub observed_per_week: Option<f64>,
    pub projected_at_deadline: Option<f64>,
    pub weeks_remaining: f64,             // 0 when past the deadline
}

#[serde(rename_all = "snake_case", tag = "status", content = "detail")]
pub enum PaceStatus {
    Ahead,
    OnTrack,
    Behind,
    AtRisk,
    Achieved,
    Missed,
    InsufficientData { reason: InsufficientReason },
}

#[serde(rename_all = "snake_case")]
pub enum InsufficientReason {
    NoBaseline,          // lift untrained / no measurement when the goal was set
    TooFewObservations,  // below the per-kind floor (§2.3)
    NoTrend,             // observations exist but no slope could be established
}
```

**The math (OQ-1: linear, deliberately).** With signed direction
`dir = signum(target − baseline)` (AC7 — a cut has `dir = −1`):

```text
required_per_week  = (target − baseline) / weeks_total
projected          = current + observed_per_week × weeks_remaining
gap                = dir × (projected − target)      // ≥ 0 ⇒ making it
```

- `Achieved` — `dir × (current − target) ≥ 0`, any time. Terminal.
- Past `target_date` and not achieved → `Missed`. Terminal (AC6). Terminal
  reports carry `required_per_week: None` and stop projecting.
- Otherwise, with `band = 0.25 × |required_per_week| × weeks_remaining`
  (OQ-2 — a fixed 25 % tolerance on the *remaining* required change, so the
  band tightens as the deadline nears):
  - `gap ≥ band` → `Ahead`
  - `gap ≥ −band` → `OnTrack`
  - `gap < −band` and (`weeks_remaining < 2` or observed pace is zero/wrong-
    signed while ≥ 25 % of the goal remains) → `AtRisk`
  - else → `Behind`

Linear-required-pace is recorded as a simplification: strength gains
decelerate, so late-goal `Behind` may be optimistic. A wrong curve is worse
than an honest straight line; revisit with logged data.

**Consistency** has no projection: `observed_per_week` is the mean of full
ISO weeks since `set_on`, compared to `sessions_per_week` with the same 25 %
band; `Achieved` only at the deadline (you cannot finish early — the point is
sustaining it), so before the deadline the ceiling is `Ahead`.

### 2.3 Insufficient data (AC5)

Floors, per kind — below them `assess` returns `InsufficientData`, never a
projection:

| Kind | Floor |
|---|---|
| Strength | baseline present, `sessions ≥ 3`, slope present |
| Body | baseline present, `measurements ≥ 3` spanning `span_days ≥ 14`, slope present |
| Consistency | ≥ 1 completed ISO week since `set_on` |

Body's floor is higher than strength's because scale weight is noisier than
e1RM (OQ-3). The api layer passes observation counts through `PaceSignal`;
core owns the thresholds as `pub const`s beside R-0039's magnitude caps.

### 2.4 Goal validation (typed, never a panic)

```rust
pub enum GoalError {
    TargetDateNotAfterCreation,
    TargetDateTooFar,        // > MAX_GOAL_HORIZON_DAYS (730) — magnitude cap, AC5 lesson from R-0041
    NonFiniteTarget,
    NonPositiveTarget,
    BodyTargetEmpty,         // Body with neither field set (AC2)
    BlankLift,
    LiftTooLong,             // > 80 chars
    ZeroSessions,            // Consistency with sessions_per_week == 0
    TooManySessions,         // > 14/week — magnitude cap
    KindMismatch,            // assess() given a signal of the wrong kind
}
```

`pub fn validate(kind: &GoalKind, set_on: NaiveDate, target_date: NaiveDate)
-> Result<(), GoalError>` — called by the api on create, exhaustive-matched to
fixed `reason` tokens exactly as R-0041's `to_unprocessable` (no `_` arm; a new
variant is a compile error).

### 2.5 Storage (api)

`backend/migrations/00010_goals.sql`:

```sql
CREATE TABLE goals (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind         JSONB NOT NULL,     -- serialized GoalKind (tagged enum)
    baseline     JSONB NOT NULL,     -- serialized Baseline, captured at creation
    set_on       DATE NOT NULL,
    target_date  DATE NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_goals_user_created ON goals (user_id, created_at DESC, id DESC);
```

No status column: status is derived, and storing it would let it go stale
(AC8 — computing writes nothing). Whole-value JSONB for the tagged enums,
same rationale as R-0041's program column.

### 2.6 Endpoints

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/goals` | validate → capture baseline → store, `201` with the initial `PaceReport` |
| `GET` | `/goals` | caller's goals, newest first, each with its current `PaceReport` |
| `GET` | `/goals/:id` | one goal + report |
| `DELETE` | `/goals/:id` | remove a goal, `204` |

All owner-scoped via one `load_owned` (`WHERE id = $1 AND user_id = $2` →
`404`, never 403 — AC3), `AuthenticatedUser` before `Path`/`Json` in every
signature, `:id` route syntax. `DELETE` is a deliberate spec-level addition
beyond R-0042's ACs: AC8's "read-only" governs *status computation*, not the
user's ability to abandon a goal, and without it stale `Missed` rows accumulate
forever. Flagged for architect/owner; drop if either objects.

**Baseline capture on create** reuses the R-0041 path: `find_sessions_by_user`
→ `summarize`-derived signal → the relevant current value becomes the stored
`Baseline`. A strength goal for an untrained lift stores `e1rm_kg: None` and
will report `InsufficientData { NoBaseline }` until the lift is trained — it
does not fail creation (you may set a goal before your first squat session).

**Signal assembly on read** builds `PaceSignal` from the same `summarize`
output `/training-summary` uses, so the two endpoints can never disagree about
the trend. `GET /goals` runs `summarize` **once** and assesses every goal
against it — no per-goal recomputation.

### 2.7 Windowing honesty

Trends come from R-0015's 8-week window (`DEFAULT_WINDOW_WEEKS`), so
`observed_per_week` is the *recent* pace, not the lifetime average — that is
the right input for "will I make it from here". `current` for strength uses
the windowed e1RM; a lift idle 9+ weeks degrades to
`InsufficientData { NoTrend }` rather than projecting from stale data —
consistent with R-0041's null-load precedent.

## 3. Code outline

```rust
// api/goals/handlers.rs
pub(crate) async fn create(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateGoalRequest>,     // { kind, target_date }
) -> ApiResult<(StatusCode, Json<GoalResponse>)> {
    let today = Utc::now().date_naive();
    validate(&body.kind, today, body.target_date).map_err(|e| to_unprocessable(&e))?;
    let signal = signal_for(&state, &user, &body.kind).await?;   // shared with reads
    let baseline = Baseline::capture(&body.kind, &signal);       // pure, core
    let goal = Goal { kind: body.kind, baseline, set_on: today, target_date: body.target_date };
    let row = insert_goal(&state.pool, user.user_id, &goal).await?;
    let report = assess(&goal, &signal, today).map_err(|e| to_unprocessable(&e))?;
    Ok((StatusCode::CREATED, Json(GoalResponse::from_parts(row, goal, report))))
}
```

`GoalResponse { id, goal: Goal, report: PaceReport, created_at }` — the domain
value round-trips whole (the R-0041 AC9 pattern; no re-modelling, no duplicated
fields).

## 4. Non-goals

Per R-0042 §4: no auto-adjustment, no notifications, no trainer-assigned goals,
no gamification, no open-ended (undated) goals, no update/edit (delete +
recreate is the v1 mutation story), no new statistics — the only trend source
is R-0015's `summarize`.

## 5. Open questions — resolved

- **OQ-1:** linear required pace, recorded as a simplification (§2.2).
- **OQ-2:** fixed 25 % band on the remaining required change (§2.2) — tightens
  toward the deadline; a variance-based band needs data we don't have yet.
- **OQ-3:** per-kind floors (§2.3); body stricter than strength.
- **OQ-4:** multiple active goals allowed, conflicts permitted and *surfaced*
  (each reports independently); no cross-goal constraint solving in v1.

## 6. Acceptance criteria

AC1 baseline server-side (§2.6 capture); AC2 three kinds one shape (§2.1,
`BodyTargetEmpty`); AC3 ownership-as-absence (§2.6); AC4 typed status + numbers
(`PaceReport`); AC5 `InsufficientData` with reason, floors in §2.3; AC6
terminal states (§2.2); AC7 signed-direction math (§2.2 `dir`); AC8 no derived
state stored (§2.5), no writes on read; AC9 `core::goals` pure, explicit
`today` (§2.2); AC10 the test plan below.

## 7. Test plan

Core unit tests (no database) — table-driven over `(goal, signal, today) →
expected status` covering, per kind: ahead / on-track / behind / at-risk /
achieved-early / missed / each `InsufficientData` reason; the fat-loss
(`dir = −1`) case where weight *rising* is `Behind`; band boundary cases at
`gap = ±band`; consistency's no-early-`Achieved` rule; every `GoalError`
variant; `validate ↔ assess` non-overlap (a goal that validates never hits
`KindMismatch` with a matching signal).

Integration tests (`#[sqlx::test]`) — auth on every route incl. malformed id
→ 401; create → 201 with report; table-driven `GoalError` token sweep;
baseline is captured from *logged data, not the request* (log a squat, create
a goal, assert stored baseline equals the logged e1RM); ownership 404;
cross-user signal isolation; delete → 204 then 404; list newest-first
(membership + non-increasing timestamps, the R-0041 fix); round-trip equality
on the deserialized `Goal`.

## 8. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-08 | `assess` takes a `PaceSignal`, not `TrainingSummary` | Core stays decoupled from R-0015's full report shape; the api adapts once. |
| 2026-08-08 | Band = 25 % of *remaining* required change | Tightens naturally toward the deadline; one constant, no variance model. |
| 2026-08-08 | No stored status column | Derived state stored is derived state stale; AC8 makes reads pure. |
| 2026-08-08 | Untrained-lift goal creation succeeds with `None` baseline | Setting a goal before the first session is a legitimate flow; `InsufficientData` covers the gap honestly. |
| 2026-08-08 | `DELETE /goals/:id` added beyond R-0042's ACs | Abandoning a goal is user data hygiene, not status mutation; flagged for architect/owner veto. |
| 2026-08-08 | Consistency cannot `Achieved` early | The target is *sustaining* a rate; declaring victory at week 1 of 12 would be a lie. |

## Changelog

- _2026-08-08 — created (Draft)._
