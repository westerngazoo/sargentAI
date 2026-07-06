# SPEC-0030 — Visual body-type picker (no-photo program path)

- **Status:** Accepted
- **Realizes:** R-0030
- **Author:** Claude (retro-spec, R-0057)
- **Created:** 2026-07-03
- **Depends on:** SPEC-0013 (Implemented) — `core::pose::FrameFeatures`,
  `core::matching::rank`; SPEC-0014 (Implemented) — `core::program::instantiate`,
  `ProgramProposal`, `api::program::handlers::UserProgramResponse`,
  `db::insert_program`, `ApiError::Conflict`; SPEC-0012 (Implemented) —
  `core::archetype::{library, WidthBand, LengthBand}`; SPEC-0002 (Implemented) —
  `AuthenticatedUser`, `ApiError`, `AppState`, `db::find_profile_by_user`.
- **Module(s):**
  `backend/crates/api/src/synthetic/mod.rs` (new — wire types, lookup table,
  two handlers),
  `backend/crates/api/src/synthetic/routes.rs` (new — router),
  `backend/crates/api/src/lib.rs` (merge `synthetic::routes::routes()`),
  `mobile/lib/src/program/models/synthetic_match.dart` (new — `BodyShape`,
  `FatBand`, `SyntheticMatchResponse`),
  `mobile/lib/src/program/presentation/body_type_picker_screen.dart` (new),
  `mobile/lib/src/program/presentation/synthetic_proposals_screen.dart` (new),
  `mobile/lib/src/program/services/program_service.dart` (extend — two methods),
  `mobile/lib/src/router/app_router.dart` (add `/programs/get` route),
  `mobile/lib/src/program/presentation/program_detail_screen.dart` (CTA routes
  to `/programs/get`).

