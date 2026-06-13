# SPEC-0013 — Photo → archetype matching

- **Status:** Accepted
- **Realizes:** R-0013
- **Author:** Claude (main session), with owner
- **Created:** 2026-06-13
- **Depends on:** SPEC-0006 (Implemented) — photo sessions, the `ObjectStore` seam, `AppState`; SPEC-0012 (Implemented) — `core::archetype` `library()`/`FrameProfile`/`ArchetypeResponse`; SPEC-0002 (Implemented) — `AuthenticatedUser`, `ApiError`, the router.
- **Module(s):** `backend/crates/core/pose` (new — pure keypoints + frame-feature derivation), `backend/crates/core/matching` (new — the pure weighted matcher), `backend/crates/api/pose` (new — the `PoseEstimator` seam + the ONNX impl + a test fake), `backend/crates/api/matching` (new — the match endpoint + DTO), `backend/crates/api/lib` (router merge + `AppState` field). **No migration, no DB write** (matching is read-only; persisting the chosen archetype is R-0014).

## 1. Motivation

Realizes [R-0013](../requirements/0013-archetype-matching.md): the bridge from a
user's photo to the curated prior. A photo's bytes (R-0006) become pose
keypoints (in-process ONNX), the keypoints become a `FrameFeatures` profile in
the **same vocabulary the library is authored in** (SPEC-0012 §2.2), and the
library is **ranked by weighted distance** to that profile. The output is the
ranked candidates R-0014 turns into a chosen program + diet. R-0013 is matching
only — no generation, no persistence, no training.

## 2. Design

### 2.1 Shape

```
core::pose          Keypoint{x,y,score}, PoseKeypoints (COCO-17, named access),
                    FrameFeatures{ shoulder_to_waist, clavicle_width?, limb_length?, confidence },
                    derive_frame_features(&PoseKeypoints) -> Result<FrameFeatures, FrameError>
core::matching      rank(&FrameFeatures, &[Archetype]) -> Vec<RankedMatch<'a>>   // nearest-first
api::pose           trait PoseEstimator { estimate(bytes, content_type) -> Result<PoseKeypoints, PoseError> }
                      OnnxPoseEstimator (ort + bundled model)   //  real
                      FakePoseEstimator (injected keypoints)     //  tests
api::matching       POST /photo-sessions/:id/match -> MatchResponse | 4xx
                      MatchResponse{ matches: [RankedArchetype] }   //  ArchetypeResponse + distance/score
```

The **pure** core (derivation + matching) carries the algorithm; the **impure**
api edge (inference + IO) is a thin seam. This is the same split as R-0006
(`core::photo` pure metadata vs `api::storage` IO) and keeps the matcher
unit-testable with no model.

### 2.2 Pure pose + features (`core::pose`)

- **`Keypoint { x: f32, y: f32, score: f32 }`** — a normalized image coordinate
  (`0.0..=1.0`) plus the model's per-point confidence.
- **`PoseKeypoints`** — the fixed **COCO-17** landmark set (nose, eyes, ears,
  shoulders, elbows, wrists, hips, knees, ankles) addressed through a
  `Landmark` enum, so derivation reads `kp.get(Landmark::LeftShoulder)` rather
  than a magic index. Constructed from a `[Keypoint; 17]`.
- **`FrameFeatures`** — the **query** profile (mirrors `FrameProfile`, but only
  what one 2-D photo yields):
  - `shoulder_to_waist: f64` — the V-taper proxy: shoulder-point span ÷
    hip-point span (the waist proxy), in the library's `1.0..=2.5` band;
  - `clavicle_width: Option<WidthBand>` — banded from shoulder span vs. a
    height/torso normalizer (absent if the relevant keypoints are low-confidence);
  - `limb_length: Option<LengthBand>` — banded from limb-segment lengths vs. torso;
  - `confidence: f64` — the aggregate keypoint confidence the derivation rested on.
  - **Deliberately no `build`/`structure_tags`** — somatotype and muscle density
    are **not** derivable from 2-D keypoints (R-0013 honesty constraint); these
    are simply absent from the query, not invented.
