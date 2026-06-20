# SPEC-0014 — Program + diet generation from matched archetype

- **Status:** Accepted
- **Realizes:** R-0014
- **Author:** Claude (main session), with owner
- **Created:** 2026-06-20
- **Depends on:** SPEC-0013 (Implemented) — `POST /photo-sessions/:id/match`,
  `core::matching::rank`, `api::matching` DTO shapes;
  SPEC-0012 (Implemented) — `core::archetype::{Archetype, ProgramTemplate,
  DietTemplate, MacroEmphasis, VolumeBand}`, `library()`;
  SPEC-0002 (Implemented) — `AuthenticatedUser`, `ApiError`, `AppState`.
- **Module(s):**
  `backend/crates/core/src/program/` (new — pure instantiation logic),
  `backend/crates/api/src/program/` (new — endpoints + DTOs),
  `backend/crates/api/src/error.rs` (extend — add `Conflict` variant),
  `backend/migrations/00006_user_programs.sql` (new),
  `mobile/lib/screens/program_proposals_screen.dart` (new),
  `mobile/lib/screens/program_detail_screen.dart` (new),
  `mobile/lib/services/program_service.dart` (new),
  `mobile/lib/models/` — two new model files.

## 1. Motivation

Realizes [R-0014](../requirements/0014-program-diet-from-archetype.md): turn
the archetype ranking into a real choice the user owns. R-0013 produces a
ranked list and stops (scope guard R-0013/AC9); R-0014 expands the top-3 into
concrete proposals, lets the user pick, and persists the choice. The resulting
`user_programs` row is the structured record R-0027 reads to drive earbud-guided
sessions and M5 reads to learn per-user response.

## 2. Design

### 2.1 Shape

```
core::program       GeneratedProgram, GeneratedDiet, ProgramProposal
                    fn instantiate(archetype: &Archetype, profile: &Profile,
                                   today: NaiveDate) -> ProgramProposal

api::program        GET /photo-sessions/:id/program-proposals → ProposalsResponse
                    POST /programs                            → UserProgramResponse (201)
                    GET /programs/me/current                  → UserProgramResponse (200 | 404)
                    GET /programs/me                          → ProgramHistoryResponse (200)

api::error          extend ApiError with Conflict { reason: &'static str } → 409

DB                  user_programs (migration 00006)

Flutter             ProgramProposalsScreen  (proposals list + pick)
                    ProgramDetailScreen     (active program view)
```

### 2.2 Pure instantiation (`core::program`)

**`GeneratedProgram`** (serializable):
```rust
pub struct GeneratedProgram {
    pub split: String,                       // from ProgramTemplate::split
    pub days_per_week: u8,                   // derived from split keyword (§2.2.1)
    pub weekly_frequency_per_muscle: u8,
    pub volume: VolumeBand,
    pub intensity_guidance: String,          // from ProgramTemplate::intensity
    pub rest_guidance: String,               // from ProgramTemplate::rest
    pub progression_guidance: String,
    pub estimated_session_duration_min: u16, // derived from VolumeBand (§2.2.1)
    pub highlight_exercises: Vec<String>,    // derived from split (§2.2.1)
}
```

**`GeneratedDiet`** (serializable):
```rust
pub struct GeneratedDiet {
    pub approach: String,           // from DietTemplate::approach
    pub calorie_strategy: String,   // from DietTemplate::calorie_strategy
    pub macro_emphasis: MacroEmphasis,
    pub meal_structure: String,
    pub estimated_kcal: u32,        // derived (§2.2.2)
    pub protein_g: u32,
    pub carbs_g: u32,
    pub fat_g: u32,
}
```

**`ProgramProposal`** (serializable) — the per-card wire shape:
```rust
pub struct ProgramProposal {
    pub archetype_id: String,   // slug — never internal_name
    pub display_name: String,
    pub summary: String,
    pub score: f64,
    pub distance: f64,
    pub program: GeneratedProgram,
    pub diet: GeneratedDiet,
}
```

**`fn instantiate(archetype: &Archetype, profile: &Profile, today: NaiveDate) -> ProgramProposal`** —
pure; no I/O; `today` is a parameter so callers and tests control the date (no
`Utc::now()` inside).

#### 2.2.1 Program derivation

`split`, `weekly_frequency_per_muscle`, `volume`, `intensity_guidance`,
`rest_guidance`, `progression_guidance` are copied from `ProgramTemplate`.

**`days_per_week`** — derived by keyword matching on `ProgramTemplate::split`
(case-insensitive substring check):