> **Retro-spec notice.** This document is written *after* the feature shipped
> (PR #30, merged 2026-07-03). It describes what **actually** landed on `main`.
> Under R-0057 the **requirement R-0030 was amended to ratify this as-built
> scope** (3 shapes, 9-entry `api::synthetic` lookup, Material icons, two
> synthetic endpoints). The authoritative, current AC mapping is **§6 — all 11
> amended criteria are MET** (QA sign-off 2026-07-06). Where §2/§4/§5/§7 below
> speak of "divergence from R-0030" or reference original AC numbers, that is
> **historical** — it records how the shipped feature differs from the
> *original* (pre-amendment) requirement, retained for context, and does not
> reflect an open gap.

## 1. Motivation

Realizes [R-0030](../requirements/0030-body-type-picker.md): a no-photo
onboarding path where the user picks a coarse body shape and body-fat band from
a visual grid instead of uploading a photo. The selection is turned into a
synthetic `FrameFeatures`, fed to the existing `rank()` function unchanged, and
expanded into the same top-3 `ProgramProposal` cards the photo path (R-0014)
produces. Choosing a proposal persists a `user_programs` row with **no** photo
session referenced — the path stores no biometric data.

The intelligence and the ranking/instantiation logic are entirely reused; this
spec adds only the synthetic feature-derivation seam and the Flutter picker UI.

## 2. Design

### 2.1 Shape

```
api::synthetic       BodyShape { Ectomorph | Mesomorph | Endomorph }
                     FatBand   { Lean | Moderate | Bulky }
                     fn synthetic_features(shape, fat_band) -> FrameFeatures
                     POST /match/synthetic      → SyntheticMatchResponse (200)
                     POST /programs/synthetic   → UserProgramResponse   (201)

Flutter              BodyShape / FatBand enums (mirror the backend)
                     BodyTypePickerScreen       (shape cards → fat-band chips)
                     SyntheticProposalsScreen   (expandable proposal cards → choose)
                     ProgramService.syntheticMatch / chooseSyntheticProgram
```

The backend lives in **`api::synthetic`**, not `core`. This diverges from
R-0030/AC5, which called for a `core::body_picker` or `core::matching` module.
The shipped lookup table (`synthetic_features`) is a private `fn` in
`backend/crates/api/src/synthetic/mod.rs:83`. See §4 and §6/AC5.

### 2.2 The synthetic-features lookup table

`synthetic_features(shape, fat_band) -> FrameFeatures`
(`backend/crates/api/src/synthetic/mod.rs:83-110`) is a pure, total `match` over
the **9** `(BodyShape, FatBand)` pairs — three shapes × three fat bands. Each
arm yields a `(shoulder_to_waist: f64, clavicle_width: WidthBand, limb_length:
LengthBand)` triple; `confidence` is fixed at `1.0` (synthetic features are
exact by definition — no measurement uncertainty).

| shape × band | `shoulder_to_waist` | `clavicle_width` | `limb_length` |
|---|---|---|---|
| Ectomorph / Lean | 1.25 | Narrow | Long |
| Ectomorph / Moderate | 1.20 | Narrow | Long |
| Ectomorph / Bulky | 1.15 | Narrow | Average |
| Mesomorph / Lean | 1.65 | Wide | Long |
| Mesomorph / Moderate | 1.55 | Wide | Average |
| Mesomorph / Bulky | 1.50 | Average | Average |
| Endomorph / Lean | 1.40 | Average | Short |
| Endomorph / Moderate | 1.30 | Average | Short |
| Endomorph / Bulky | 1.15 | Narrow | Short |

The produced `FrameFeatures` is the exact struct
(`core::pose::FrameFeatures` — `shoulder_to_waist: f64`,
`clavicle_width: Option<WidthBand>`, `limb_length: Option<LengthBand>`,
`confidence: f64`) that `rank()` accepts, so `rank` is called with **no code
change** (R-0030/AC6). `shoulder_to_waist` values sit inside the library's
`RATIO_MIN..=RATIO_MAX` (1.0..=2.5) envelope, so every entry is valid.

**This is 9 entries, not the 36 (12 silhouettes × 3 bands) the requirement
specified** — the shipped picker has 3 shapes, not 12 silhouettes, and the band
is folded into the same lookup rather than layered on top. See §4 and §6/AC2,
AC5.

### 2.3 Backend handlers (`api::synthetic`)

Both handlers require an authenticated user and a profile
(`db::find_profile_by_user` → `ApiError::NotFound` if absent), derive
`FrameFeatures` from the request's `shape`+`fat_band`, then reuse the R-0014
ranking/instantiation path verbatim:

```rust
let features = synthetic_features(body.shape, body.fat_band);
let today = Utc::now().date_naive();
let proposals: Vec<ProgramProposal> = rank(&features, library())
    .into_iter()
    .take(3)
    .map(|m| {
        let score = 1.0 - m.distance;
        instantiate(m.archetype, &profile, score, m.distance, today)
    })
    .collect();
```

#### 2.3.1 `POST /match/synthetic`

Request `SyntheticRequest { shape: BodyShape, fat_band: FatBand }`. Returns
`200 SyntheticMatchResponse { shape, fat_band, proposals: Vec<ProgramProposal> }`
— the top-3 proposals, echoing the selection back. No photo is read; **no
`photo_session` row is created** (R-0030/AC7).

#### 2.3.2 `POST /programs/synthetic`

Request `SyntheticChooseRequest { archetype_id: String, shape, fat_band }`.
Re-derives the top-3 from the stored `shape`+`fat_band` (the client resends
them — there is no server-side stored selection), verifies the chosen
`archetype_id` is among the top-3 (`ApiError::Conflict { reason:
"archetype_not_in_proposals" }` → 409 otherwise), then persists via
`db::insert_program(pool, user_id, archetype_id, None /* no photo session */,
&program, &diet)` and returns `201 UserProgramResponse`. The `None`
`source_session_id` is what makes this a no-photo record (R-0030/AC7).

#### 2.3.3 Router wiring

`synthetic::routes::routes()` registers `POST /match/synthetic` and
`POST /programs/synthetic`, merged into `app()` in `lib.rs:70`. `CorsLayer`
handling (permissive, for local Flutter-web dev) was added in the same PR but is
incidental to this feature.

### 2.4 Flutter

#### 2.4.1 Models — `synthetic_match.dart`

`BodyShape { ectomorph, mesomorph, endomorph }` and
`FatBand { lean, moderate, bulky }` enums mirror the backend; each carries a
plain-language `label`, `description`/`sublabel`, and a `value` getter (the
`name`, serialized as the snake_case wire value). `SyntheticMatchResponse`
`fromJson` parses `shape`, `fat_band`, and the reused `ProgramProposal` list.
Labels are plain-language and carry **no athlete names** (R-0030/AC3): e.g.
"Lean & Narrow — Slender frame, narrow shoulders…", "Athletic & Broad", "Stocky
& Solid".

#### 2.4.2 `BodyTypePickerScreen`

A `ConsumerStatefulWidget` holding `_shape`, `_fatBand`, `_loading`, `_error`.
`_ShapeGrid` renders three `_ShapeCard`s (one per `BodyShape`); selecting a
shape reveals the `_FatBandChips` (three `ChoiceChip`s). The "Find my program"
`FilledButton` is enabled only when `_shape != null && _fatBand != null &&
!_loading` (`_canConfirm`, R-0030/AC10). Confirm calls
`ProgramService.syntheticMatch` and pushes `SyntheticProposalsScreen` with the
returned proposals plus the chosen shape+band.

Silhouettes are rendered as **Material icons**
(`Icons.straighten` / `Icons.fitness_center` / `Icons.circle_outlined`), not SVG
assets. There is no `mobile/assets/body_types/` directory and no `pubspec.yaml`
asset registration. This diverges from R-0030/AC2 and AC9. See §4 and §6.

#### 2.4.3 `SyntheticProposalsScreen`

A `ListView` of `_ProposalCard`s. Each shows `displayName`, a rank chip ("Best
match" / "Close match" / "Good option"), `daysPerWeek`, `estimatedKcal`, and up
to 3 `highlightExercises`. Tapping expands one card at a time to reveal
intensity/rest/progression guidance, the macro row, and a "Choose this program"
button, which calls `ProgramService.chooseSyntheticProgram`, invalidates
`currentProgramProvider`, and navigates to `/programs/current`. A 409/other
`ApiException` surfaces as a snackbar.

#### 2.4.4 Entry point & routing

The picker is reached from the home screen's `CurrentProgramCard` "Get your
program" CTA (`program_detail_screen.dart:472` → `context.go('/programs/get')`);
`/programs/get` maps to `BodyTypePickerScreen` in `app_router.dart:66-69`. This
is a **single** entry point — there is no "Don't want to upload a photo?" link
on a photo-upload screen and no "Update my body match" profile entry. The
*original* R-0030 asked for both; the amended requirement specifies this single
entry point, so amended AC1 is MET (see §6). Additional entry points are a
deferred non-goal (§4).

## 3. Code outline

Representative shape of the shipped backend module
(`backend/crates/api/src/synthetic/mod.rs`):

```rust
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BodyShape { Ectomorph, Mesomorph, Endomorph }

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FatBand { Lean, Moderate, Bulky }

fn synthetic_features(shape: BodyShape, fat_band: FatBand) -> FrameFeatures {
    let (stw, clav, limb) = match (shape, fat_band) {
        (Ectomorph, Lean) => (1.25, Narrow, Long),
        // … 9 arms total …
        (Endomorph, Bulky) => (1.15, Narrow, Short),
    };
    FrameFeatures {
        shoulder_to_waist: stw,
        clavicle_width: Some(clav),
        limb_length: Some(limb),
        confidence: 1.0,
    }
}

pub(crate) async fn synthetic_match(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<SyntheticRequest>,
) -> ApiResult<Json<SyntheticMatchResponse>> { /* rank → take(3) → instantiate */ }

pub(crate) async fn choose_synthetic(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<SyntheticChooseRequest>,
) -> ApiResult<(StatusCode, Json<UserProgramResponse>)> { /* verify top-3 → insert_program(None) */ }
```

Flutter service methods (`program_service.dart`):

```dart
Future<SyntheticMatchResponse> syntheticMatch(BodyShape shape, FatBand fatBand);
Future<UserProgram> chooseSyntheticProgram(
    String archetypeId, BodyShape shape, FatBand fatBand);
```

## 4. Non-goals

Scope R-0030 explicitly guards (amended AC11), all honoured by the shipped code:

- **No new ML inference** — `synthetic_features` is a static `match`; `rank()`
  and `instantiate()` are reused unchanged.
- **No photo storage** — the choose handler passes `None` as
  `source_session_id`; no `photo_session` row is touched.
- **No changes to `rank()`, the archetype library, or archetype entries.**

Deliberate scope reductions relative to R-0030 (documented here so they are not
mistaken for oversights — each is a gap tracked in §6):

- **3 coarse shapes, not 12 silhouettes.** The morphology grid is
  ectomorph/mesomorph/endomorph only; the "×3 fat band" axis is folded into the
  9-entry lookup rather than producing 12×3 = 36 entries.
- **Material icons, not SVG assets.** No `mobile/assets/body_types/` SVGs; no
  `pubspec.yaml` asset registration. (Still satisfies "no network request" —
  icons are bundled — but not "bundled SVG silhouettes".)
- **Backend logic in `api::synthetic`, not `core::body_picker`/`core::matching`.**
- **Single entry point** (home "Get your program" CTA). No photo-screen
  fallback link, no profile "Update my body match" entry.

## 5. Open questions

Resolutions as actually decided in the shipped code (R-0030 §5):

| OQ-H | Requirement's question | Shipped resolution |
|------|------------------------|--------------------|
| **H1** | New `POST /match/synthetic` endpoint vs. adapted photo-session flow with `synthetic: true` | **New endpoints.** `POST /match/synthetic` (proposals) and `POST /programs/synthetic` (choose). No photo-session row is created; the choose handler persists with `source_session_id = None`. |
| **H2** | Who authors the 12 SVG silhouettes and what grid | **Not built as specified.** 3 coarse shapes rendered as Material icons; no SVG author, no 12-cell grid. Deferred. |
| **H3** | How the 36 lookup entries are derived (formulaic vs. hand-authored) | **9 hand-authored entries** (3 shapes × 3 bands), calibrated by phenotype so each combination's nearest archetype matches the selected build. Not 36; not formulaic. |
| **H4** | Does the picker persist the selection for future re-matches | **Stateless.** The selection is not stored server-side; the client resends `shape`+`fat_band` on choose so the backend can re-derive and verify the top-3. |
| **H5** | Gender-aware silhouettes (two grids by `sex`) | **Deferred / not built.** A single shape set is used regardless of profile `sex`. The `Utc::now()`-based `today` and the `profile` (which carries `sex`) still feed `instantiate`'s calorie math, but the *shape lookup* is sex-agnostic. |

## 6. Acceptance criteria

Maps to the **amended (as-built) requirement AC1–AC11**. All are MET by the
shipped code and covered by tests backfilled under R-0057.

- [x] **AC1 (MET).** "Get my program" entry routes into the picker without a
      photo (home CTA `program_detail_screen.dart:472` → route `/programs/get`
      in `app_router.dart:66` → `BodyTypePickerScreen`). *Test:* every widget
      test pumps the picker with no photo dependency.
- [x] **AC2 (MET).** The grid shows **3** body-shape cards rendered as bundled
      Material icons, no network request (`body_type_picker_screen.dart`
      `_ShapeGrid`/`_ShapeCard._icon`). *Test:* `grid renders all three
      body-shape cards` asserts the 3 labels + `Icons.straighten`/
      `fitness_center`/`circle_outlined`.
- [x] **AC3 (MET).** Plain-language labels/descriptions, no athlete names
      (`synthetic_match.dart:11-27`). *Test:* `match_synthetic_returns_top3_
      proposals` asserts no internal labels leak on the wire.
- [x] **AC4 (MET).** Band chips (Lean/Moderate/Bulky) appear after shape
      selection; shape+band form the synthetic `FrameFeatures`
      (`body_type_picker_screen.dart:79-88`). *Test:* `fat-band chips appear
      only after a shape is selected`.
- [x] **AC5 (MET).** All **9** shape×band combos map to a valid `FrameFeatures`
      accepted by `rank()` with a non-empty top-3, in the `api::synthetic`
      lookup (`synthetic/mod.rs`). *Tests:* unit
      `every_combination_yields_valid_frame_features` +
      `every_combination_ranks_a_non_empty_top3`.
- [x] **AC6 (MET).** Synthetic features passed to `rank()` unchanged
      (`synthetic/mod.rs:132`); top-3 feed the same proposal/choose flow.
      *Test:* `match_synthetic_returns_top3_proposals`; widget `confirming calls
      syntheticMatch…`.
- [x] **AC7 (MET).** No photo stored; program created with
      `source_session_id = NULL` (`insert_program(..., None, ...)`). *Test:*
      `choose_synthetic_commits_program_with_null_source_session` (reads the row,
      asserts NULL).
- [x] **AC8 (MET).** Two endpoints — `POST /match/synthetic`,
      `POST /programs/synthetic` — and the selection is stateless (choose
      re-derives from client-resent shape+band). *Tests:* both endpoints'
      happy-path + auth/no-profile cases in `tests/synthetic.rs`.
- [x] **AC9 (MET).** Confirm disabled until both shape and band selected
      (`body_type_picker_screen.dart:26` `_canConfirm`). *Test:* `Confirm is
      disabled until both a shape and a band are selected`.
- [x] **AC10 (MET).** Tests present: **2** backend unit + **8** integration
      (happy/401/404/409/422 incl. the NULL-source-session assertion) + **4**
      Flutter widget. All green.
- [x] **AC11 (MET).** Scope guard honoured: no new ML, no photo storage, no
      `rank()`/`library`/archetype change — `synthetic_features` is a static
      `match` and reuses `rank`/`instantiate`/`library` unchanged.

QA sign-off (R-0057, 2026-07-06): all 11 criteria MET; Flutter (4) + backend
unit (2) + integration (8) suites green; `flutter analyze`, `cargo clippy
-D warnings`, and `cargo fmt --check` clean.

## 7. Decision log

Retro-recorded — decisions inferred from the shipped code (PR #30), not
prospectively agreed.

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-03 | **New `POST /match/synthetic` + `POST /programs/synthetic` endpoints** (resolves OQ-H1). | Cleanest reuse of the R-0014 rank→instantiate→persist path; leaves no photo object. |
| 2026-07-03 | **9-entry hand-authored lookup in `api::synthetic`, not 36 in `core`.** | 3 coarse shapes were shipped instead of 12 silhouettes; the module was placed at the API seam beside the other match handlers. Divergence from R-0030/AC5. |
| 2026-07-03 | **`confidence = 1.0`** for all synthetic features. | Synthetic values are exact; no measurement uncertainty to model. |
| 2026-07-03 | **Stateless selection** (resolves OQ-H4). | Client resends shape+band on choose; backend re-derives and verifies the top-3, avoiding a stored-selection table. |
| 2026-07-03 | **Material icons instead of SVG silhouettes** (OQ-H2). | Faster to ship; still zero network requests. Divergence from R-0030/AC2/AC9. |
| 2026-07-03 | **Sex-agnostic shape lookup** (resolves OQ-H5). | Single shape set; gender-aware grids deferred. |

## Changelog

- _2026-07-03 — retro-spec created under R-0057 to document the feature that
  merged via PR #30 without a spec. Status **Accepted** (feature already on
  `main`). Records the divergences from R-0030 (9 vs. 36 entries, icons vs. SVG,
  `api::synthetic` vs. `core`, single entry point, missing tests) as explicit
  gaps for the qa agent to backfill._