- **`derive_frame_features(&PoseKeypoints) -> Result<FrameFeatures, FrameError>`**
  — pure geometry. Rejects degenerate input (`FrameError`): too few keypoints
  above a confidence floor, a zero/near-zero hip span (division guard), or a
  ratio outside a sane envelope. The numeric ratio is always produced when the
  shoulder+hip points clear the floor; the banded fields are produced only when
  their keypoints clear it (else `None`). Reuses `WidthBand`/`LengthBand` from
  `core::archetype` (one vocabulary).

### 2.3 Pure matcher (`core::matching`)

- **`RankedMatch<'a> { archetype: &'a Archetype, distance: f64 }`** — a borrow of
  a library archetype plus its distance (zero-copy over the `'static` library).
- **`rank(features: &FrameFeatures, library: &[Archetype]) -> Vec<RankedMatch>`**
  — computes the distance to every archetype's `frame_profile`, returns them
  **sorted ascending (nearest first)**, ties broken by library order (stable
  sort). Total and deterministic.
- **Distance (documented, weighted):**
  - **numeric** — `|features.shoulder_to_waist − profile.shoulder_to_waist|`
    normalized by the `1.5`-wide band → `0.0..=1.0`, weight **0.6** (dominant —
    it is the most reliable single-photo signal);
  - **clavicle_width** — ordinal band distance (`0`/`0.5`/`1.0` for 0/1/2 bands
    apart), weight **0.2**, **only if `features.clavicle_width` is `Some`**;
  - **limb_length** — same ordinal scheme, weight **0.2**, only if `Some`;
  - **absent fields are skipped and the remaining weights renormalized** to sum
    to 1 — an absent field never penalizes a match (R-0013 AC2).
  - `score = 1.0 − distance` is surfaced alongside `distance` for the wire.
- **Total ordering & edges:** distances are sorted with **`f64::total_cmp`** (never
  `partial_cmp().unwrap()`) so no stray `NaN` can panic the sort — ranking is
  total. If **both** banded fields are absent on the query, the numeric term
  renormalizes to weight `1.0`; the ratio is always present and the hip-span
  division is guarded in `derive_frame_features` (§2.2), so renormalization never
  divides by zero.

### 2.4 The `PoseEstimator` seam (`api::pose`)

```rust
#[async_trait]
pub trait PoseEstimator: Send + Sync {
    async fn estimate(&self, bytes: &[u8], content_type: ImageContentType)
        -> Result<PoseKeypoints, PoseError>;
}
```

- **`OnnxPoseEstimator`** — holds an `Arc<ort::Session>` loaded **once at
  startup** from a bundled model. `estimate()`: decode bytes (the `image` crate)
  → resize to the model's input → build the input tensor → run inference under
  **`tokio::task::spawn_blocking`** (ORT inference is CPU-bound and blocking) →
  parse the output into `PoseKeypoints`. The model file and its exact tensor
  I/O are fixed in §2.6 from the grounding pass (OQ-H1/H5).
- **`FakePoseEstimator`** — test-only (`#[cfg(test)]`/test module): returns
  caller-injected `PoseKeypoints`, so the endpoint and matching suites are
  deterministic and never load the model. Dependency-inverted exactly like the
  R-0006 `LocalObjectStore`.