| Split contains | `days_per_week` |
|---|---|
| "ppl" or "push/pull" | 6 |
| "upper" and "lower" | 4 |
| "full body" or "whole body" | 3 |
| otherwise | 4 |

**`estimated_session_duration_min`** — from `VolumeBand` (three real variants in
`core::archetype`):

| `VolumeBand` | minutes |
|---|---|
| `Low` | 45 |
| `Moderate` | 60 |
| `High` | 75 |

**`highlight_exercises`** — static mapping by split category:

| Split category | Exercises |
|---|---|
| PPL | Bench Press, Overhead Press, Squat, Barbell Row, Deadlift, Pull-up |
| Upper/Lower | Barbell Squat, Bench Press, Barbell Row, Overhead Press, Romanian Deadlift, Pull-up |
| Full Body | Barbell Squat, Bench Press, Deadlift, Barbell Row |
| Default | Barbell Squat, Bench Press, Deadlift, Barbell Row |

These are mnemonic highlights for the Flutter card — not a full program.

#### 2.2.2 Diet derivation

**Calorie estimate** (Mifflin-St Jeor TDEE):

```
weight = profile.weight_kg.get()          // f64 kg
height = profile.height_cm.get()          // f64 cm
age    = profile.age_on(today) as f64     // i32 → f64

bmr = 10.0 * weight + 6.25 * height − 5.0 * age + sex_offset
  where sex_offset = match profile.sex {
      Some(Sex::Male)   =>  5.0,
      Some(Sex::Female) => −161.0,
      None              =>  0.0,    // conservative mid-point
  }

tdee = bmr * 1.55    // moderate activity (target audience is actively training)

primary_goal = profile.goals.as_slice().first().copied()
                   .unwrap_or(Goal::Maintain)

kcal_target = match primary_goal {
    Goal::LoseFat                       => tdee * 0.80,   // 20 % deficit
    Goal::BuildMuscle | Goal::GainStrength => tdee * 1.15, // 15 % surplus
    Goal::Recomp | Goal::Maintain       => tdee,
}
```

**Macro split** by `DietTemplate::macro_emphasis`:

| `MacroEmphasis` | Protein | Fat | Carbs |
|---|---|---|---|
| `HighProtein` | `weight × 2.2 g` | `kcal × 0.25 / 9` | remainder |
| `Balanced` | `weight × 1.8 g` | `kcal × 0.30 / 9` | remainder |
| `HighCarb` | `weight × 1.6 g` | `kcal × 0.20 / 9` | remainder |
| `LowCarb` | `weight × 2.0 g` | `kcal × 0.40 / 9` | remainder |

`carbs_g = ((kcal_target − protein_g as f64 * 4.0 − fat_g as f64 * 9.0) / 4.0)
              .max(0.0).round() as u32`  — never negative.

All gram values `round() as u32`. `estimated_kcal = protein_g * 4 + carbs_g * 4 + fat_g * 9`
(recomputed from the rounded grams so the displayed kcal is consistent with the
macros).

### 2.3 Database — `user_programs` table

**Migration `00006_user_programs.sql`:**
```sql
CREATE TABLE user_programs (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archetype_id        TEXT        NOT NULL,   -- archetype slug; in-process library, no FK
    source_session_id   UUID        REFERENCES photo_sessions(id) ON DELETE SET NULL,
    program             JSONB       NOT NULL,   -- GeneratedProgram serialized
    diet                JSONB       NOT NULL,   -- GeneratedDiet serialized
    active              BOOLEAN     NOT NULL DEFAULT TRUE,
    chosen_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_programs_user_active ON user_programs (user_id, active);
```

`source_session_id` is nullable (not all future code paths will have a
photo session — e.g. a future manual-pick flow) but is populated from the
`photo_session_id` in `POST /programs` to preserve traceability. `ON DELETE SET
NULL` so deleting a photo session does not cascade-delete the user's program.

### 2.4 Backend API — `api::program`

#### 2.4.1 Shared helper — `derive_proposals`

Both `GET /program-proposals` and `POST /programs` need the top-3 matching
result. Extract:

```rust
/// Derives the top-3 archetype proposals for a session.
/// Returns `ApiError::NotFound` for a missing/foreign session,
/// `ApiError::Unprocessable` for no usable photo/pose.
async fn derive_proposals(
    state: &AppState,
    user_id: Uuid,
    session_id: Uuid,
    profile: &Profile,
    today: NaiveDate,
) -> ApiResult<Vec<ProgramProposal>>
```

