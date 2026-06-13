# R-0013 — Photo → archetype matching

- **Status:** Accepted
- **Milestone:** M4 (Archetype prior) — the differentiator fast-track
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-13
- **Depends on:** R-0006 (Done — photo sessions: the stored bytes + the
  `ObjectStore` seam this reads), R-0012 (Done — the archetype `library()` this
  matches against)
- **Realized by:** SPEC-0013 (to be written in step 2)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

An authenticated user can turn a **progress-photo session into an archetype
match**: the backend runs **server-side pose estimation** over the session's
stored photo bytes, derives a **frame-feature profile** (the same shape
`core::archetype::FrameProfile` carries — a shoulder-to-waist ratio and the
banded/structural descriptors), and **ranks the curated archetypes by weighted
distance** to that profile, returning the ranking (each with a score) through a
read API. This is the "**upload your photo and the AI gives you your
archetype**" step of the differentiator: it consumes the R-0006 photo substrate
and the R-0012 library, and produces the ranked candidates R-0014 turns into a
proposed program + diet the user chooses from.

Pose estimation runs **in-process in Rust** via the ONNX Runtime (`ort` crate) —
no Python sidecar, single binary (owner decision, 2026-06-13). The inference is
isolated behind a **`PoseEstimator` seam** so the pure feature-derivation and
matching logic are testable without the model, and dev/CI need the model only
for the one path that exercises real inference.

## 2. Rationale

R-0012 built the library but nothing connects a *user* to it. R-0013 is the
bridge: it converts a photo into the same `FrameProfile` vocabulary the library
is authored in (deliberately so — SPEC-0012 §2.2), making the match a weighted
nearest-neighbor lookup over `library()`. Ranked output (not a single winner)
feeds R-0014's "present 2–3 targets and let the user choose," and the per-match
distances are reusable confidence signal. Keeping inference in-process in Rust
holds to the thin-client, server-side-intelligence principle without standing up
a second runtime.

## 3. Acceptance criteria

- **AC1. Frame-feature derivation (pure).** A pure module turns pose keypoints
  (normalized landmark coordinates + per-point visibility) into a
  **`FrameFeatures`** value carrying at minimum the numeric **shoulder-to-waist
  ratio** and the geometrically-derivable banded descriptors
  (`clavicle_width`, `limb_length`); fields a single 2-D photo cannot reliably
  determine (e.g. `build`/somatotype, fine `structure_tags`) are **explicitly
  optional/absent**, never fabricated. Derivation is validated (out-of-range or
  degenerate geometry → a typed error) and unit-tested from fixed keypoints with
  no model in the loop.
- **AC2. Weighted nearest-neighbor matching (pure).** A pure matcher computes a
  **distance** between a `FrameFeatures` and each `Archetype::frame_profile` in
  `core::archetype::library()`, over a **documented, weighted** field set (the
  numeric ratio dominates; present categorical fields contribute; absent fields
  are skipped, not penalized), and returns the archetypes **ranked nearest-first
  with their scores**. Deterministic and total; unit-tested (a frame near a
  known archetype ranks it first; ties broken stably).
- **AC3. `PoseEstimator` seam.** Inference sits behind a trait (`image bytes →
  keypoints | PoseError`) with **(a)** a real ONNX implementation (`ort`) and
  **(b)** a deterministic fake used by the matching/endpoint tests. The seam is
  dependency-inverted into the handler the way the R-0006 `ObjectStore` is, so
  the suite runs without invoking the model except where AC8 requires it.
- **AC4. Real ONNX inference works.** The real implementation loads a
  **bundled pose model** (an Apache-2.0/permissively-licensed model — candidate
  **MoveNet**; final choice in SPEC-0013) and produces keypoints from a JPEG/PNG.
  At least one test runs the **actual model on a fixture image** end-to-end
  (bytes → keypoints → features → ranking). No raw image or keypoint set is ever
  persisted or returned beyond the derived features (the photo bytes stay in the
  object store; SPEC-0012's prior-only and SPEC-0006's privacy rules hold).
- **AC5. Match API.** **`POST /photo-sessions/:id/match`** (authenticated) runs
  the pipeline over the session's photos and returns `200` + the **ranked
  archetype matches** (each: the archetype's user-facing `ArchetypeResponse`
  shape **plus** a `distance`/`score`), nearest first. Reuses R-0012's wire
  contract — **`internal_name`/`sources` never cross the wire** (AC4 of R-0012).
- **AC6. Honest failure modes.** A session with **no usable photo** (none stored,
  or none from which a person/pose can be extracted with sufficient confidence)
  → a typed `400`/`422` with a clear reason (no fabricated match). A **missing or
  foreign session** → `404` (cross-user is `404`, never `403` — the R-0006 rule).
  `401` unauthenticated.
