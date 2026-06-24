# R-0030 — Visual Body-Type Picker

- **Status:** Accepted
- **Milestone:** M3 (fast-track) / onboarding
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-22
- **Depends on:** R-0013 (Done — matching flow this feeds into),
                  R-0014 (Done — proposals + program choice downstream)
- **Realized by:** SPEC-0030 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

## 1. Statement

An alternative onboarding path where the user selects their approximate body
type from a grid of reference silhouettes (no photo upload required). The
selection produces a synthetic `FrameFeatures` that feeds the existing
`rank()` function and the archetype-ranking → proposal → program-choice flow.
The photo-upload path (R-0013) remains the primary, more accurate route.

---

## 2. Rationale

Photo upload is high-friction. Some users won't want to upload a photo during
onboarding; others want to re-match without a new photo. A silhouette picker
removes the barrier while preserving the same downstream program-generation
flow (R-0014). Privacy is improved: no photo is stored or processed.

---

## 3. Acceptance criteria

- **AC1.** A "Don't want to upload a photo?" link/button is visible on the
  photo-upload screen and routes to the picker.
- **AC2.** The picker screen shows a grid of ~12 reference silhouette images
  covering the main morphology space: ectomorph / mesomorph / endomorph ×
  lean / moderate / bulky (9 cells) plus 3 supplementary edge cases (e.g.
  very tall-narrow, very wide-short, heavy-set). Images are static Flutter
  asset SVGs — no network request.
- **AC3.** Each silhouette is labelled with a plain-language description
  (e.g. "Lean, narrow shoulders", "Broad shoulders, moderate build"). No
  medical jargon, no famous athlete names.
- **AC4.** The user selects one silhouette. A body-fat band selector (3
  discrete chips: Lean / Moderate / Bulky) is shown after selection.
  Combining silhouette + band gives the full synthetic `FrameFeatures`.
- **AC5.** Each silhouette × band combination maps to a synthetic
  `FrameFeatures` (from `crate::pose`) via a lookup table authored by the
  team. The lookup table lives in `core::matching` or a new
  `core::body_picker` module and is unit-tested for every combination
  (12 silhouettes × 3 bands = 36 entries — all must produce valid
  `FrameFeatures` accepted by `rank()`).
- **AC6.** The synthetic `FrameFeatures` is passed to the existing
  `rank()` function without modification. The resulting top-3 ranked matches
  feed the existing proposals and program-choice flow (R-0014) identically
  to the photo-upload path.
- **AC7.** No photo is stored or processed when using the picker path.
  No `photo_session` row is created that references a stored object.
  (Whether a lightweight synthetic-features endpoint is used or the existing
  photo-session endpoint is adapted is deferred to SPEC-0030.)
- **AC8.** The picker is accessible at initial onboarding AND from the
  profile screen under "Update my body match".
- **AC9.** SVG assets are bundled in `mobile/assets/body_types/` — no CDN
  dependency, no runtime network request for images.
- **AC10.** The Confirm button is disabled until both a silhouette and a
  body-fat band are selected.
- **AC11.** Widget tests cover: grid renders all 12 silhouettes; tapping
  a silhouette highlights it; band chips appear after selection; Confirm is
  disabled before both selections; confirming with a given combination
  calls the matching flow with the expected synthetic `FrameFeatures`.
- **AC12.** Scope guard — no new ML inference, no photo storage, no changes
  to the `rank()` function, no changes to the archetype library data, no
  new archetype entries.

---

## 4. Constraints & non-goals

- `rank()` must accept synthetic `FrameFeatures` without any code change.
- SVG silhouettes are vector, bundled, no CDN.
- Famous athlete names must not appear in silhouette labels or UI copy.
- No biometric data stored from the picker path (no photo, no anthropometric
  measurement).
- The picker does not replace photo upload — it is a fallback.

---

## 5. Open questions (deferred to SPEC-0030)

- **OQ-H1:** Does the Flutter client send synthetic `FrameFeatures` to a new
  lightweight backend endpoint (`POST /match/synthetic`) or to the existing
  photo-session flow adapted with a `synthetic: true` flag? Either path must
  leave no stored photo object.
- **OQ-H2:** Who authors the 12 SVG silhouettes and what morphology grid do
  they cover exactly?
- **OQ-H3:** How are the 36 lookup-table entries derived? (Formulaic mapping
  from band combinations vs. hand-authored values reviewed by a sports-
  science reference.)
- **OQ-H4:** Does the picker save the user's selection so future re-matches
  default to it, or is each visit stateless?
- **OQ-H5:** Gender-aware silhouettes? (Two grids: male / female, selected
  from the user's profile `sex` field.) Defer or include in v1?

---

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-22 | SVG assets bundled in Flutter (`mobile/assets/body_types/`) | No CDN dependency; works offline; fast load. |
| 2026-06-22 | Lookup table, not ML | 36 entries is hand-authorable; no inference needed; unit-testable. |
| 2026-06-22 | Discrete 3-band chips (Lean/Moderate/Bulky) over a continuous slider | Lower cognitive load; maps cleanly to `Somatotype`/`WidthBand` enums already in `FrameProfile`. |
| 2026-06-22 | `rank()` unchanged | Synthetic features must be valid `FrameFeatures`; the matching function needs no knowledge of how features were derived. |
| 2026-06-22 | No photo stored | Privacy: the picker path is explicitly a no-photo path. |

## Changelog

- _2026-06-22 — created and **Accepted**._