Steps (identical for both handlers):
1. `db::session_exists_for_user` → `404` if missing/foreign.
2. `db::match_candidates_for_session` → `422 no_usable_photo` if empty.
3. `estimate_first_usable` (reuse from `api::matching`) → `422 no_person_detected`.
4. `derive_frame_features` → `422 degenerate_frame` via existing `FrameError → ApiError`.
5. `rank(features, library())` → take first 3 entries.
6. Call `core::program::instantiate(archetype, profile, today)` for each.
7. Return `Vec<ProgramProposal>` (3 items).

This ensures the ONNX estimator is called **at most once per request** regardless
of which endpoint is invoked, and the two handlers share a single derivation path.

#### 2.4.2 `GET /photo-sessions/:session_id/program-proposals`

1. Fetch profile — `db::get_profile(user_id)` → `404` if no profile.
2. Call `derive_proposals(state, user_id, session_id, &profile, today)`.
3. Return `200 ProposalsResponse { proposals: Vec<ProgramProposal> }`.

#### 2.4.3 `POST /programs`

Request body:
```json
{ "photo_session_id": "<uuid>", "archetype_id": "<slug>" }
```

1. Fetch profile → `404` if no profile.
2. `proposals = derive_proposals(state, user_id, photo_session_id, &profile, today)`.
3. Check `archetype_id ∈ proposals.iter().map(|p| &p.archetype_id)` →
   `409 Conflict` via new `ApiError::Conflict { reason: "archetype_not_in_proposals" }`
   if not found.
4. `program_proposal = proposals.into_iter().find(|p| p.archetype_id == archetype_id).unwrap()`.
5. In a single transaction:
   - `UPDATE user_programs SET active = FALSE WHERE user_id = $1 AND active = TRUE`
   - `INSERT INTO user_programs (user_id, archetype_id, source_session_id, program, diet, active) VALUES ...`
6. Return `201` + `UserProgramResponse`.

`UserProgramResponse`:
```rust
pub struct UserProgramResponse {
    pub id: Uuid,
    pub archetype_id: String,
    pub program: GeneratedProgram,
    pub diet: GeneratedDiet,
    pub active: bool,
    pub chosen_at: DateTime<Utc>,
}
```

#### 2.4.4 `GET /programs/me/current`

```sql
SELECT * FROM user_programs
WHERE user_id = $1 AND active = TRUE
ORDER BY chosen_at DESC
LIMIT 1
```

→ `200 UserProgramResponse` or `404`. The `ORDER BY chosen_at DESC` makes the
result deterministic if two concurrent transactions both commit with `active = TRUE`
(edge case; the transaction in §2.4.3 serialises the UPDATE+INSERT but not two
concurrent `POST /programs` calls).

#### 2.4.5 `GET /programs/me`

Query params: `limit` (default 20, max 100), `offset` (default 0).
```sql
SELECT * FROM user_programs WHERE user_id = $1 ORDER BY chosen_at DESC LIMIT $2 OFFSET $3
```
→ `200 ProgramHistoryResponse { programs: Vec<UserProgramResponse>, total: i64 }`.
`total` from `SELECT COUNT(*)` in the same handler.

#### 2.4.6 `ApiError::Conflict` — extension to `api::error`

Add to `ApiError` in `backend/crates/api/src/error.rs`:

```rust
/// A request conflicts with the derived state of an existing resource
/// (e.g. chosen archetype is not among the session's top-3 proposals).
Conflict { reason: &'static str },
```

`IntoResponse` arm:
```rust
ApiError::Conflict { reason } => (
    StatusCode::CONFLICT,
    Json(json!({ "error": reason })),
).into_response(),
```

This mirrors the existing `Unprocessable { reason }` pattern (same field name,
same response shape, different status code).

#### 2.4.7 Router wiring

```rust
// api::program::routes()
Router::new()
    .route("/photo-sessions/:id/program-proposals", get(get_proposals))
    .route("/programs",            post(choose_program))
    .route("/programs/me",         get(get_history))
    .route("/programs/me/current", get(get_current))
```

Merged into `app()` in `lib.rs` alongside `matching::routes()`.

### 2.5 Flutter

#### 2.5.1 New models (`mobile/lib/models/`)

- **`program_proposal.dart`** — `ProgramProposal`, `GeneratedProgram`,
  `GeneratedDiet`, `ProposalsResponse` with `fromJson`.
- **`user_program.dart`** — `UserProgram`, `ProgramHistoryResponse` with
  `fromJson`.

#### 2.5.2 `ProgramService` (`mobile/lib/services/program_service.dart`)

