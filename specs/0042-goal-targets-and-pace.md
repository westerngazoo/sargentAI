# SPEC-0042 — Goal targets & pace tracking (`core::goals` + `api::goals`)

- **Status:** Draft — architect-reviewed, all required changes applied (§9)
- **Realizes:** R-0042 (as amended 2026-08-08 — see its changelog)
- **Author:** Claude (main session)
- **Created:** 2026-08-08
- **Depends on:** SPEC-0015 (`LiftSummary`, `BodyTrend`, `Adherence` — the
  measured signal), SPEC-0041 (persistence + ownership pattern, windowing
  precedent).
- **Module(s):** `backend/crates/core/src/goals.rs` (new — pure);
  `backend/crates/api/src/goals/{mod,handlers}.rs` (new);
  `backend/migrations/00010_goals.sql` (new). No change to `aggregate.rs`.

## 1. Motivation

Realizes [R-0042](../requirements/0042-goal-targets-and-pace.md): a dated
target plus the answer to "am I on pace?". The signal already exists —
R-0015's `summarize`. This spec adds the target and the pure math turning
(goal, signal, date) into a typed status.

## 2. Design

### 2.1 Domain model (pure, `core::goals`)

```rust
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum GoalKind {
    Strength { lift: String, target_e1rm_kg: f64 },
    Body { target_weight_kg: Option<f64>, target_body_fat_pct: Option<f64> },
    Consistency { sessions_per_week: u32 },
}

/// What was measurable when the goal was set — captured server-side (AC1).
/// `None` means the metric had no data at creation; §2.2 late-binds it.
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Baseline {
    Strength { e1rm_kg: Option<f64> },
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

```rust
/// The R-0015 aggregates, borrowed as-is. Core owns every derivation
/// (slope presence, per-metric counts, spans) — the api adapts nothing.
pub struct GoalSignal<'a> {
    pub lifts: &'a [LiftSummary],
    pub body: &'a BodyTrend,
    pub adherence: &'a Adherence,
}