- **AC7. Privacy & isolation.** Matching is scoped to the token's `sub`; a user
  can only match **their own** sessions; the photo bytes are read through the
  R-0006 storage seam and never leave the server. No new long-term storage of
  biometrics beyond what R-0006 already holds (persisting the chosen archetype is
  **R-0014's** concern, not this one).
- **AC8. Tests.** Unit tests for feature derivation (AC1) and matching (AC2) from
  fixed keypoints; an integration test for the real-ONNX path on a fixture image
  (AC4); integration tests for the endpoint (ranked `200`, no-pose `4xx`,
  cross-user `404`, `401`) using the **fake** estimator for determinism. All
  gates green (`cargo fmt`/`clippy`/`test`/`build`), and CI is wired with
  whatever the `ort`/ONNX-Runtime build needs.
- **AC9. Scope guard.** R-0013 is **matching only**: photo → features → ranked
  archetypes. **No** program/diet generation (R-0014), **no** persistence of the
  user's chosen archetype (R-0014), **no** M5 training, **no** mobile UI, **no**
  progress-over-time photo analytics (R-0018/R-0019). The famous data remains the
  prior; this requirement reads `library()`, it does not write it.

## 4. Constraints & non-goals

- **No program/diet generation, no target selection, no persistence of the
  match** — all R-0014.
- **No Python / no sidecar service** — in-process ONNX via `ort` (owner
  decision). A sidecar is explicitly rejected for v1.
- **No on-device inference** — the mobile client stays thin; matching is
  server-side.
- **No ML training** — this is inference + a deterministic distance, not a
  learned model; the famous archetypes are never a training set (R-0012 AC5).
- **No deep photo analytics** — symmetry/muscle-belly/progress-over-time features
  are M6 (R-0018/R-0019); R-0013 derives only the frame features matching needs.
- **No image transforms persisted** — any resize/normalization is in-memory for
  inference only; stored bytes are untouched (SPEC-0006).
- **No mobile UI** — the "your archetype" screen is a later M3/M4 requirement on
  top of this API.

## 5. Open questions

Settled in this step-1 discussion (owner, 2026-06-13):

- **OQ1 — Pose engine?** RESOLVED → **in-process Rust ONNX (`ort`)**, no sidecar.
  (Statement, AC3/AC4)
- **OQ2 — Match output?** RESOLVED → **ranked top-N with distances/scores** (not
  a single winner), feeding R-0014's choice. (AC2/AC5)

Deferred to the SPEC-0013 design discussion (HOW):

- **OQ-H1 — Which model?** Candidate **MoveNet** (Apache-2.0, 17 keypoints —
  enough for shoulder/hip/limb geometry); confirm licence, variant
  (Lightning/Thunder), input size, and whether the `.onnx` is committed to the
  repo or fetched at build. (AC4)
- **OQ-H2 — The exact `FrameFeatures` ↔ `FrameProfile` mapping and the distance
  weights** (which fields, how banded thresholds are set, how absent fields are
  handled). (AC1/AC2)
- **OQ-H3 — Which photo(s) drive the match** when a session has several angles
  (e.g. front for width ratios, side for depth) and how multi-photo features are
  combined. (AC1/AC5)
- **OQ-H4 — The pose-confidence threshold** below which a photo is "no usable
  pose," and the response shape for partial confidence. (AC6)
- **OQ-H5 — `ort` linking strategy** (download vs. system vs. static ONNX
  Runtime) and the CI setup it implies. (AC8)
- **OQ-H6 — Endpoint ergonomics:** match a whole **session** (proposed) vs. a
  single photo; compute-on-demand (proposed) vs. cache the ranking. (AC5)

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-13 | **In-process Rust ONNX (`ort`); no Python sidecar.** | Keeps a single binary and the server-side-intelligence principle without a second runtime/container; owner decision. (OQ1) |
| 2026-06-13 | **Ranked top-N matches with distances, not a single winner.** | Feeds R-0014's "present 2–3 targets to choose from"; the scores are reusable confidence signal; one source of truth for the ranking. (OQ2) |
| 2026-06-13 | **`PoseEstimator` seam (real ONNX + fake), mirroring the R-0006 `ObjectStore`.** | Makes feature/matching logic testable without the model and keeps inference swappable; the suite stays fast and the model is exercised only where it must be. |
| 2026-06-13 | **Pure feature-derivation and pure matcher, separate from inference.** | Determinism + unit-testability; the numeric ratio is the load-bearing signal, categorical fields contribute when present, absent fields are skipped not fabricated (honest about what one 2-D photo can determine). |
| 2026-06-13 | **Matching reads `library()`, never writes it; chosen-archetype persistence is R-0014.** | Keeps R-0013 a coherent read-only slice and the prior-only guardrail intact. |

## Changelog

- _2026-06-13 — created (Draft). The bridge from a user's photo to the curated library: server-side in-process ONNX pose estimation → a `FrameFeatures` profile → weighted nearest-neighbor over `core::archetype::library()` → ranked archetype matches via `POST /photo-sessions/:id/match`. Two step-1 forks owner-resolved (in-process Rust ONNX; ranked top-N). Six HOW-level questions deferred to SPEC-0013 (model choice, feature/distance mapping, multi-photo handling, confidence threshold, `ort` linking/CI, endpoint ergonomics)._
- _2026-06-13 — **Accepted.** Owner accepted AC1–AC9 as drafted, including the honesty constraint (a single 2-D photo yields geometry, not somatotype/density — the matcher weights what is derivable and never fabricates the rest). Next: step 2 — SPEC-0013 + the architect design review (settling OQ-H1..H6)._
