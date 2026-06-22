# Agent Task — Write Requirement R-0030 (Visual Body-Type Picker)

**Project root:** `/Users/goose/projects/fitAI`
**Branch:** create `R-0030-body-type-picker` from `main`, push to it
**Output file:** `requirements/0030-body-type-picker.md`

---

## What you are doing

You are writing a new accepted requirement file for the fitAI project. The owner
wants an alternative to photo upload for archetype matching — a screen that
shows reference body silhouettes so users can self-select their type without
needing to take or upload a photo.

---

## Step 1 — Read these files before writing anything

```
requirements/0013-archetype-matching.md              ← the flow this complements
requirements/0014-program-diet-from-archetype.md     ← requirement format to match
backend/crates/core/src/matching/mod.rs              ← rank() function + FrameFeatures
backend/crates/core/src/archetype/mod.rs             ← FrameFeatures, StructureTag
CLAUDE.md                                            ← engineering constitution
```

---

## Step 2 — Context

**Current flow (R-0013):** user uploads photo → server runs pose estimation →
derives `FrameFeatures` (shoulder-to-waist ratio + banded clavicle/limb
descriptors + StructureTag) → `rank()` matches against archetype library → top-3
proposals → user picks program (R-0014).

**Problem:** photo upload is high-friction. Some users won't want to upload a
photo during onboarding. Others want to re-match without a new photo.

**Solution:** A screen showing ~12 reference silhouette images spanning the
main morphology space. User picks the closest silhouette + optionally selects an
estimated body-fat % band (lean / moderate / bulky). The selection maps to a
synthetic `FrameFeatures` object. That synthetic features object enters the
same `rank()` → proposals → program flow — no photo stored.

**Important:** Read `FrameFeatures` carefully. The synthetic features must be
valid inputs to the existing `rank()` function. The mapping (silhouette choice →
FrameFeatures values) is a lookup table authored by the team, not computed by
ML.

---

## Step 3 — Write the requirement file

Follow the **exact format** of `requirements/0014-program-diet-from-archetype.md`.

**Metadata:**
```
Status: Accepted
Milestone: M3 (fast-track) / onboarding
Created: 2026-06-21
Depends on: R-0013 (Done — matching flow this feeds into),
            R-0014 (Done — proposals + program choice flow downstream)
```

**Statement:** An alternative onboarding path where the user selects their
approximate body type from a grid of reference silhouettes (no photo upload).
The selection produces a synthetic `FrameFeatures` that feeds the existing
archetype-ranking and program-proposal flow. The photo-upload path (R-0013)
remains the primary, more accurate route.

**Acceptance criteria (write 10–12)** covering:
- AC1: "Don't want to upload a photo?" option visible on the photo-upload screen
- AC2: Picker screen shows a grid of ~12 silhouette images (static Flutter
  assets) covering ectomorph/mesomorph/endomorph × lean/moderate/bulky
- AC3: Each silhouette is labelled with a plain-language description
  (e.g. "Lean and narrow-shouldered", "Broad shoulders, moderate body fat"),
  no medical jargon, no famous names
- AC4: User picks one silhouette; optionally adjusts a body-fat % slider
  (three bands: <15 % / 15–25 % / >25 % for men; <22 % / 22–32 % / >32 % for
  women — or a simpler lean/moderate/bulky label if slider feels heavy)
- AC5: The selection maps to a synthetic `FrameFeatures` via a lookup table
  (authored by the team, stored in `core::archetype` or `core::matching`).
  The lookup table is unit-tested for every silhouette → FrameFeatures mapping.
- AC6: The synthetic FrameFeatures feeds into the existing `rank()` function
  without modification — no new ML, no new backend route required for the
  ranking step. (The question of whether a new lightweight endpoint is needed
  to accept synthetic features from the client — vs reusing the photo-session
  endpoint — is deferred to SPEC-0030.)
- AC7: The resulting top-3 proposals and the program-choice flow (R-0014) work
  identically to the photo-upload path from the user's perspective.
- AC8: No photo is stored when using the picker path.
- AC9: The picker is accessible at initial onboarding AND from the profile
  screen ("Update my archetype match").
- AC10: The silhouette images are vector SVGs bundled as Flutter assets
  (no CDN dependency, no network request for images).
- AC11: Widget tests cover: grid renders 12 items, tapping a silhouette selects
  it, confirm button is disabled until a selection is made, selection produces
  the expected FrameFeatures via the lookup table.
- AC12: Scope guard — no new ML inference, no photo storage, no changes to the
  `rank()` function or the archetype library data.

**Open questions to defer to SPEC-0030:**
- Does the Flutter client send synthetic FrameFeatures directly to a new
  lightweight backend endpoint (`POST /match/synthetic`), or does it create a
  photo-session record with a synthetic-features flag? (No photo stored either way.)
- Exact silhouette set — how many images, which morphology grid they cover,
  who authors the SVGs.
- Body-fat band labels — slider vs discrete chips.
- Whether the picker stores the selection so future re-matches default to it.

**Decision log:** Record: no photo stored, SVGs as Flutter assets, feeds
existing rank() unchanged, lookup table unit-tested — all with 2026-06-21 date.

Mark status **Accepted**.

---

## Step 4 — Commit and push

```bash
git checkout main
git checkout -b R-0030-body-type-picker
# write the file
git add requirements/0030-body-type-picker.md
git commit -m "R-0030: step-1 requirement — visual body-type picker (Accepted)"
git push -u origin R-0030-body-type-picker
```