```dart
class ProgramService {
  Future<ProposalsResponse> getProposals(String sessionId);
  Future<UserProgram> chooseProgram(String sessionId, String archetypeId);
  Future<UserProgram> getCurrent();
  Future<ProgramHistoryResponse> getHistory({int limit = 20, int offset = 0});
}
```

Dio-backed; errors surfaced via the existing `ApiException.fromDio` pattern
(established in R-0008).

#### 2.5.3 `ProgramProposalsScreen`

- **Navigation**: pushed from the match-result flow — after
  `POST /photo-sessions/:id/match` succeeds, the router pushes
  `ProgramProposalsScreen(sessionId: id)`.
- **Layout**: a `ListView` of 3 `ProposalCard` widgets.
- **`ProposalCard`**: collapsed state shows archetype `display_name`, score
  label (`"Best match"` / `"Close match"` / `"Good option"` for ranks 1/2/3),
  `days_per_week` days/week, first 3 `highlight_exercises` as chips, and
  estimated kcal. Tapping expands inline to show the full `GeneratedProgram`
  (all fields) and `GeneratedDiet` (macro table). Expansion is exclusive — one
  card open at a time.
- **"Choose this program" button**: visible in expanded state only. On tap,
  calls `ProgramService.chooseProgram`, shows a loading indicator while
  in-flight (button disabled), navigates to `ProgramDetailScreen` on success.
- **Error handling**: `409` surfaces as a snackbar "Selection no longer available
  — please refresh." Other errors use the project's standard `ApiException` toast.

#### 2.5.4 `ProgramDetailScreen`

- **Navigation**: pushed from `ProgramProposalsScreen`; also reachable via
  named route `/programs/current`.
- **Data source**: `currentProgramProvider` — a `FutureProvider` backed by
  `ProgramService.getCurrent()`.
- **Layout**:
  - Header: archetype `display_name`, `chosen_at` formatted date.
  - Program section: training overview (days/week, frequency/muscle, volume,
    session duration) + guidance block (intensity, rest, progression).
  - Diet section: macro table (`protein_g / carbs_g / fat_g / kcal`) + approach
    + meal structure text.
- **Home shortcut**: a `CurrentProgramCard` widget on the home screen — shows
  active program's split and kcal, or a "Get your program" CTA if 404. Navigates
  to `ProgramDetailScreen` or to the match flow respectively.

## 3. Testing plan

### 3.1 Backend unit tests (`crates/core/src/program/tests.rs`)

- `instantiate_upper_lower_split_gives_4_days` — split "Upper/Lower" → `days_per_week == 4`.
- `instantiate_ppl_split_gives_6_days` — split "PPL" → `days_per_week == 6`.
- `instantiate_full_body_split_gives_3_days`.
- `instantiate_unknown_split_gives_4_days`.
- `instantiate_volume_low_gives_45_min` — `VolumeBand::Low` → 45.
- `instantiate_volume_moderate_gives_60_min`.
- `instantiate_volume_high_gives_75_min`.
- `instantiate_kcal_tracks_mifflin_st_jeor_male_moderate` — fixed profile
  (male, known weight/height/dob) + `Maintain` goal → expected kcal ± 5.
- `instantiate_kcal_deficit_for_lose_fat` — kcal ≈ tdee × 0.80 ± 5.
- `instantiate_kcal_surplus_for_build_muscle` — kcal ≈ tdee × 1.15 ± 5.
- `instantiate_high_protein_split_protein_correct` — protein ≈ weight × 2.2.
- `instantiate_carbs_never_negative` — pathological inputs → `carbs_g == 0`.
- `instantiate_kcal_consistent_with_macros` — `protein*4 + carbs*4 + fat*9 == estimated_kcal` (always true by construction; validates the recompute step).

### 3.2 Backend integration tests (`crates/api/tests/program.rs`)

All use `FakePoseEstimator` (via the existing `build_app_with_object_store`
helper, extended with pose injection as the R-0013 tests do).

- `proposals_returns_top3_for_own_session` — POST match session, GET proposals
  → 200, exactly 3 proposals, scores descending, no `internal_name` in body.