/// Pure and total: every input produces a report; degradation is a status
/// (`InsufficientData`), never an error. `today` is the only clock.
pub fn assess(goal: &Goal, signal: &GoalSignal<'_>, today: NaiveDate) -> PaceReport;

pub struct PaceReport {
    /// Most severe metric status under the pinned order (§2.2.1).
    pub status: PaceStatus,
    /// One per tracked metric: Strength and Consistency have exactly one;
    /// Body has one per set target field.
    pub metrics: Vec<MetricPace>,
}

#[serde(rename_all = "snake_case", tag = "metric")]
pub enum Metric {
    E1rmKg { lift: String },
    WeightKg,
    BodyFatPct,
    SessionsPerWeek,
}

pub struct MetricPace {
    pub metric: Metric,
    pub status: PaceStatus,
    pub baseline: Option<f64>,            // None when late-bound (§2.2.3)
    pub current: Option<f64>,
    pub target: f64,
    pub required_per_week: Option<f64>,   // None once terminal
    pub observed_per_week: Option<f64>,
    pub projected_at_deadline: Option<f64>,
    pub weeks_remaining: f64,             // clamped at 0
}

#[serde(rename_all = "snake_case", tag = "status", content = "detail")]
pub enum PaceStatus {
    Achieved, Ahead, OnTrack, Behind, AtRisk, Missed,
    InsufficientData { reason: InsufficientReason },
}

#[serde(rename_all = "snake_case")]
pub enum InsufficientReason {
    NoSignal,            // neither a stored baseline nor any current value
    TooFewObservations,  // below the per-kind floor (§2.3)
    NoTrend,             // observations exist; no slope could be established
}
```

#### 2.2.1 Severity order (pinned)

`Achieved < Ahead < OnTrack < Behind < AtRisk < Missed < InsufficientData`.
The report's overall `status` is the maximum across metrics. A Body goal with
weight `OnTrack` and body-fat `InsufficientData` therefore reports
`InsufficientData` overall while still carrying the weight metric's full
numbers — the honest reading of "I can't fully answer yet".

#### 2.2.2 The math (per metric)

Direction by explicit comparison — **never** `f64::signum`, whose `signum(0.0)
== 1.0` would silently pick a direction for an already-met target:

```text
dir = +1 if target > base_eff, −1 if target < base_eff
      (target == base_eff ⇒ the Achieved check below decides immediately)

weeks_total     = days(set_on → target_date) / 7.0
weeks_remaining = max(0, days(today → target_date)) / 7.0
required_per_week = (target − base_eff) / weeks_total
projected         = current + observed_per_week × weeks_remaining
gap               = dir × (projected − target)        // ≥ 0 ⇒ making it
band              = 0.25 × |required_per_week| × weeks_remaining
```

Day-count → `f64` casts carry `#[allow(clippy::cast_precision_loss)]` with a
"day counts are tiny" comment, matching `aggregate.rs`.

- `Achieved` — `dir × (current − target) ≥ 0`, including at creation (`POST`
  returns `201` with `Achieved` for an already-met target). **Before the
  deadline this is a current-state report, not a terminal fact**: regress and
  it reverts. Only post-deadline statuses are terminal (§2.2.4).
- Otherwise, pre-deadline:
  - `gap ≥ band` → `Ahead`
  - `gap ≥ −band` → `OnTrack`
  - `gap < −band` and (`weeks_remaining < 2`, or observed pace is zero /
    wrong-signed while the remaining fraction ≥ 0.25) → `AtRisk`
  - else → `Behind`

  The remaining fraction is `dir × (target − current) / (target − baseline)`
  and is **skipped** (that AtRisk arm disabled) when the baseline is late-bound
  or `|target − baseline|` is ~0 — no division by an unknown or zero span.
- **Endgame, documented:** as `weeks_remaining → 0` the band vanishes while
  `gap` converges to the fixed shortfall, so anyone short of target slides
  one-way into `AtRisk` in the final days. That is honest ("one day left,
  3 kg to go" *is* at risk) — no epsilon floor, by decision.

#### 2.2.3 Late-bound baseline

A stored `None` baseline is **not** a permanent `InsufficientData` trap. When
the stored value is `None` but the signal has a current value, `assess` uses
`base_eff = current`: required pace becomes `(target − current) /
weeks_remaining`, the report carries `baseline: None`, and the meaning is
"unknown start; here is the pace needed from here". `InsufficientData {
NoSignal }` is reserved for *neither* stored baseline *nor* current signal.
Nothing is written — AC8 holds.

#### 2.2.4 Deadlines and terminality (AC6)

Status is derived fresh from a trailing window, so "achieved is terminal"
cannot be a property of a stored flag — it must be a property of the *inputs*.
The api anchors the signal at `anchor = min(today, target_date)`: for an
expired goal it calls `summarize(goal.target_date, …)` over the same fetched
rows, making post-deadline status a **stable, deterministic function of
history**. Post-deadline: `Achieved` iff the deadline-anchored current met the
target; anything else — including insufficient data at the deadline — is
`Missed` (not demonstrably achieved is missed). Terminal reports carry
`required_per_week: None` and no projection.

#### 2.2.5 Consistency

No projection. Observed = mean sessions/week over the trailing
`min(full ISO weeks since set_on, DEFAULT_WINDOW_WEEKS)` **completed** weeks —
the in-progress week is excluded, and weeks absent from
`Adherence.weekly_days` count as **zero** (the aggregate omits empty weeks;
core fills them). A since-`set_on` lifetime mean is both uncomputable from the
8-week-windowed aggregate and dishonest — a strong first month would mask a
collapse; the requirement says *sustained*. Compared against
`sessions_per_week` with the same 25 % band. `Achieved` only at the deadline
(sustaining a rate cannot be finished early); the pre-deadline ceiling is
`Ahead`.

### 2.3 Data floors (AC5)

Owned by core as `pub const`s beside R-0039's magnitude caps. Below a floor →
`InsufficientData`, never a projection:

| Metric | Floor |
|---|---|
| Strength | current or baseline present; `sessions ≥ 3`; slope from ≥ 2 distinct-date `e1rm` points |
| Body — weight | ≥ 3 `weight` points spanning ≥ 14 days |
| Body — body-fat | ≥ 3 `body_fat_pct` points spanning ≥ 14 days (per-metric: bf is sparser than weight) |
| Consistency | ≥ 1 completed ISO week since `set_on` |

Slope presence is decided by core from the trend-point vectors — R-0015's bare
`slope == 0.0` is ambiguous between "plateau" and "no data", so core never
trusts the scalar alone. Counts and spans come from `BodyTrend`'s point vecs;
`aggregate.rs` is untouched.

### 2.4 Goal validation (typed, never a panic)

```rust
pub enum GoalError {
    TargetDateNotAfterCreation,
    TargetDateTooFar,        // > MAX_GOAL_HORIZON_DAYS (730)
    NonFiniteTarget,
    NonPositiveTarget,
    TargetTooLarge,          // weight > 500 kg or e1RM > 1500 kg — garbage in, garbage pace forever
    BodyFatOutOfRange,       // outside (0, 75)
    BodyTargetEmpty,         // Body with neither field set (AC2)
    BlankLift,
    LiftTooLong,             // > 80 chars
    ZeroSessions,
    TooManySessions,         // > 14/week
}
```

`pub fn validate(kind: &GoalKind, set_on: NaiveDate, target_date: NaiveDate)
-> Result<(), GoalError>`, exhaustive-matched to fixed `reason` tokens exactly
as R-0041 (no `_` arm). **"Unknown lift" rejection (old AC10 wording) is
unenforceable and dropped, with the requirement amended:** lifts are free-text
exercise names with no registry; blank/over-length is the enforceable proxy,
and an untrained-lift goal is a legitimate flow handled by §2.2.3.

### 2.5 Storage (api)

Whole-value, per the R-0041 precedent — nothing queries the dates today, so
shredding the domain type across columns is premature:

```sql
CREATE TABLE goals (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    goal        JSONB NOT NULL,   -- the whole serialized core::goals::Goal
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_goals_user_created ON goals (user_id, created_at DESC, id DESC);
```

No status column — derived state stored is derived state stale (AC8).

### 2.6 Endpoints

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/goals` | validate → capture baseline → store, `201` + initial report |
| `GET` | `/goals` | caller's goals, newest first, each with its report |
| `GET` | `/goals/:id` | one goal + report |
| `DELETE` | `/goals/:id` | abandon a goal, `204` (R-0042 AC11, added by amendment) |

All owner-scoped through one `load_owned` (404, never 403 — AC3);
`AuthenticatedUser` precedes `Path`/`Json` everywhere; `:id` route syntax.

**Baseline capture on create:** `find_sessions_by_user` → `summarize` → the
relevant current values become the stored `Baseline` (possibly `None` fields —
creation succeeds; §2.2.3 covers the gap).

**Signal assembly on read:** the same `summarize` output `/training-summary`
uses, so the two can never disagree. `GET /goals` fetches rows once and runs
`summarize` **once per distinct anchor date** (§2.2.4) — in practice once for
all active goals plus once per distinct expired deadline.

## 3. Code outline

```rust
pub(crate) async fn create(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateGoalRequest>,          // { kind, target_date }
) -> ApiResult<(StatusCode, Json<GoalResponse>)> {
    let today = Utc::now().date_naive();
    validate(&body.kind, today, body.target_date).map_err(|e| to_unprocessable(&e))?;
    let inputs = fetch_inputs(&state, &user).await?;          // sessions + measurements
    let summary = inputs.summarize(today);
    let baseline = Baseline::capture(&body.kind, &summary);   // pure, core
    let goal = Goal { kind: body.kind, baseline, set_on: today, target_date: body.target_date };
    let row = insert_goal(&state.pool, user.user_id, &goal).await?;
    let report = assess(&goal, &GoalSignal::from(&summary), today);
    Ok((StatusCode::CREATED, Json(GoalResponse::from_parts(row, goal, report))))
}
```

`GoalResponse { id, goal: Goal, report: PaceReport, created_at }` — the domain
value round-trips whole (R-0041 AC9 pattern; no duplicated fields).

## 4. Non-goals

Per R-0042 §4: no auto-adjustment, notifications, trainer-assigned goals,
gamification, undated goals, or update/edit (delete + recreate is v1's
mutation story). No new statistics — R-0015's `summarize` is the only trend
source.

## 5. Open questions — resolved

- **OQ-1:** linear required pace, recorded as a simplification.
- **OQ-2:** 25 % band on the remaining required change; documented endgame
  slide into `AtRisk` (§2.2.2), no epsilon floor.
- **OQ-3:** per-metric floors (§2.3), body-fat stricter in practice because
  its points are sparser.
- **OQ-4:** multiple active goals allowed; conflicts surfaced, not solved.

## 6. Acceptance criteria

AC1 server-side baseline (§2.6); AC2 three kinds — Body's two-metric case
reported per-metric with a pinned severity fold (§2.2.1); AC3
ownership-as-absence (§2.6); AC4 typed status + per-metric numbers; AC5 floors
+ `InsufficientData` reasons (§2.3), late-binding prevents the permanent trap
(§2.2.3); AC6 deadline-anchored terminality (§2.2.4); AC7 explicit-comparison
direction (§2.2.2); AC8 no writes on read, no stored status (§2.5); AC9 pure
`assess` with explicit `today`; AC10 (as amended) blank/over-length lift proxy;
AC11 (amendment) delete; tests below.

## 7. Test plan

Core unit tests (no database) — table-driven `(goal, signal, today) →
expected` covering, per kind: ahead / on-track / behind / at-risk /
achieved-early (and its reversion on regression) / missed / each
`InsufficientReason`; the fat-loss `dir = −1` case where rising weight is
`Behind`; band boundaries at `gap = ±band`; **achieved-at-creation**;
**Body with both metrics set and differing statuses** (severity fold);
**late-bound baseline** transitioning out of `InsufficientData` after three
sessions; **endgame**: same shortfall at 3 weeks out (`Behind`) vs 5 days out
(`AtRisk`); **post-deadline stability**: one expired goal assessed at two later
dates → identical terminal status; consistency: partial-week exclusion,
absent-week-as-zero, a goal older than 8 weeks, and no-early-`Achieved`;
every `GoalError` variant.

Integration tests (`#[sqlx::test]`): auth on every route incl. malformed id →
401; create → 201 with report; table-driven `GoalError` token sweep; baseline
captured from logged data not the request; ownership 404; cross-user signal
isolation; delete → 204 then 404; list newest-first (membership +
non-increasing timestamps); round-trip equality on the deserialized `Goal`.

## 8. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-08 | `assess` borrows the R-0015 aggregates (`GoalSignal`) | Core owns every derivation; no api-side adapter, no runtime `KindMismatch` for a state the type system can preclude. Supersedes the tagged-enum design. |
| 2026-08-08 | Per-metric reports with a pinned severity fold | A two-target Body goal has two answers; one scalar row was undefined. |
| 2026-08-08 | Late-bound baseline at read time | Stored `None` must not be a permanent `InsufficientData` trap; no write, AC8 intact. |
| 2026-08-08 | Post-deadline signal anchored at `target_date` | Terminality as a property of inputs, not a stored flag; pre-deadline `Achieved` is explicitly revertible. |
| 2026-08-08 | Direction by comparison, never `signum` | `signum(0.0) == 1.0` would decide an already-met goal by IEEE accident. |
| 2026-08-08 | Consistency = trailing-window mean, absent weeks zero | Since-creation mean is uncomputable from the windowed aggregate and masks collapse. |
| 2026-08-08 | Whole-value `goal` JSONB | R-0041 precedent; date columns have no querying customer yet. |
| 2026-08-08 | Unknown-lift rejection dropped, requirement amended | No lift registry exists; blank/length is the enforceable proxy. |
| 2026-08-08 | `DELETE` kept, backed by R-0042 AC11 amendment | Load-bearing for delete-and-recreate; an endpoint must trace to an AC. |
| 2026-08-08 | Band = 25 % of remaining change; no floor | Endgame slide into `AtRisk` is honest and monotone — verified no flapping. |

## 9. Architect review — ACCEPT-WITH-CHANGES (2026-08-08), all applied same day

13 findings; every one folded in above. The substantive redesigns: per-metric
`PaceReport` (finding 1 → §2.2.1), late-bound baseline (2 → §2.2.3),
deadline-anchored terminality (3 → §2.2.4), `GoalSignal` over aggregates
replacing the parallel enum (4 → §2.2), explicit-comparison direction and the
zero-span guard (5 → §2.2.2), trailing-window consistency (6 → §2.2.5),
unknown-lift drop + requirement amendment (7 → §2.4), whole-value storage
(8 → §2.5), DELETE backed by AC11 (9 → §2.6), endgame documented with no band
floor (10 → §2.2.2), target magnitude caps (11 → §2.4), pinned week arithmetic
+ clippy allowance (12 → §2.2.2), and the expanded test table (13 → §7).

## Changelog

- _2026-08-08 — created (Draft)._
- _2026-08-08 — architect review applied in full (§9); R-0042 amended in step
  (AC10 wording, AC11 delete)._
