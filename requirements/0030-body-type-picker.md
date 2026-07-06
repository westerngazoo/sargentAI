# R-0030 — Visual Body-Type Picker

- **Status:** Accepted (amended to as-built 2026-07-06, R-0057)
- **Milestone:** M3 (fast-track) / onboarding
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-22
- **Depends on:** R-0013 (Done — matching flow this feeds into),
                  R-0014 (Done — proposals + program choice downstream)
- **Realized by:** [SPEC-0030](../specs/0030-body-type-picker.md)
- **QA:** `qa` agent run scoped to this requirement

---

> **Amendment note (R-0057, 2026-07-06).** This feature merged via PR #30 ahead
> of its spec. During the retro-spec/QA reconciliation the owner elected to
> **amend the acceptance criteria to the as-built scope** rather than build the
> originally-specified richer picker. The original AC set (12 SVG silhouettes,
> 36-entry lookup, `core` module, photo-screen link, profile entry point,
> gender-aware grids) is preserved in the decision log; the criteria below are
> what the shipped code is verified against. Items intentionally dropped are in
> §4 non-goals.

## 1. Statement

An alternative onboarding path where the user selects their approximate body
type from a small grid of reference shapes (no photo upload required). The
selection produces a synthetic `FrameFeatures` that feeds the existing `rank()`
function and the archetype-ranking → proposal → program-choice flow. The
photo-upload path (R-0013) remains the primary, more accurate route.

---

## 2. Rationale

Photo upload is high-friction. A shape picker removes the barrier while
preserving the same downstream program-generation flow (R-0014). Privacy is
improved: no photo is stored or processed.

---

## 3. Acceptance criteria (as-built)

- **AC1.** A program-start entry point ("Get my program") routes the user into
  the body-shape picker without requiring a photo.
- **AC2.** The picker shows a grid of **3 body-shape cards** spanning the coarse
  morphology space (lean/narrow, athletic/moderate, broad/heavy), rendered as
  **bundled Material icons** — no network request for imagery.
- **AC3.** Each shape is labelled with a plain-language description (e.g. "Lean,
  narrow build"). No medical jargon, no famous athlete names.
- **AC4.** After selecting a shape, a body-fat band selector (3 chips:
  Lean / Moderate / Bulky) is shown. Shape + band together give the full
  synthetic `FrameFeatures`.
- **AC5.** Each shape × band combination (3 × 3 = **9 entries**) maps to a
  synthetic `FrameFeatures` via the `synthetic_features` lookup in the
  `api::synthetic` module, unit-tested for **every one of the 9 combinations**
  (each produces a valid `FrameFeatures` accepted by `rank()` with a non-empty
  top-3).
- **AC6.** The synthetic `FrameFeatures` is passed to the existing `rank()`
  function **without modification**; the resulting ranked matches feed the
  existing proposals and program-choice flow (R-0014) identically to the
  photo-upload path.
- **AC7.** No photo is stored or processed on this path. A program created from
  the picker sets `source_session_id = NULL` — no `photo_session` row is
  referenced.
- **AC8.** The path is served by two endpoints: `POST /match/synthetic`
  (returns ranked proposals) and `POST /programs/synthetic` (commits the chosen
  program). The selection is **stateless** (not persisted as a re-match default).
- **AC9.** The Confirm button is disabled until **both** a shape and a body-fat
  band are selected.
- **AC10.** Tests cover the feature: a backend unit test for all 9 lookup
  entries; backend integration tests for both endpoints (happy path, auth
  required, no-profile handling, `source_session_id` NULL); Flutter widget tests
  (grid renders the 3 shapes; tapping highlights; band chips appear after a
  shape is chosen; Confirm gated on both selections; confirming calls the
  matching flow with the expected shape+band).
- **AC11.** Scope guard — no new ML inference, no photo storage, no changes to
  `rank()`, no changes to the archetype library data, no new archetype entries.

---

## 4. Constraints & non-goals

- `rank()` accepts synthetic `FrameFeatures` without any code change.
- Shape imagery is bundled (Material icons), no CDN, no runtime network request.
- Famous athlete names must not appear in labels or UI copy.
- No biometric data stored from the picker path.
- The picker does not replace photo upload — it is a fallback.
- **Explicitly not built (deferred):** the 12-SVG silhouette grid, the 36-entry
  lookup, a `core::body_picker` module, an in-photo-screen "don't want to upload"
  link, a profile-screen "Update my body match" entry point, and gender-aware
  (male/female) grids. Any of these can return as a follow-up requirement.

---

## 5. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-22 | (original) 12 SVG silhouettes, 36-entry lookup, `core` module, photo-screen + profile entry points, gender grids | Richer morphology coverage. |
| 2026-06-22 | Lookup table, not ML | Hand-authorable; no inference; unit-testable. |
| 2026-06-22 | Discrete 3-band chips over a slider | Lower cognitive load; maps to existing enums. |
| 2026-06-22 | `rank()` unchanged | Synthetic features must be valid `FrameFeatures`. |
| 2026-06-22 | No photo stored | Privacy: picker is the no-photo path. |
| 2026-07-06 | **Amend to as-built (R-0057):** 3 shapes (Material icons), 9-entry lookup in `api::synthetic`, `POST /match/synthetic` + `POST /programs/synthetic`, stateless, single home entry point, no gender grids | The reduced picker shipped, works, and is simpler; amending is more honest than building the 12-SVG version to satisfy a doc. Missing tests backfilled under R-0057. |

## Changelog

- _2026-06-22 — created and **Accepted**._
- _2026-07-06 — **amended to as-built** and tests backfilled under R-0057; SPEC-0030 written (retro-spec)._