- **`AppState`** gains `pose: Arc<dyn PoseEstimator>` (alongside `store`).
- **`PoseError`** (typed): `NoPersonDetected` (no keypoint set clears the
  confidence floor), `Decode` (the stored bytes won't decode), `Inference`
  (model/runtime fault). **Mapping (decided here, not deferred):**
  `NoPersonDetected` and `core::pose::FrameError` → **`422`** via the new
  `ApiError::Unprocessable` variant; `Decode` and `Inference` → **`500`** (opaque)
  — a photo that was content-type-validated at upload (R-0006) yet won't decode,
  or a model fault, is a **server-side** fault, not bad user input, so 500 (not
  422) is correct.
- **`ApiError::Unprocessable { reason: &'static str }`** is **added** to the api
  error enum → **`422` `{"error":"unprocessable","reason":<token>}`**. `reason` is
  a fixed `&'static str` token (`"no_usable_photo"` / `"no_person_detected"` /
  `"degenerate_frame"`), **never free text** (the no-stringly-typed-errors rule,
  CLAUDE.md §6). `From<PoseError>` / `From<FrameError>` map onto it, mirroring the
  existing `From<ObjectStoreError> for ApiError`.

### 2.5 The match endpoint (`api::matching`)

- **`POST /photo-sessions/:id/match`** (authenticated):
  1. **Ownership check** scoped to `user.user_id` — missing/foreign → **`404`**
     (cross-user is 404 never 403). Use the lightweight owner-scoped
     `session_exists_for_user` (`db.rs`), **not** the serializable `PhotoSession`
     aggregate (which omits `storage_key`, R-0006 AC9).
  2. **Load match candidates** via a **new owner-scoped query**
     `match_candidates_for_session(pool, user_id, session_id) -> Vec<MatchCandidate>`
     where `MatchCandidate { angle: Option<Angle>, content_type: ImageContentType,
     storage_key: String }` (joined on `photo_sessions.user_id`; `storage_key` is
     needed here and stays server-side). An **empty** session → **`422
     no_usable_photo`**. (Deliberately list-returning so multi-photo fusion —
     OQ-H3 — is later additive.)
  3. **Pick + estimate (deterministic, bounded):** order candidates **front-angle
     first, then stored order**; for each in turn, read its bytes (`ObjectStore`
     seam) and `estimate()`. On the **first usable pose, stop** and use it; a
     candidate that yields `NoPersonDetected` falls through to the next. If **no**
     candidate yields a usable pose → **`422 no_person_detected`**. Worst case is
     one inference per photo; the common case is one. (A "highest-confidence"
     selection that scores *all* photos is deferred with fusion — OQ-H3.)
  4. `derive_frame_features(&keypoints)` — `FrameError` → **`422 degenerate_frame`**
     (no fabricated match).
  5. `rank()` the library → **`200`** + `MatchResponse`.
- **`MatchResponse { matches: Vec<RankedArchetype> }`** where
  **`RankedArchetype`** flattens the R-0012 `ArchetypeResponse` (so
  `internal_name`/`sources` still never cross the wire) and adds `distance: f64`
  and `score: f64`. Nearest first.
- `401` unauthenticated (the `AuthenticatedUser` extractor).

### 2.6 The ONNX model + inference (locked — OQ-H1/H5)

**Model — MoveNet SinglePose (Apache-2.0).** A single-stage, single-person model
emitting **COCO-17** keypoints — exactly the joints matching needs (shoulders,
hips, elbows, wrists, knees, ankles) and no more. Apache-2.0 is granted on the
model artifact itself (cleaner than RTMPose/BlazePose, whose pretrained-weight
terms differ; **YOLOv8-pose is AGPL-3.0 → disqualified** for a closed-source
product). The `.onnx` is the **Xenova ONNX export** (`fp16`, ~4.8 MB) **embedded
in the binary via `include_bytes!`** — no runtime file IO, reproducible builds.

- **Variant:** **Thunder (256×256 input)** for body-ratio precision over latency
  (a static physique photo is not latency-sensitive); Lightning (192) is the
  smaller fallback. The exact node names are pinned against the chosen `.onnx`
  with Netron at impl time.
- **Input tensor (a deliberate gotcha to encode):** **NHWC `int32` `[1,256,256,3]`,
  RGB, values 0–255 — NOT NCHW, NOT float-normalized.** Preprocessing is an
  **aspect-preserving letterbox** (mirrors MoveNet's `resize_with_pad`); the pad
  offsets are tracked so normalized keypoints map back to true image coordinates
  (un-letterboxing) before ratios are computed.
- **Output tensor:** `f32 [1,1,17,3]`, last dim **`(y, x, score)`**, `y`/`x`
  normalized to `[0,1]` over the padded frame; `score` is per-keypoint
  confidence. Parsed into `PoseKeypoints` in COCO-17 order.

**Runtime — `ort` (ONNX Runtime bindings).** Pinned **`ort = "=2.0.0-rc.12"`**
(no stable 2.x exists yet; the 2.x API is the supported line — `Session::builder()
.commit_from_memory(MODEL)` / `session.run(ort::inputs![...])` / `extract`).
`ort` and ONNX Runtime are both **MIT/Apache-2.0**. `Session` is **`Send + Sync`**,
so `OnnxPoseEstimator` holds an **`Arc<Session>` loaded once at startup** and
shares it across requests; each `estimate()` runs the blocking `session.run` under
**`tokio::task::spawn_blocking`** (a cloned `Arc`), never stalling the async
runtime.

**Image decode — `image` (`0.25.x`, MIT/Apache).** Decode the stored JPEG/PNG
bytes → RGB8 → letterbox-resize to 256 → flatten to the NHWC `int32` buffer.
(`fast_image_resize` is a SIMD drop-in if resize cost ever matters.)

**Dependencies & CI (OQ-H5).** New api deps: `ort` (default features, incl.
`download-binaries`), `image`, and `ndarray` (ort's tensor interop). On GitHub
Actions **`ubuntu-latest` this works with no apt/system setup** — `ort`'s build
script fetches a prebuilt ONNX Runtime from pyke's CDN and links it. **Caveat
recorded:** that is a **build-time HTTPS fetch** (not hermetic) and the Docker
image must carry the copied `libonnxruntime` dylib; a vendored library
(`ORT_LIB_PATH`) or `load-dynamic` + `ORT_DYLIB_PATH` is the hermetic/container
alternative, deferred to the R-0026 deploy requirement (the CI smoke build here
uses the default path). The embedded model adds ~4.8 MB to the binary —
acceptable; the `.onnx` is committed under `backend/crates/api/models/` with the
model's Apache-2.0 `LICENSE`/`NOTICE` **shipped alongside it in the diff** (not
merely cited).

**Confidence floor.** Keypoints below **~0.2–0.3 score** are dropped before any
ratio (OQ-H4); the numeric ratio needs both shoulders + both hips above the
floor, else `derive_frame_features` returns `FrameError` → `422`. Exact thresholds
are tuned against the AC4 fixture image at impl time.

**Honesty (grounding-confirmed).** 2-D keypoints are a **skeletal-frame proxy
only** — bony-landmark positions. They **cannot** recover somatotype, muscle mass,
density, or body-fat (those need silhouette/segmentation/biometric inputs). So
`FrameFeatures` carries geometry and leaves `build`/`structure_tags` absent
(§2.2); the wire never presents a fabricated body-composition reading.

### 2.7 Testing (SAC → AC)

- **core unit (`core::pose`)** — `derive_frame_features` from hand-authored
  `PoseKeypoints`: a known wide-shoulder/narrow-hip geometry → expected ratio +
  bands; low-confidence keypoints → bands `None`; degenerate (zero hip span, too
  few points) → `FrameError`. (AC1)
- **core unit (`core::matching`)** — `rank` over `library()`: a `FrameFeatures`
  authored near a specific archetype ranks it first; absent categorical fields
  don't penalize; ranking is stable and total; `distance`/`score` bounds. (AC2)
- **api integration (fake estimator)** — `POST …/match`: ranked `200` shape +
  the privacy contract (no `internal_name`/`sources`); `422` no-pose; `422`
  empty session; `404` foreign/missing; `401`. (AC5/AC6/AC7)
- **api integration (real ONNX)** — one test loads `OnnxPoseEstimator` and runs a
  **committed fixture image** end-to-end (bytes → keypoints → features →
  ranking) → `200`, **and asserts the derived `shoulder_to_waist` lands in a
  plausible range** (not just status `200`) — so a silently-wrong preprocessing
  (NHWC/int32/letterbox mistake) that yields a distorted-but-non-erroring pose is
  caught. Gated only if CI cannot run it (decision in §2.6). (AC4)
- All gates green; CI carries the `ort`/ONNX-Runtime build (AC8).

### 2.8 Privacy & isolation (AC7)

Bytes are read through the R-0006 storage seam, decoded **in memory** for
inference, and never re-persisted or transformed on disk. Keypoints and
`FrameFeatures` are computed transiently and returned in the response but **not
stored** (persistence is R-0014). Every step is scoped to the token `sub`;
cross-user is `404`. The library is read via `library()`; nothing here writes the
prior (guardrail intact).

## 3. Code outline

```rust
// core/src/pose/mod.rs (excerpt)
pub enum Landmark { Nose, LeftShoulder, RightShoulder, LeftHip, RightHip, /* …COCO-17… */ }
pub struct Keypoint { pub x: f32, pub y: f32, pub score: f32 }
pub struct PoseKeypoints([Keypoint; 17]);
pub struct FrameFeatures {
    pub shoulder_to_waist: f64,
    pub clavicle_width: Option<WidthBand>,
    pub limb_length: Option<LengthBand>,
    pub confidence: f64,
}
/// # Errors
/// [`FrameError`] for too-few confident keypoints or degenerate geometry.
pub fn derive_frame_features(kp: &PoseKeypoints) -> Result<FrameFeatures, FrameError> { /* … */ }

// core/src/matching/mod.rs (excerpt)
pub struct RankedMatch<'a> { pub archetype: &'a Archetype, pub distance: f64 }
pub fn rank<'a>(f: &FrameFeatures, library: &'a [Archetype]) -> Vec<RankedMatch<'a>> { /* weighted, sorted */ }
```

```rust
// api/src/pose/mod.rs (excerpt)
#[async_trait]
pub trait PoseEstimator: Send + Sync {
    async fn estimate(&self, bytes: &[u8], content_type: ImageContentType)
        -> Result<PoseKeypoints, PoseError>;
}

// api/src/matching/handlers.rs (excerpt)
pub(crate) async fn match_session(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Json<MatchResponse>> {
    if !db::session_exists_for_user(&state.pool, user.user_id, session_id).await? {
        return Err(ApiError::NotFound);                         // foreign/missing → 404
    }
    let candidates = db::match_candidates_for_session(&state.pool, user.user_id, session_id).await?;
    if candidates.is_empty() {
        return Err(ApiError::Unprocessable { reason: "no_usable_photo" });
    }
    // front-angle first, then stored order; first usable pose wins (bounded).
    let keypoints = estimate_first_usable(&state, front_first(candidates)).await?; // 422 no_person_detected
    let features = derive_frame_features(&keypoints)?;          // 422 degenerate_frame
    let matches = rank(&features, fitai_core::archetype::library());
    Ok(Json(MatchResponse::from_ranked(&matches)))
}
```

`ApiError` gains `Unprocessable { reason: &'static str }` → `422` (§2.4);
`From<PoseError>`/`From<FrameError>` map onto it. `db::session_exists_for_user`
already exists (R-0006); `db::match_candidates_for_session` is the one new query
(returns `{angle, content_type, storage_key}` joined on the owner).

## 4. Non-goals

Inherits R-0013 §4: no program/diet generation or target persistence (R-0014); no
Python/sidecar; no on-device inference; no ML training; no deep photo analytics
(R-0018/19); no mobile UI; no caching of the ranking (compute-on-demand v1). Also:
no multi-photo fusion (v1 picks one photo), no storing keypoints/features.

## 5. Open questions (for the architect review)

- **OQ-H1 — Model + licence. RESOLVED → MoveNet SinglePose (Apache-2.0,
  COCO-17), Xenova fp16 `.onnx` embedded via `include_bytes!`** (§2.6). Thunder
  256 for ratio precision. YOLOv8-pose rejected (AGPL); RTMPose/BlazePose set
  aside (two-stage and/or weight-licensing nuance).
- **OQ-H2 — Feature↔profile mapping + weights.** Proposed: ratio 0.6 / clavicle
  0.2 / limb 0.2, absent-skip-renormalize, ordinal band distance (§2.3).
- **OQ-H3 — Multi-photo.** Proposed: v1 matches **one** photo (front-preferred,
  else highest-confidence); fusion deferred.
- **OQ-H4 — Confidence threshold.** Proposed: a per-keypoint floor + an aggregate
  floor below which it is `NoPersonDetected`/`FrameError` → `422`; exact values
  tuned against the fixture in impl.
- **OQ-H5 — `ort` linking + CI. RESOLVED → `ort = "=2.0.0-rc.12"` with default
  `download-binaries`** (works on ubuntu-latest with no system setup); `Arc<Session>`
  + `spawn_blocking` (§2.6). Recorded caveat: a build-time HTTPS fetch and a
  Docker dylib step — a hermetic/vendored alternative is an R-0026 decision.
- **OQ-H6 — Endpoint granularity.** Proposed: match a **session**
  (`POST /photo-sessions/:id/match`), compute-on-demand, no persistence.

## 6. Acceptance criteria

- [ ] **SAC1 → AC1.** `core::pose` derives `FrameFeatures` from keypoints (pure,
  validated, banded fields optional, no fabricated `build`/tags); unit-tested
  from fixed keypoints.
- [ ] **SAC2 → AC2.** `core::matching::rank` is a documented weighted
  nearest-neighbor, deterministic/total, absent-field-safe; unit-tested.
- [ ] **SAC3 → AC3.** A `PoseEstimator` seam with a real ONNX impl + a fake; the
  endpoint depends on `Arc<dyn PoseEstimator>` in `AppState`.
- [ ] **SAC4 → AC4.** The real impl loads a bundled permissive model and yields
  keypoints; one test runs it on a fixture image end-to-end; no image/keypoints
  persisted.
- [ ] **SAC5 → AC5.** `POST /photo-sessions/:id/match` → `200` + ranked matches
  (`ArchetypeResponse` + `distance`/`score`); `internal_name`/`sources` never on
  the wire.
- [ ] **SAC6 → AC6.** **Both** `422` triggers are tested — an empty/no-usable
  session (`no_usable_photo`) **and** a photo whose pose is below the floor
  (`no_person_detected`) or whose geometry is degenerate (`degenerate_frame`);
  foreign/missing session → `404` (cross-user 404-not-403); `401` unauthenticated.
- [ ] **SAC7 → AC7.** Scoped to `sub`; bytes via the seam, never leave the
  server; no new biometric storage.
- [ ] **SAC8 → AC8/AC9.** Unit + integration suites; gates green; CI carries the
  ONNX build; the diff is matching-only (no generation/persistence/training/UI).

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-13 | **Pure `core::pose` + `core::matching`, impure `api::pose` seam.** | Mirrors R-0006's pure-core/IO-edge split; the algorithm is unit-testable with no model, inference is swappable. |
| 2026-06-13 | **`PoseEstimator` trait (real ONNX + fake), `Arc<dyn>` in `AppState`.** | The R-0006 `ObjectStore` precedent; fast, deterministic suites; the model runs only where AC4 requires. |
| 2026-06-13 | **`FrameFeatures` omits `build`/`structure_tags` (Option for the rest).** | One 2-D photo yields geometry, not somatotype/density (R-0013 honesty); absent ≠ fabricated. |
| 2026-06-13 | **Weighted distance: ratio 0.6, clavicle 0.2, limb 0.2, absent-skip-renormalize.** | The numeric ratio is the most reliable single-photo signal; absent fields must not penalize (AC2). |
| 2026-06-13 | **Match a session, one photo (front-preferred), compute-on-demand, no persistence.** | Keeps R-0013 a read-only slice; multi-photo fusion + chosen-archetype persistence are later (R-0014). |
| 2026-06-13 | **Reuse the R-0012 `ArchetypeResponse` wire shape inside `RankedArchetype`.** | One wire contract for archetypes; `internal_name`/`sources` stay off the wire by construction. |
| 2026-06-13 | **Model: MoveNet SinglePose (Apache-2.0, COCO-17), `fp16` `.onnx` embedded via `include_bytes!`.** | Single-stage, tiny (~4.8 MB), licence granted on the artifact; emits exactly the joints matching needs. YOLOv8-pose (AGPL) disqualified for a closed-source product; RTMPose/BlazePose two-stage / weight-licensing nuance. (OQ-H1) |
| 2026-06-13 | **`ort = "=2.0.0-rc.12"`, `Arc<Session>` + `spawn_blocking`, default `download-binaries`.** | No stable 2.x yet (pin the rc); `Session` is `Send+Sync` so one shared session; inference is blocking so it leaves the async runtime; CI needs no system setup. Hermetic/Docker dylib is an R-0026 deploy decision. (OQ-H5) |
| 2026-06-13 | **MoveNet input is NHWC `int32` 0–255 with aspect-preserving letterbox (not NCHW-float).** | The model's actual contract (a widely-misreported gotcha); letterbox + un-letterbox keeps body ratios undistorted. |
| 2026-06-13 | **(architect finding 3) Add `ApiError::Unprocessable { reason: &'static str }` → `422`; map `PoseError::NoPersonDetected`/`FrameError` onto it; `Decode`/`Inference` → `500`.** | A no-usable-pose is not a request-field validation (`400`); a corrupt stored photo is a server fault (`500`). `reason` is a fixed token, not free text. |
| 2026-06-13 | **(architect findings 1–2) New owner-scoped `match_candidates_for_session` query; deterministic front-first, first-usable-pose selection (bounded), else `422`.** | The `PhotoSession` aggregate omits `storage_key`; a list-returning candidate query keeps multi-photo fusion additive and bounds inference to one success. |
| 2026-06-13 | **(architect findings 4–5) `f64::total_cmp` for ranking; AC4 fixture asserts a plausible ratio range; the model `LICENSE`/`NOTICE` ships in the diff.** | No `NaN` sort panic; catch silently-wrong preprocessing (green-but-wrong); honour the Apache-2.0 attribution. |

## Changelog

- _2026-06-13 — created (Draft). Realizes the accepted R-0013. Fixes the pure-core/seam design (pose keypoints → `FrameFeatures` → weighted ranked match) and the `POST /photo-sessions/:id/match` contract. §2.6/OQ-H1/H5 locked from a citation-backed grounding pass: **MoveNet SinglePose (Apache-2.0, COCO-17)** embedded via `include_bytes!`, **`ort = "=2.0.0-rc.12"`** with `Arc<Session>` + `spawn_blocking` and default `download-binaries` CI; the NHWC-int32 input gotcha and the 2-D-keypoints-are-skeletal-proxy-only honesty point are recorded. Ready for the architect design review._
- _2026-06-13 — architect design review **APPROVE WITH NITS**; all six findings applied: the `ApiError::Unprocessable`→`422` variant decided (not deferred), with `From<PoseError>`/`From<FrameError>` mapping and `Decode`/`Inference`→`500`; a new owner-scoped `match_candidates_for_session` query + a deterministic front-first/first-usable-pose selection (the `PhotoSession` aggregate omits `storage_key`); `f64::total_cmp` ranking; the AC4 fixture asserts a plausible ratio (not just `200`); the model `LICENSE`/`NOTICE` ships in the diff; SAC6 now enumerates both `422` triggers. Awaiting owner acceptance to close step 2._