- `proposals_cross_user_session_is_404`.
- `proposals_unauthenticated_is_401`.
- `proposals_no_profile_is_404` — user without a profile → 404.
- `proposals_no_photo_is_422` — empty session → 422.
- `choose_creates_active_user_program` — POST /programs → 201, `active: true`.
- `choose_deactivates_previous_program` — second choose → first row `active = false`.
- `choose_archetype_not_in_top3_is_409`.
- `choose_cross_user_session_is_404`.
- `choose_no_profile_is_404`.
- `choose_unauthenticated_is_401`.
- `current_returns_active_program` — after a choose → 200.
- `current_no_program_is_404` — fresh user → 404.
- `history_returns_programs_newest_first` — two chooses → newest first.
- `history_pagination_limit_offset` — limit=1 → 1 item; offset=1 → skips first.

### 3.3 Flutter widget tests

- `proposals_screen_renders_three_cards`.
- `proposals_screen_card_expand_collapse` — tap card 1 expands; tap card 2
  collapses card 1 and expands card 2.
- `proposals_screen_choose_button_navigates_to_detail` — mock choose succeeds →
  `ProgramDetailScreen` pushed.
- `program_detail_screen_renders_program_and_diet` — split label, macro values
  visible.
- `home_current_program_card_navigates_to_detail`.
- `home_no_program_cta_navigates_to_match_flow`.

## 4. Resolved open questions

| OQ-H | Decision |
|------|----------|
| H1 — template parameterisation | Mifflin-St Jeor TDEE × goal multiplier; macro split by `MacroEmphasis` protein-first; split keyword → `days_per_week`; `VolumeBand` → session duration; static exercise list per split. See §2.2. |
| H2 — proposals caching | Re-derive inline on every call via shared `derive_proposals` helper (§2.4.1). No cache table. The ONNX estimator is called once per request regardless of endpoint. |
| H3 — wire shape | `GeneratedProgram` + `GeneratedDiet` in §2.2; `ProgramProposal` in proposals response; `UserProgramResponse` in program endpoints. |
| H4 — 409 vs 422 | **`409 Conflict`** for archetype-not-in-top-3 (new `ApiError::Conflict { reason }` variant, §2.4.6). `422` for no-photo/no-pose (existing R-0013 `Unprocessable` path). |
| H5 — Flutter navigation | Pushed from match-result handler; named route `/programs/current` for detail screen; home shortcut card. See §2.5.3/§2.5.4. |

## 5. Constraints

- **No ML** — all derivation is deterministic arithmetic.
- **`internal_name` / `sources` never cross the wire** — `ProgramProposal`
  carries only `archetype_id` (slug), `display_name`, `summary`.
- **No new `AppState` fields** — existing `pool`, `store`, `pose` are sufficient.
- **Prior-only guardrail** — `instantiate` reads `library()` as the prior; it
  writes nothing back and is not a training input.
- **Scope guard** (R-0014/AC11) — no ML inference, no program adjustment from
  logs, no earbud driving, no nutrition-log UI, no photo analytics.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-20 | **Shared `derive_proposals` helper, ONNX called once per request.** | Prevents the estimator running twice per choose interaction; single derivation path for both endpoints. |
| 2026-06-20 | **Mifflin-St Jeor TDEE × goal multiplier.** | Industry-standard; sufficient for a bootstrapping prior; M5 learns real per-user response from logs. |
| 2026-06-20 | **Static exercise roster per split.** | Mnemonic highlights for the Flutter card — a full exercise DB is a later concern (R-0017). |
| 2026-06-20 | **`source_session_id` nullable in `user_programs`.** | Preserves traceability for audit/re-matching; nullable for future non-photo code paths; `ON DELETE SET NULL` so photo deletion never removes programs. |
| 2026-06-20 | **`ApiError::Conflict { reason }` mirrors `Unprocessable`.** | Consistent error shape; `reason` field identifies the specific conflict without a new error type hierarchy. |
| 2026-06-20 | **`ORDER BY chosen_at DESC` on `GET /programs/me/current`.** | Deterministic result under concurrent transactions; the most recently chosen program wins. |

## Changelog

- _2026-06-20 — created. Five HOW-level questions resolved._
- _2026-06-20 — revised after architect review (REQUEST CHANGES → fixes applied):
  removed phantom `VolumeBand::VeryHigh`; corrected `Medium → Moderate`; fixed
  `goals.0 → goals.as_slice().first()`; fixed `Goal::GainMuscle → BuildMuscle`;
  fixed `.value() → .get()` accessors; added `ApiError::Conflict` extension
  description (§2.4.6); introduced `derive_proposals` shared helper (§2.4.1)
  to prevent double ONNX invocation; added `ORDER BY chosen_at DESC` to current
  endpoint; added `choose_no_profile_is_404` test; added `source_session_id`
  nullable column with `ON DELETE SET NULL`._
- _2026-06-20 — **Accepted** (post-revision)._
