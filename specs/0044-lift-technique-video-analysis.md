# SPEC-0044 — Lift technique video analysis (`core::technique` + `api::technique`)

- **Status:** Draft — architect review REJECT (2026-08-25) reworked in full (§10)
- **Realizes:** R-0044 (as amended 2026-08-25 — AC0, AC8, AC9, AC10, AC10b,
  AC10c; see its changelog)
- **Author:** Claude (main session)
- **Created:** 2026-08-25
- **Depends on:** SPEC-0013 (`PoseEstimator`, `PoseKeypoints`, `Landmark`,
  MoveNet/ONNX — all shipped), SPEC-0041 (owner-scoped persistence, list-route
  projection precedent), SPEC-0042 (typed-refusal precedent, derived-state rule).
- **Consumed by:** R-0045 (lift biomechanics) reads the persisted keypoint
  series — §2.10 declares it a stable on-disk format.
- **Module(s):** `backend/crates/core/src/technique/` (new — pure, factored per
  §2.13); `backend/crates/api/src/technique/{mod,handlers}.rs` (new);
  `backend/migrations/00011_lift_analyses.sql` (new);
  `mobile/lib/src/technique/**` (new — capture, framing guide, results).
- **Touches existing:** `backend/crates/core/src/pose/mod.rs` (§2.10.1),
  `backend/crates/api/src/pose/mod.rs` (§2.12), `backend/crates/api/src/lib.rs`
  (`AppState` gains the analysis semaphore, §2.9.5).

## 1. Motivation

Realizes [R-0044](../requirements/0044-lift-technique-video-analysis.md). The
pose infrastructure exists and is idle: R-0013 shipped COCO-17 `Landmark`,
`PoseKeypoints`, a `PoseEstimator` trait with a test fake, and MoveNet embedded
as ONNX — all for still photos. **Video is that pipeline with a time axis.**

The first draft of this spec was rejected because three of its measurements were
physically impossible from COCO-17 and several of its algorithms were degenerate
in exactly the cases they existed to detect. The requirement was amended in step
(AC8/AC9/AC10 rewritten, AC10b/AC10c/AC0 added). This revision matches the
amended contract and carries the review's corrections; §10 maps every finding to
where it landed.

## 2. Design

### 2.1 Frames, not video — the decision that shapes everything

`PoseEstimator::estimate` takes **encoded image bytes**
(`api/src/pose/mod.rs:36-44`), not video. Rather than add a video decoder to
the API container, **the client extracts sampled frames and uploads those**:

```
phone: record → sample N frames as JPEG → upload multipart
server: PoseEstimator per frame → PoseKeypoints series → core::technique
```

Consequences:

- **No `ffmpeg` in the container.** A video decoder is a large native
  dependency and a well-known source of parser CVEs; the image path is already
  built, bounded, and tested. *This is the load-bearing reason.*
- **AC1 holds.** Frame extraction is not inference; the model still runs
  server-side. This is exactly the split `project-specifics.md` asks for: the
  phone captures, the server thinks.
- **The clip need never leave the phone** (R-0044 §4's retention concern
  dissolves — see §2.10.4).

**Upload size — corrected.** The rejected draft claimed "~50× smaller". That
number was computed for **10 frames** against a 30-second clip while the bound
in the very next section was **400 frames**; it was false. The honest
arithmetic, at the frame size §2.1.2 requires:

| | bytes |
|---|---|
| 40 s at `SAMPLE_HZ` = 10, 400 frames × ~35 KiB (480 px JPEG, q80) | ≈ 14 MiB |
| 40 s of 1080p H.264 at a phone's ~8–20 Mbps | ≈ 40–100 MiB |
| 10 s single-rep set: 100 frames × ~35 KiB | ≈ 3.4 MiB |
| 10 s of 1080p H.264 | ≈ 10–25 MiB |

So roughly **3–7× smaller**, not 50×, and the ratio is a function of the
client's encoder settings rather than a property of the design. Upload size is a
*pleasant side effect*, not a justification. The decision stands on the decoder
argument and the retention argument alone.

#### 2.1.1 The extraction mechanism (named, not hand-waved)

One Dart interface, `FrameExtractor`, over the platform's own frame reader:

| Platform | Mechanism | Availability |
|---|---|---|
| iOS | `AVAssetImageGenerator` with `maximumSize` and `requestedTimeToleranceBefore/After = .zero` | iOS 4+; not the constraint |
| Android | `MediaMetadataRetriever.getScaledFrameAtTime(us, OPTION_CLOSEST, w, h)` | **API 27** (Android 8.1) |

**Record-then-extract, not `startImageStream`.** The `camera` plugin's live
image stream would also work, but on low-end Android the ISP is already busy
encoding video and the stream drops frames unpredictably — precisely the
hardware variance `project-specifics.md` names as the reason inference is
server-side. Record-then-extract also lets the user *review the clip before
spending the upload*, which matters more on the target market's mobile data than
the milliseconds it costs.

**`ffmpeg_kit_flutter` must not be adopted.** The project was retired in 2025
and its prebuilt binaries were withdrawn; depending on it would mean vendoring
an unmaintained native media stack — the exact CVE surface §2.1 avoids.

**min-SDK implication — needs an owner decision.** `getScaledFrameAtTime` is
API 27. Flutter's current default floor is lower, so adopting it raises the
app's `minSdk` to **27**, cutting off Android 5–8.0 devices in a market with
old hardware. The alternative is `getFrameAtTime` (API 10) plus a Dart-side
downscale, which decodes a full-resolution frame on the device — slower and
memory-hungry on exactly the phones the lower floor exists to serve. **Proposed:
`minSdk = 27`.** Flagged for the owner; it is a product decision, not an
engineering one.

#### 2.1.2 Required output frame size

**Long edge ≤ 480 px, JPEG, quality ≈ 80.**

MoveNet Lightning's input is **192 × 192** (`INPUT_EDGE`, `api/src/pose/mod.rs`)
and `preprocess` letterboxes down to it with a Triangle filter. Anything above
roughly 2× that edge is bytes uploaded, decoded, and then thrown away. 480 px
keeps 2.5× headroom for the resampler at about a fifth of 1080p's bytes and
**zero** accuracy cost. Uploading 1080p is pure waste.

This is a *client contract*, not a server bound. The server's
`MAX_FRAME_PIXELS` (§2.3) is a hostile-input ceiling and is deliberately far
looser — the two numbers do different jobs and must not be conflated.

#### 2.1.3 When the device cannot hit `SAMPLE_HZ`

**Send what it got, with truthful `t_ms`** — the frame's actual presentation
timestamp, never the nominal grid. The server then refuses with
`IrregularSampling` (§2.6) rather than silently analysing a 3 Hz series as if it
were 10 Hz, which would misread every tempo and every turning point.

The client *should* also check locally and warn the user before uploading
("this phone couldn't sample fast enough — try a shorter clip, or more light").
That is a UX courtesy. **The correctness boundary is server-side**, because a
client that lies about its timestamps must not be able to buy a verdict.

### 2.2 The vector structure (AC5)

```rust
/// One sampled frame: when it was taken, relative to clip start, and where the
/// joints were.
pub struct PoseFrame { pub t_ms: u32, pub pose: PoseKeypoints }

/// A lift's full sampled series — the "video converted to its basics".
/// This is the stable on-disk format R-0045 reads (§2.10.2).
pub struct LiftSeries {
    pub schema_version: u16,   // 1
    pub lift: Lift,
    pub view: CameraView,
    /// Source frame dimensions, stored once. Load-bearing — see §2.2.2.
    pub frame_width: u32,
    pub frame_height: u32,
    pub sample_hz: u32,        // the nominal rate the series was captured at
    pub frames: Vec<PoseFrame>,
}

#[serde(rename_all = "snake_case")]
pub enum Lift { Squat, Bench, Deadlift }

#[serde(rename_all = "snake_case")]
pub enum CameraView { Side, Front }
```

A 40-second set at `SAMPLE_HZ = 10` is 400 frames × 17 joints × 3 floats —
20,400 numbers. Nothing downstream ever sees a pixel.

#### 2.2.1 `t_ms` is metadata, not a coordinate

**Every fault geometry is computed in frame-index space.** Rep boundaries,
turning points, touch frames, early-pull windows — all are indices into
`frames`. `t_ms` is consulted for exactly two things:

1. **Validating regularity** (§2.6, `IrregularSampling`).
2. **Converting an index span into seconds** for the per-rep tempo (§2.7.4).

This is deliberate. Interpolating geometry onto a time axis would make every
measurement depend on the client's clock; keeping it in index space means a
mis-timestamped clip is *refused*, never silently resampled.

`t_ms` is **relative to clip start**, so `frames[0].t_ms == 0` always. It is
`u32` because 40,000 ms fits trivially; `u32` **epoch** millis would wrap every
~49.7 days and is not an option. Validation, in order:

| Check | Failure |
|---|---|
| `frames[0].t_ms == 0` | 4xx `bad_timestamps` (well-formedness) |
| strictly increasing | 4xx `bad_timestamps` (well-formedness) |
| `last.t_ms ≤ MAX_CLIP_MS` | 4xx `clip_too_long` |
| median gap within `±MEDIAN_GAP_TOLERANCE` of `1000 / sample_hz` | `Refusal::IrregularSampling` |
| `max gap < MAX_GAP_FACTOR × median gap` | `Refusal::IrregularSampling` |

The split is the §2.9.4 rule applied to a genuinely ambiguous case:
*well-formedness* of the timestamps is a request contract the client can check
itself (4xx); *regularity* is a statement about capture quality the analysis
discovered (refusal).

#### 2.2.2 Coordinate space — and a contract on `PoseEstimator`

`Keypoint.x` / `.y` are **normalized to the model's letterboxed square canvas**,
not to the source image. `OnnxPoseEstimator::preprocess` scales the image by a
single uniform factor and centres it on a 192 × 192 black canvas, and
`parse_keypoints` passes MoveNet's normalized output through unchanged.

That has one consequence that the whole of §2.7 rests on: **the canvas space is
isotropic**. A uniform scale plus a translation preserves angles and preserves
length *ratios*, so every angle and every dimensionless ratio in this spec is
correct computed directly in canvas coordinates, with no aspect correction.

Had the coordinates been normalized to the source image independently in x and y
(`x/width`, `y/height`), they would be **anisotropic**: `hypot(dx, dy)` would be
meaningless and every angle would be skewed by the frame's aspect ratio. The
letterbox is what saves this, and nothing currently documents that.

**Therefore, as an explicit invariant of the `PoseEstimator` seam:** any
implementation must emit normalized coordinates in a **uniformly scaled**
(isotropic) space. `OnnxPoseEstimator` satisfies it via the letterbox;
`FakePoseEstimator` fixtures must be authored in the same space, or every
synthetic-series test is measuring a different geometry from production. A
doc-comment on the trait states this, and §2.12 makes it part of the fake's
contract.

`frame_width` / `frame_height` are therefore **not** needed for R-0044's own
ratios. They are stored because (a) R-0045 needs a pixel→metre scale, and (b)
the letterbox is invertible from `(frame_width, frame_height, INPUT_EDGE)`, so
those two integers are exactly what a consumer needs to recover source-image
pixels. Storing them costs 8 bytes per analysis and is unrecoverable later.

> *Out of scope, worth an issue:* `core::pose::derive_frame_features` computes
> `limb / torso` across mixed orientations. It is correct today only because of
> the same letterbox invariant, which is undocumented there too.

#### 2.2.3 A note `R-0045` must read before choosing quasi-static

`SAMPLE_HZ = 10` is **too coarse for double differentiation**, and that
effectively pre-resolves R-0045's OQ-2.

A squat turnaround lasts roughly 200 ms — **two samples** at 10 Hz. Acceleration
needs a second difference, whose noise is `√6 · σ / Δt²`. With MoveNet's
keypoint error at σ ≈ 4 % of torso length (§2.8) and Δt = 0.1 s:

```
√6 × 0.04 torso-lengths / 0.01 s²  ≈  9.8 torso-lengths/s²  ≈  4.6 m/s²
```

against a true turnaround acceleration of roughly 2–4 m/s². **The noise is
larger than the signal**, and smoothing cannot rescue it because the smoothing
window is wider than the event. So:

> R-0045 must either accept a **quasi-static** model on this series, or raise
> `SAMPLE_HZ` — which raises `MAX_FRAMES`, the bounds table, the per-row budget,
> and the analysis duration with it. Full inverse dynamics on a 10 Hz series
> would be confident nonsense of exactly the kind AC12 forbids.

Stated here rather than in SPEC-0045 so the constraint is inherited, not
rediscovered.

### 2.3 Bounds (AC2) — enforced before any work

| Bound | Value | Why |
|---|---|---|
| `SAMPLE_HZ` | 10 | a rep lasts 2–4 s; 10 Hz resolves the turnaround (and §2.2.3 states the ceiling this buys) |
| `MAX_FRAMES` | 400 | 40 s at 10 Hz; each frame is one inference |
| `MIN_FRAMES` | 20 | 2 s — below this no rep can be segmented |
| `MAX_FRAME_BYTES` | 256 KiB | ~5× the expected 480 px JPEG (§2.1.2); a pathology bound, not a target |
| `MAX_TOTAL_BYTES` | 32 MiB | request-level ceiling — 400 frames at a generous 80 KiB average |
| `MAX_FRAME_PIXELS` | 4 194 304 (4 MiPx) | decoded `width × height`; 12 MiB of RGB8 |
| `MAX_CLIP_MS` | 45 000 | `t_ms` span ceiling, with slack over `MAX_FRAMES / SAMPLE_HZ` |
| `BODY_LIMIT` | 34 MiB | `MAX_TOTAL_BYTES` + 2 MiB multipart slack (400 part headers ≈ 80 KiB) |
| `MAX_CONCURRENT_ANALYSES` | 2 | §2.9.5 |

#### 2.3.1 The bounds invariant, asserted at compile time

The rejected draft had `MAX_FRAME_BYTES` (2 MiB) × `MIN_FRAMES` (20) = 40 MiB
against a `MAX_TOTAL_BYTES` of 24 MiB — a *minimum-length* request at the
maximum legal frame size was structurally impossible to submit. The bounds must
satisfy:

```
MAX_FRAME_BYTES × MIN_FRAMES  ≤  MAX_TOTAL_BYTES  ≤  MAX_FRAME_BYTES × MAX_FRAMES
```

- **Left:** the shortest legal request at the largest legal frame must fit, or
  the total cap silently forbids a request the frame caps permit.
- **Right:** the total cap must be reachable, or it is vacuous decoration.

With the values above: `256 KiB × 20 = 5 MiB ≤ 32 MiB ≤ 256 KiB × 400 =
100 MiB`. ✓

This is **pinned as a compile-time assertion** beside the constants, so a future
edit to any one of the four cannot reintroduce the contradiction:

```rust
const _: () = assert!(MAX_FRAME_BYTES * MIN_FRAMES <= MAX_TOTAL_BYTES);
const _: () = assert!(MAX_TOTAL_BYTES <= MAX_FRAME_BYTES * MAX_FRAMES);
```

A runtime `#[test]` restating it as a property is listed in §8 for readers who
prefer to see it in the suite; the `const` assertion is the one that cannot be
skipped.

#### 2.3.2 The decoded-dimension bound — the OOM hole

A 2 MiB JPEG can declare enormous dimensions and decode to **gigabytes** of
RGB8; the JPEG spec allows 65 535 × 65 535, which is ~12.9 GB decoded. The
container has **1 GiB** (`fly.toml`). A byte-size bound does not bound the
decode at all.

So, **before any full decode**, per frame:

```rust
let (w, h) = image::ImageReader::new(Cursor::new(&bytes))
    .with_guessed_format()?
    .into_dimensions()?;                       // header only — no pixel decode
if (w as u64) * (h as u64) > MAX_FRAME_PIXELS { reject }
```

`ImageReader::into_dimensions()` reads the header and stops. Only after it
passes does the frame reach `PoseEstimator::estimate`.

> **The existing photo path shares this hole.** `api::pose::preprocess` calls
> `image::load_from_memory` — a *full* decode — on bytes validated only by
> `photo::MAX_BYTES` (10 MiB). Out of scope for R-0044, but it is a live
> denial-of-service on a 1 GiB container and is worth its own issue.

#### 2.3.3 Streaming multipart

Bounds are useless if the body is buffered first. Parts are validated **as they
arrive**:

1. Read the `lift`, `view`, `session_id?` text parts.
2. For each frame part, in order: enforce `MAX_FRAME_BYTES` on that part,
   enforce `MAX_FRAME_PIXELS` from its header (§2.3.2), add to a **running
   total** checked against `MAX_TOTAL_BYTES`, and increment a **running count**
   checked against `MAX_FRAMES`. Any breach aborts the request immediately —
   the remaining parts are never read.
3. Only once the whole body is read and `count ≥ MIN_FRAMES` does **any**
   inference run.

`DefaultBodyLimit::max(BODY_LIMIT)` is layered on the routes, exactly as
`photo::mod.rs` does, so a body beyond the ceiling is rejected at the layer
(`413`) before the handler is entered at all. The two mechanisms are
complementary: the layer stops the pathological body, the streaming validator
stops the merely-oversized one with a precise field token.

**N inferences is the expansion an attacker controls** — the R-0041 `sets`
lesson, at 400× the unit cost.

### 2.4 The pipeline and its ordering (AC12 — refusal before verdict)

```rust
/// Pure, total, deterministic. No I/O, no model, no clock.
/// `calib` is `None` in v1 (§2.11) — its presence narrows uncertainty and
/// never changes a code path.
pub fn analyze(series: &LiftSeries, calib: Option<&Calibration>) -> AnalysisOutcome;
```

Order, each step gated on the last:

| # | Step | Refusal |
|---|---|---|
| 1 | frame count | `TooFewFrames` |
| 2 | timestamp regularity (§2.2.1) | `IrregularSampling` |
| 3 | required landmarks present in ≥ `MIN_FRAME_COVERAGE` of frames | `OutOfFrame` |
| 4 | mean confidence over *this lift's* load-bearing joints | `LowConfidence` |
| 5 | quiet-stance window (§2.6.1) | `NoStableStart` |
| 6 | view classification on that window + stability (§2.5.4) | `WrongView`, `UnstableView` |
| 7 | camera-static check over the segmentation window (§2.6.4) | `CameraMoved` |
| 8 | rep segmentation (§2.6.2–2.6.5) | `NoRepsDetected` |
| 9 | per-lift findings (§2.7) | — |

**No verdict is computed until 1–8 pass.** The ordering is not arbitrary:

- Confidence precedes everything geometric because the cues are themselves
  keypoint-derived — classifying a view from untrustworthy points is fitting
  noise.
- The quiet-stance window precedes view classification and the camera check
  because **both are defined on it** (§2.5.4 classifies on its median; §2.6.4
  would otherwise report `CameraMoved` on every squat walkout).

Internally the steps return `Result<_, Refusal>` so `?` composes; `analyze`
converts once at the boundary. There is exactly one persisted/wire
representation (`AnalysisOutcome`, §2.10.3) and it is not `Result`.

**On step 1 and the §2.9.4 rule:** the api edge rejects an under-`MIN_FRAMES`
upload with a 4xx *before a series is ever built*, so `TooFewFrames` is
unreachable over HTTP. It stays in the enum because `analyze` must be **total** —
a direct unit test may hand it a 3-frame series and must get a typed answer, not
a panic.

### 2.5 View detection (OQ-3, re-resolved)

**The rejected draft's rule was wrong, not merely imprecise.** It used
shoulder-width ÷ hip-width. Under a yaw rotation `θ` away from front-on, *both*
spans are frontal-plane segments and both foreshorten by the **same** factor:

```
shoulder_span / hip_span  =  (S·cos θ) / (H·cos θ)  =  S / H
```

The cosine cancels exactly. The ratio is **constant in the very variable it was
supposed to measure**, and side-on it is `0 / 0`.

#### 2.5.1 The primary cue

```
view_ratio = shoulder_span / torso_length
torso_length = ‖ shoulder_mid − hip_mid ‖     (EUCLIDEAN, not vertical)
```

The shoulder span is a frontal-plane segment: it images as `S·cos θ`. The torso
is a segment along the body's long axis; a yaw rotation is *about* that axis, so
its image length is unchanged. The ratio is a clean cosine, `(S/T)·cos θ` —
maximal front-on, collapsing to zero side-on.

**Euclidean, and this is the point.** A *vertical* torso measure would collapse
when the lifter pitches forward — which is the deadlift setup's whole posture.
The Euclidean length does not: for the side view deadlift requires, the pitch is
a rotation **within the image plane**, so the segment rotates but keeps its
length. (Front-on, a pitch foreshortens the torso and pushes `view_ratio`
*higher*, i.e. further into "Front" — the correct classification anyway. Pitch
can never move a Side view out of Side.)

#### 2.5.2 Corroboration and an orthogonal cue

One ratio is not enough — a mis-detected shoulder swings it. Classification
requires **agreement** from a second cosine cue and support from a third that
does not depend on spans at all:

| Cue | Side | Front |
|---|---|---|
| `shoulder_span / torso_length` | ≤ `VIEW_SIDE_MAX` (0.30) | ≥ `VIEW_FRONT_MIN` (0.75) |
| `hip_span / torso_length` | ≤ `HIP_VIEW_SIDE_MAX` (0.22) | ≥ `HIP_VIEW_FRONT_MIN` (0.55) |
| `min(ear_score) / max(ear_score)` | ≤ `EAR_RATIO_SIDE_MAX` (0.35) | ≥ `EAR_RATIO_FRONT_MIN` (0.70) |
| `｜nose.x − shoulder_mid.x｜ / torso_length` | ≥ `NOSE_OFFSET_SIDE_MIN` (0.12) | ≤ `NOSE_OFFSET_FRONT_MAX` (0.06) |

The ear/nose cues are **orthogonal to the span cues** — they come from
occlusion and head geometry, not from foreshortening, so they fail differently.
Side-on, the far ear is behind the head and MoveNet scores it near zero while
the nose sits laterally displaced from the head's centre; front-on, both ears
score similarly and the nose is centred.

**View detection is the one place that deliberately uses both sides** of a
bilateral pair — the foreshortening *is* the signal. Everywhere else §2.7.1's
near-side rule applies.

#### 2.5.3 Two thresholds and a deliberately wide refusal band

A single cut would classify a 45° camera as *something*. There are two, and
everything between them is `WrongView`:

```
view_ratio ≤ 0.30  ⇒ Side          (nominal S/T ≈ 0.85 ⇒ yaw ≳ 69°)
view_ratio ≥ 0.75  ⇒ Front         (⇒ yaw ≲ 28°)
0.30 < ratio < 0.75 ⇒ Indeterminate ⇒ refuse
```

The band spans yaw 28°–69° — nearly half the quadrant — and that width is
**intentional**, for a reason worth naming: `S/T` varies between people by
roughly ±15 %, so the ratio is `(person's S/T) · cos θ` and the person's
constant is unknown. A narrow band would trade a real refusal for a
person-dependent guess. This is also the cleanest thing AC7 calibration would
fix — supplying the lifter's own `S/T` collapses the band (§2.11).

#### 2.5.4 Classify on the median, then check stability

Per-frame cues are noisy and, mid-rep, genuinely change (a squatting lifter's
torso pitches). So:

1. Compute the four cues on each frame of the **standing / setup window** found
   by §2.6.1 — the one stretch where posture is defined.
2. Take the **median** of each cue over that window and classify once.
3. Then classify **every** frame in the series independently and require the
   result to be stable: if more than `MAX_UNSTABLE_FRACTION` (10 %) of frames
   disagree with the median classification, that is
   `Refusal::UnstableView { side, front, indeterminate }`.

**Unstable classification is itself a refusal.** A series that reads Side for
its first half and Front for its second is a panned camera or a rotating lifter;
either way there is no single view the geometry can assume, and assuming one
would be a confident wrong verdict.

Framing (`OutOfFrame`) is separate and prior: the landmarks each lift needs must
clear the confidence floor in ≥ `MIN_FRAME_COVERAGE` (90 %) of frames.

### 2.6 Rep segmentation (AC6, OQ-2 re-resolved)

**Sign convention, stated once:** image `y` increases *downward*. All of §2.6
and §2.7 works on `height = −y`, so "up" means increasing. Every claim below
depends on this.

**Signal:** hip-midpoint height for squat and deadlift; wrist-midpoint height
for bench. (Side view: near side only — §2.7.1.)

#### 2.6.1 Quiet-stance detection — skip the walkout

The rejected draft segmented from frame 0. A squat clip begins with a
**walkout**: unrack, two or three steps back, a settle. A bench clip begins with
a **rack-out**: a horizontal wrist translation off the hooks. Both produce large
excursions that a turning-point detector happily counts as reps, and both
corrupt the "standing reference" every excursion is measured against.

Find the **first** window of `QUIET_WINDOW_FRAMES` consecutive frames in which
*both* hold:

- hip-midpoint height range ≤ `QUIET_TOL` × torso_length, **and**
- near-side ankle horizontal range ≤ `QUIET_TOL` × shank_length

```
QUIET_WINDOW_FRAMES = max(3, round(0.5 s × SAMPLE_HZ))   // 5 at 10 Hz
QUIET_TOL           = 0.08
```

`QUIET_TOL` is sized between two hard constraints: it must exceed MoveNet's own
keypoint noise (±3–5 % of torso length — a 5-frame range of a ±4 % process runs
to ~8 %) or it can never trigger; and it must sit far below a walkout step
(~0.3 m ≈ 60 % of torso length) or it triggers during one. 0.08 sits at the
first bound with two-thirds of the gap to the second in hand.

- The window's end is where segmentation **starts**. Everything before it —
  walkout, rack-out, setup shuffle — is discarded.
- The window is also the **standing reference** height, and the window §2.5.4
  classifies the view on, and the window §2.6.4's camera-static check runs over
  (a walkout moves the ankles legitimately; checking over the whole clip would
  report `CameraMoved` on every squat).
- No such window ⇒ `Refusal::NoStableStart` — the lifter was never still, or
  the clip starts mid-set.

**Deadlift is the sign-flipped case.** Its quiet window is the **setup**: hips
low and static, not standing. The reference is therefore a *low* position and
rep excursion is measured **upward** from it. One constant, one explicit
per-lift sign — not a special case in the algorithm.

#### 2.6.2 Centred moving average, window derived from `SAMPLE_HZ`

```
w      = max(1, round(SMOOTH_SECONDS × SAMPLE_HZ / 2))    // 2 at 10 Hz ⇒ 5-frame window
smooth[i] = mean(signal[i−w ..= i+w])                     // shrunk at the edges
SMOOTH_SECONDS = 0.5
```

**Centred, not trailing.** A trailing average lags the signal by `w` frames,
which shifts every detected turning point *later* by a fixed amount. That lag
cancels in a *duration* (both ends move together) but **not** in an *index*: the
"bottom frame" and the "touch frame" are where §2.7 reads depth and forearm
angle, and reading them two frames late measures the wrong instant of the lift.
A phase-shifted turning point is a silently wrong measurement, which is the
failure mode this whole spec is organized against.

At the edges the window **shrinks** to what exists rather than padding —
padding invents data at exactly the frames where the lifter is standing still,
biasing the standing reference.

#### 2.6.3 Prominence, not bare reversal

The rejected draft took "the direction of travel reverses" as a turning point.
Two extremely common cases break it:

- A **paused squat** — a 2 s hold at the bottom — is a flat minimum with
  micro-reversals from keypoint noise. It counts as 2–3 reps.
- A **grinding rep** with a mid-ascent stall reverses slightly at the sticking
  point. It counts as 2 reps.

So candidate extrema are filtered by **topographic prominence**: the prominence
of a minimum is how far the signal must climb from it before reaching a lower
minimum (the mirror of the standard peak-prominence definition). Noise wiggles
and stall bumps have near-zero prominence; the genuine bottom of a rep has the
full rep excursion.

```
MIN_REP_PROMINENCE = 0.25 × torso_length
```

Well above the ±4 % noise floor, well below a real rep's ~0.75 torso-length
excursion.

#### 2.6.4 Excursion from the reference, and the camera-static check

Prominence alone still accepts an excursion that never departs the standing
position — a shift of weight while re-racking. So a rep's extremum must also be
at least `MIN_REP_EXCURSION` **from the standing reference**, in the lift's own
direction:

```
MIN_REP_EXCURSION = 0.20 × torso_length
squat, bench : extremum ≤ reference − MIN_REP_EXCURSION   (down)
deadlift     : extremum ≥ reference + MIN_REP_EXCURSION   (up)
```

Real excursions are ~0.75 torso lengths, so 0.20 is a floor, not a threshold on
technique.

**Camera-static check (`CameraMoved`).** Over the segmentation window only,
the standard deviation of the landmarks that must be still:

| Lift | Landmark | Normalizer |
|---|---|---|
| Squat, Deadlift | ankle (near side for Side; both for Front) | shank length |
| Bench | hip midpoint | torso length |

Exceeding `MAX_STATIC_DRIFT = 0.15` ⇒ `Refusal::CameraMoved`. Standard
deviation, not range — one bad frame must not condemn a clip.

**Honest caveat, and it goes in the user-facing string:** a lifter whose feet
genuinely shift trips the same check. The refusal says *"the camera or your feet
moved during the set"* and does not assert which. Asserting would be a guess.

#### 2.6.5 Touch-and-go deadlifts

Only **rep 1** has a static setup to reference. Reps 2..N in a touch-and-go set
start from wherever the bar was set down, which is not the original setup
height. Each rep 2..N therefore references **its own start frame** — the
prominent minimum that terminated the previous rep's descent — not the global
reference. Referencing rep 1's setup would make every subsequent rep's excursion
and torso-angle change measure a difference from a position the lifter is no
longer in.

Fewer than one rep after all of the above ⇒ `Refusal::NoRepsDetected`.

Each rep is `Rep { start: usize, extremum: usize, end: usize }` — frame indices,
per §2.2.1.

### 2.7 Per-lift measurements (AC8–AC11)

#### 2.7.1 Near-side resolution — one rule, applied everywhere

For `CameraView::Side`, **every bilateral landmark resolves to the near side**:
the side whose mean keypoint score over the **whole series** is higher, chosen
**once** and applied to every frame.

- **Once, not per frame.** Per-frame selection injects a step discontinuity into
  every signal at each frame where the far limb momentarily out-scores the near
  one — a fabricated jump in the middle of a rep.
- **The far side is never averaged in.** A midpoint of a near and an occluded
  far landmark is a fabricated centre-line whose error grows with limb
  separation — largest at the bottom of a squat, which is exactly where depth is
  read.
- Consequently, for Side view the "shoulder mid", "hip mid" and "wrist mid" of
  §2.2/§2.6 are the **near-side** shoulder, hip and wrist. AC5's parenthetical
  "wrist-midpoint bar proxy" is the general phrasing; **for a Side view the bar
  proxy is the near-side wrist**, because the far wrist is behind the lifter's
  own body. (Flagged to the owner as a wording nit in R-0044 AC5 — the substance
  is unchanged.)

For `CameraView::Front`, both sides are used and knee travel is reported **per
side**.

The single exception is §2.5's view detection, which needs both sides by
construction.

#### 2.7.2 Normalizers

All dimensionless, all from the lifter's own body, all measured in the isotropic
canvas space of §2.2.2, all taken at the reference frame (not per frame — a
per-frame normalizer would make a foreshortening limb rescale the measurement it
normalizes):

| Name | Segment |
|---|---|
| `torso_length` | shoulder → hip |
| `shank_length` | knee → ankle |
| `thigh_length` | hip → knee |
| `forearm_length` | wrist → elbow |

#### 2.7.3 The measurements

**Squat — Side**

- **`SquatDepth`** — the signed angle of the hip→knee segment relative to
  horizontal at the rep's extremum frame, in degrees. Negative = hip below knee.
  Reported relative to parallel, matching AC11's `hip 8° above parallel`.
  *Systematic bias, disclosed (AC10c):* MoveNet reports the hip **joint centre**;
  the depth standard is the hip **crease**, which sits anterior and superior to
  it by roughly 3–5 cm. On a ~40 cm thigh that is **4–7° in the optimistic
  direction** — measuring at the joint centre reads *deeper* than the crease
  standard. That is the dangerous direction: it is precisely "telling someone
  their depth is fine when it is not". The offset is carried in `uncertainty`
  (§2.8), it is why depth ships with **no severity** in v1 (§2.14), and it is
  what AC7 calibration exists to remove (§2.11).
- **`SquatTorsoAngleChange`** — the angle of the shoulder→hip vector from
  vertical; reported as `max |angle(f) − angle(rep.start)|` over the rep, in
  degrees. **Renamed from "spine angle change"** per AC8. **This is not a back
  check** — see §2.7.5.

**Squat — Front**

- **`KneeTravelInward`** — the signed horizontal offset of the knee from the
  ankle→hip line, normalized by `shank_length`, at the rep's extremum, **per
  side**. Named for the movement, not the clinic (§2.7.6).

**Bench — Side**

- **`BenchBarPathDeviation`** — horizontal excursion of the bar proxy over the
  rep, normalized by `forearm_length`.
- **`BenchForearmAngleAtTouch`** — the angle of the wrist→elbow vector from
  vertical at the touch frame, signed, in degrees. **Replaces "elbow flare"**
  per AC9: flare is humeral abduction, a *frontal-plane* angle, and from a
  camera perpendicular to the bench the upper arm points along the lens axis
  while the far arm is occluded. Forearm-vertical-under-the-bar is the
  sagittal-plane equivalent and a real coaching cue.
- **`BenchTouchPointConsistency`** — the standard deviation of the bar proxy's
  position at the touch frame across reps, **normalized by `forearm_length`**.
  **At a single rep this yields no finding at all** — `Unavailable { SingleRep }`
  (§2.8.1), never a fabricated variance of 0.0, which would read as perfect
  consistency.

**Deadlift — Side**

- **`DeadliftHipRiseBeforeBar`** — the **ratio** `Δhip_height / Δbar_height`
  over the early pull.
  *Window:* rep start to the first frame where the bar proxy has risen by
  `BAR_BREAK_EPSILON` (0.05) × `shank_length`.
  *Reading:* ≈ 1.0 is a clean pull (hips and bar rise together); ≫ 1 is the
  "stripper deadlift".
  **Not a correlation.** The rejected draft correlated hip-vertical and
  wrist-vertical velocity. Correlation against a **near-constant** bar signal is
  degenerate: when the bar barely moves, its variance is ~0 and the correlation
  coefficient's denominator collapses — the statistic is undefined *exactly in
  the case it exists to detect*, and in floating point it returns whatever the
  noise happens to be.
  *Guard:* if the bar never breaks `BAR_BREAK_EPSILON` within the rep, the ratio
  is undefined — `Unavailable { BarDidNotBreak }`, a typed outcome (§2.8.1), not
  a division by ~0.
- **`DeadliftTorsoAngleChange`** — the shoulder→hip angle from vertical at the
  **rep start frame** versus at the **frame of maximum change during the pull**,
  reported as that maximum in degrees together with its frame index.
  **The rejected draft said "setup versus bottom".** For a deadlift *the setup
  **is** the bottom* — the two frames it compared are the same frame, and the
  measurement was identically zero. Renamed and redefined.
- **`DeadliftBarDriftFromAnkle`** — the horizontal distance of the bar proxy
  from the **near-side ankle**, peak over the rep, normalized by
  **`shank_length`**.
  *Disclosed bias (AC10):* COCO-17 has **no foot landmark** — it ends at the
  ankle, so mid-foot does not exist and neither does foot length. The mid-foot
  sits roughly 5–8 cm *anterior* to the ankle, so this measurement is
  **posteriorly biased**: it reads systematically larger than true mid-foot
  drift by that constant. That is the *safe* direction — it over-flags a
  coaching cue rather than clearing a real drift — but it is a bias and it is
  stated, in the spec and in the UI.

#### 2.7.4 Per-rep tempo (AC5 names tempo; the rejected output lacked it)

```rust
pub struct RepTempo {
    /// Squat/bench: start → extremum. Deadlift: lockout → next rep's start.
    /// `None` when the phase is not in the clip (e.g. a dropped last deadlift).
    pub eccentric_s: Option<f64>,
    /// Squat/bench: extremum → end. Deadlift: setup/start → lockout.
    pub concentric_s: f64,
}
```

Squat and bench are eccentric-first; the deadlift's pull is concentric-first and
its eccentric may simply not exist (the bar was dropped). `None` says so; a
fabricated 0.0 would not.

Seconds are `(t_ms[b] − t_ms[a]) / 1000.0` — the **only** place `t_ms` reaches
the output (§2.2.1).

#### 2.7.5 Not measurable from COCO-17

*This section exists because of AC10b, and its content must appear in the UI —
not only in this document.*

- **Back rounding / spinal flexion is NOT detectable.** COCO-17 has no spine
  landmark. The shoulder→hip line measures torso **inclination**; a neutral
  spine and a fully rounded spine at the same hip angle produce an *identical*
  line. `SquatTorsoAngleChange` and `DeadliftTorsoAngleChange` are **not** back
  checks and must never be presented, labelled, or cued as though they were.
  Substituting torso lean for a spine check would be exactly the silent
  substitution AC12 forbids, on the fault most likely to injure someone.
- **Mid-foot and foot length do not exist.** COCO-17 ends at the ankle — no
  heel, no toe, no foot. "Bar over mid-foot" cannot be measured and no
  foot-length normalizer is available. See `DeadliftBarDriftFromAnkle`.
- **Elbow flare (humeral abduction) is not measurable** from the prescribed side
  view: a frontal-plane angle, with the upper arm along the lens axis and the
  far arm occluded.
- **Bar position, grip width, stance width, foot rotation** are not measurable.
  There are no foot landmarks and the bar itself is never detected — the wrist
  is a proxy for it. (These are R-0045's *variables*, supplied by the coach, not
  measured here.)
- **Head and neck position** are not reported. Nose/eyes/ears exist but a neck
  angle derived from them is dominated by their own noise.
- **Lift variants.** v1 assumes conventional deadlift, back squat, flat bench.
  Sumo, front squat and their kin change the mechanics enough that these numbers
  are not comparable; the framing guide says so and variants are out of scope
  (they are R-0045 AC7's variables).

#### 2.7.6 `Fault`, and its exact user-facing strings (AC14)

Enumerated here in full so that "no medical or injury claims" is **mechanically
reviewable** rather than a promise about copy someone will write later. §8
asserts these exact strings.

```rust
#[serde(rename_all = "snake_case")]
pub enum Fault {
    SquatDepth,
    KneeTravelInward,
    SquatTorsoAngleChange,
    BenchBarPathDeviation,
    BenchForearmAngleAtTouch,
    BenchTouchPointConsistency,
    DeadliftHipRiseBeforeBar,
    DeadliftTorsoAngleChange,
    DeadliftBarDriftFromAnkle,
}

impl Fault {
    pub fn label(self) -> &'static str;
    pub fn cue(self) -> &'static str;
}
```

| Variant | `label()` | `cue()` |
|---|---|---|
| `SquatDepth` | `Depth at the bottom` | `Aim to bring the hip crease level with the top of the knee.` |
| `KneeTravelInward` | `Knee travel (inward)` | `Think about spreading the floor with your feet so the knees track over the toes.` |
| `SquatTorsoAngleChange` | `Torso angle change` | `Try to hold the torso angle you start the rep with, all the way down and up.` |
| `BenchBarPathDeviation` | `Bar path (horizontal travel)` | `Aim for the bar to travel the same line down and up.` |
| `BenchForearmAngleAtTouch` | `Forearm angle at the chest` | `At the chest, aim to have the forearm vertical under the bar.` |
| `BenchTouchPointConsistency` | `Touch point consistency` | `Aim to touch the same spot on the chest each rep.` |
| `DeadliftHipRiseBeforeBar` | `Hip rise before the bar moves` | `Try to start the hips and the bar together rather than letting the hips rise first.` |
| `DeadliftTorsoAngleChange` | `Torso angle change during the pull` | `Aim to hold the torso angle you set up with until the bar passes the knee.` |
| `DeadliftBarDriftFromAnkle` | `Bar distance from the ankle` | `Aim to keep the bar close to the leg through the pull.` |

**"Valgus" is deliberately absent.** It is clinical vocabulary; a lifter reads
it as a diagnosis. `KneeTravelInward` describes the movement, which is all AC14
permits. Every string above describes a movement or suggests a cue; none names
a body part as damaged, predicts an injury, or implies a diagnosis.

*Follow-up (noted, not scoped here):* these strings are English-only while the
target market is Mexico and LATAM. Spanish copy is a separate piece of work and
the `&'static str` shape will have to become a lookup at that point.

### 2.8 `Finding`, uncertainty, and severity (AC10c, AC11, AC13)

```rust
pub struct Finding {
    pub fault: Fault,
    pub rep: u32,
    pub side: Option<Side>,            // Some for per-side faults (knee travel)
    pub value: f64,
    pub unit: Unit,                    // Degrees | NormalizedLength | Ratio | Seconds
    /// Same `Unit` as `value`. Half-width of the interval, not a percentage.
    pub uncertainty: f64,
    /// `None` = no accepted threshold exists for this fault yet (§2.14).
    pub severity: Option<FindingSeverity>,
    /// Mean keypoint confidence of the landmarks this value rests on.
    pub confidence: f64,
}

#[serde(rename_all = "snake_case")]
pub enum FindingSeverity { Ok, Borderline, Flagged }
```

> `FindingSeverity`, not `Severity`: `core::adjust::Severity` is already
> re-exported at the crate root, and two `Severity` types one `use` apart is the
> kind of collision that produces a wrong import that still compiles.

**Uncertainty is computed, not guessed.** A keypoint position uncertainty

```
σ_kp = KEYPOINT_SIGMA_FRACTION × torso_length          KEYPOINT_SIGMA_FRACTION = 0.04
```

(MoveNet Lightning's reported keypoint error on in-the-wild frames) is
propagated to first order through each measurement's own geometry:

| Measurement shape | `uncertainty` |
|---|---|
| angle between two points separated by `L` | `√2 · σ_kp / L` radians → degrees |
| normalized offset over normalizer `N` | `√2 · σ_kp / N` |
| ratio `A/B` | `｜A/B｜ · √((σ_A/A)² + (σ_B/B)²)` |
| plus, for `SquatDepth` | ⊕ the joint-centre → crease offset, 4–7° (§2.7.3) |

**`Borderline` is redefined per AC10c.** It is **not** "close to the threshold".
It is:

> the interval `[value − uncertainty, value + uncertainty]` **straddles** the
> threshold.

- `Ok` — the whole interval is on the pass side.
- `Flagged` — the whole interval is on the fail side.
- `Borderline` — it straddles.

The difference matters: "close to" is a second arbitrary constant layered on the
first, while a straddling interval is a statement about what the measurement can
actually support. Where a measurement carries a systematic bias comparable to
the decision margin — depth — the interval is wide, so almost everything is
`Borderline`, and *that is the honest answer*, not a bug to tune away.

#### 2.8.1 Saying what could not be measured

A finding that cannot be produced is **reported as absent with a reason**, never
omitted and never fabricated:

```rust
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Measurement {
    Measured(Finding),
    Unavailable { fault: Fault, reason: Unmeasurable },
}

#[serde(rename_all = "snake_case")]
pub enum Unmeasurable {
    SingleRep,             // touch-point consistency needs ≥ 2 reps
    BarDidNotBreak,        // hip-rise ratio has no denominator
    LandmarkNotConfident,  // the points this rests on are below the floor
    NotThisView,           // e.g. knee travel from a Side clip
}
```

This is what makes AC10b operable at the level of a single measurement, not just
in a static help page: the output can *say* "I could not measure this, and
here is why".

```rust
pub struct LiftAnalysis {
    pub view: CameraView,          // AC4/AC13: the assumed view is always stated
    pub rep_count: u32,
    pub tempo: Vec<RepTempo>,      // one per rep (§2.7.4)
    pub measurements: Vec<Measurement>,
    /// The AC10b copy, carried with the result so the UI cannot omit it.
    pub not_measurable: &'static [&'static str],
}
```

### 2.9 The endpoints — asynchronous (AC0, OQ-5 re-resolved)

**OQ-5 is resolved the other way: analysis is asynchronous.** Owner decision,
now AC0. The rejected draft's "10 frames × MoveNet is well inside a request
timeout" was arithmetic against a frame count the spec did not permit: at
`MAX_FRAMES` = 400, serialized inference on the container's **1 shared CPU**
(`fly.toml`) is tens of seconds, against a client read timeout of ~5 s.

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/technique/analyses` | multipart: `lift`, `view`, `session_id?`, N frames → **`202`** `{ id, state: "pending" }` |
| `GET` | `/technique/analyses` | caller's analyses, newest first, **without `series`** |
| `GET` | `/technique/analyses/:id` | one analysis in whatever state it is in |
| `DELETE` | `/technique/analyses/:id` | `204` |

Owner-scoped through one `load_owned` (404, never 403 — AC16),
`AuthenticatedUser` before `Path`/`Multipart`, `:id` route syntax, exhaustive
token mapping — all per the R-0041/R-0042 conventions.

#### 2.9.1 The three-state machine

```rust
#[serde(rename_all = "snake_case", tag = "state")]
pub enum AnalysisState {
    Pending,
    Done { outcome: AnalysisOutcome },   // Analyzed{..} | Refused{..}
    Failed { reason: FailureReason },    // Inference | Decode | TimedOut
}
```

**`Failed` and `Done { Refused }` are different things and must never merge.**
`Failed` means *the machine broke* — ONNX faulted, a frame would not decode, the
task was lost. `Done { Refused }` means *the analysis ran and honestly declined
to score*. Collapsing them would let a server fault read to the user as "your
video was bad" — the AC12 dishonesty, inverted.

#### 2.9.2 The request path

```
POST /technique/analyses
  1. AuthenticatedUser
  2. semaphore.try_acquire_owned()          → 503 fast if saturated (§2.9.5)
  3. stream multipart, validating per §2.3.3 → 4xx on any breach
  4. INSERT (state='pending')                → 202 { id, state: "pending" }
  5. tokio::spawn(task owning the permit + the frame bytes)
```

The task then: `PoseEstimator::estimate` per frame → build `LiftSeries` →
`core::technique::analyze` → `UPDATE` to `done`/`failed` → drop the permit and
the bytes.

**The frames are held in memory, never spooled to disk.** The obvious async
implementation writes the upload to `/tmp` and picks it up later; that would
reintroduce body footage on an ephemeral container disk that `fly.toml` already
warns is not durable storage — undoing §2.10.4's retention rule as a side effect
of a concurrency change. Memory is bounded by `MAX_CONCURRENT_ANALYSES ×
MAX_TOTAL_BYTES` = 64 MiB, plus one 12 MiB decode working set.

#### 2.9.3 Polling, and orphaned `pending` rows

`GET /:id` returns `200` in **every** state, including `pending`. The client
polls with backoff: 2 s initial, ×1.5, capped at 10 s, giving up at
`ANALYSIS_TIMEOUT`.

The task is detached, so a machine stop (`auto_stop_machines = "stop"`) or a
crash orphans a `pending` row forever. Rather than a sweeper, the timeout is
**derived on read**, per SPEC-0042's rule that derived state stored is derived
state stale:

```
state = 'pending' AND now() − created_at > ANALYSIS_TIMEOUT (5 min)
   ⇒ reported as Failed { TimedOut }
```

Nothing is written. A row that completes late still reports its real outcome.

#### 2.9.4 4xx versus refusal — the rule, stated once

> **Malformed or oversize input is a `4xx`. Well-formed but under-informative
> input is a refusal *result* inside a `done`.**

- **Request bounds → 4xx.** Frame *count* and *size*, decoded dimensions,
  timestamp well-formedness, unknown lift/view, missing parts. These are
  contract violations the client can check before spending the upload.
- **Content quality → refusal.** View, confidence, framing, sampling
  regularity, camera stability, segmentability. These are things only the
  analysis can discover.

**`MIN_FRAMES` falls on the 4xx side** (`422 too_few_frames`): a client that
extracted 12 frames knows it extracted 12 frames. §2.4 explains why
`Refusal::TooFewFrames` nonetheless remains in the enum.

A refusal is **not** an error status. The analysis ran; the R-0042
`InsufficientData`-inside-a-200 precedent.

#### 2.9.5 Concurrency: a semaphore and a fast 503

`AppState` gains `analyses: Arc<Semaphore>` with `MAX_CONCURRENT_ANALYSES = 2`.
`try_acquire_owned()` is the **first** thing `POST` does after auth; on failure
it returns `503` with `Retry-After: 5` and the fixed token
`analyses_saturated` — **before reading the body**, so a saturated server costs
a rejected client nothing but a round trip.

Why this is not optional, in the numbers from `fly.toml`: the machine is **1
shared CPU** with **1 GB**, and `/health` is checked every **15 s with a 5 s
timeout**. An unbounded pile of 20-second CPU-bound inferences starves that
check and Fly restarts the machine *mid-analysis*, which turns a slow request
into a lost one. `spawn_blocking` keeps inference off the async workers but its
pool is effectively unbounded — the semaphore is the actual bound.

Why 2 and not more: ONNX inference is already serialized behind
`OnnxPoseEstimator`'s `Mutex<Session>`, so a second permit only overlaps one
analysis's decode with another's inference. The constant is sized by **memory**
(§2.9.2), not throughput.

*Follow-up, noted (not in scope):* **rate limiting**. At ~400 inferences per
request this is the most expensive endpoint in the API by an order of magnitude,
and the semaphore bounds *concurrency*, not *rate* — one client issuing requests
serially can consume the machine indefinitely and legitimately. A per-user token
bucket is warranted; it is a cross-cutting concern that should not be invented
inside this module.

### 2.10 Persistence and retention

#### 2.10.1 Required changes to `core::pose`

The series cannot be persisted as it stands. Named explicitly:

| Change | Why |
|---|---|
| `#[derive(Serialize, Deserialize)]` on `Keypoint` | no serde derives today |
| `#[derive(Serialize, Deserialize)]` + `#[serde(rename_all = "snake_case")]` on `Landmark` | ditto; the snake_case tokens become part of the stable format |
| `#[derive(Serialize, Deserialize)]` on `PoseKeypoints` | serializes as its `[Keypoint; 17]` (serde covers fixed arrays to 32) |
| `pub fn points(&self) -> &[Keypoint; 17]` | `points` is private with only a per-landmark `get`; iteration and round-trip need an accessor |
| `pub const CONFIDENCE_FLOOR: f32` (currently private) | `core::technique` must apply the *same* floor, not a second copy that can drift |

All additive; no existing behaviour changes.

#### 2.10.2 The series is a stable on-disk format

R-0045 reads these rows. `LiftSeries` therefore carries
`schema_version: u16` (v1), and the rule is stated so a future author cannot
break R-0045 by accident:

> Within a version, changes are **additive only** — new optional fields. Any
> change to the *meaning*, *unit*, *coordinate space*, or *name* of an existing
> field bumps `schema_version`, and readers must handle every version they may
> encounter. `sample_hz`, `frame_width`, `frame_height` and the coordinate-space
> contract of §2.2.2 are part of the format, not incidental.

#### 2.10.3 Do not persist a `Result`

```rust
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum AnalysisOutcome {
    Analyzed(LiftAnalysis),
    Refused(Refusal),
}
```

Serde encodes `Result` as `{"Ok": …}` / `{"Err": …}` — a Rust-internal type name
leaking into a stable on-disk format and onto the wire, where it means nothing
to a Dart client and pins the storage format to a language detail. The tagged
enum says what it means: `{"outcome": "analyzed", …}` / `{"outcome":
"refused", …}`.

#### 2.10.4 Table

```sql
CREATE TABLE lift_analyses (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    session_id     UUID REFERENCES workout_sessions (id) ON DELETE SET NULL,  -- OQ-6
    lift           TEXT NOT NULL,
    view           TEXT NOT NULL,
    state          TEXT NOT NULL,          -- 'pending' | 'done' | 'failed'
    schema_version SMALLINT NOT NULL,
    frame_width    INTEGER NOT NULL,
    frame_height   INTEGER NOT NULL,
    series         JSONB,                  -- the LiftSeries; NULL while pending
    analysis       JSONB,                  -- AnalysisOutcome; NULL while pending
    failure        TEXT,                   -- fixed token when state='failed'
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at   TIMESTAMPTZ,

    CONSTRAINT lift_analyses_state_shape CHECK (
        (state = 'pending' AND analysis IS NULL     AND failure IS NULL)
     OR (state = 'done'    AND analysis IS NOT NULL AND failure IS NULL)
     OR (state = 'failed'  AND failure IS NOT NULL)
    )
);

CREATE INDEX idx_lift_analyses_user_created
    ON lift_analyses (user_id, created_at DESC, id DESC);
```

The `CHECK` puts the §2.9.1 state machine in the schema, not only in Rust — a
`done` row with no analysis is unrepresentable.

**JSONB, not BYTEA — decided against the frame budget.** At `MAX_FRAMES` = 400
the series is 400 × 17 × 3 floats:

| Encoding | Size |
|---|---|
| JSON numerics | ≈ 330 KiB |
| packed `f32` in BYTEA | 81.6 KiB |

JSONB wins on four counts: the row is read **whole** by R-0045 and by nothing
else, so there is no per-key query BYTEA would sacrifice; at 330 KiB it TOASTs
and pglz compresses repetitive numerics substantially, closing most of the gap
in practice; a packed BYTEA is a **hand-rolled binary format** that would need
its own versioning, endianness rule and decoder — a second stable format to
maintain for ~250 KiB; and a format another requirement consumes benefits from
being inspectable. **Per-row budget: ≈ 340 KiB** (`series` ~330 KiB + `analysis`
~10 KiB), TOASTed out of line.

**The list route must not select `series`.** `GET /technique/analyses` projects
`id, session_id, lift, view, state, analysis, created_at, completed_at` only —
selecting `series` would fetch N × 330 KiB to render a list of dates. This is
the `authored_programs` precedent (`name` denormalized "so the list endpoint is
a pure SQL projection that never deserializes a program"), and §8 asserts the
projection, not just the response body. The list also carries
`LIST_PAGE_LIMIT = 50` newest-first; keyset pagination on the existing
`(user_id, created_at DESC, id DESC)` index is a follow-up.

**Uploaded frames are discarded once poses are extracted** — never written to
disk, never to R2, and (per §2.9.2) never spooled by the async path either. The
series is all the analysis needs, and it is non-identifying in a way body
footage is not. This resolves R-0044 §4's retention concern outright rather than
deferring it to the M8 legal review.

`session_id` is nullable so a clip can stand alone, but linking technique to
load is the far more valuable path (OQ-6) — and R-0045 AC3 depends on it.

#### 2.10.5 `Lift` ↔ free-text exercise names

R-0045 AC3 needs the logged load for the analysed lift, and the workout log
stores **free-text exercise names**. The bridge is explicit:

```rust
impl Lift {
    /// The `periodize::lift_key` form of this lift's canonical name.
    pub fn canonical_key(self) -> &'static str {
        match self {
            Lift::Squat    => "squat",
            Lift::Bench    => "bench press",
            Lift::Deadlift => "deadlift",
        }
    }
    /// Free-text names that resolve to this lift (already in `lift_key` form).
    pub fn aliases(self) -> &'static [&'static str];
    /// Exact lookup after `lift_key` normalization — not fuzzy matching.
    pub fn from_exercise_name(name: &str) -> Option<Self>;
}
```

`periodize::lift_key` is `name.trim().to_lowercase()`, so every
`canonical_key()` and every alias must **already** be in that form; §8 asserts
`lift_key(k) == k` for all of them, which is what stops the two from drifting.
(`lift_key` is `pub(crate)`, so the assertion lives in `core`.)

Aliases, v1 — including Spanish, for the target market:

| Lift | Aliases |
|---|---|
| `Squat` | `squat`, `back squat`, `barbell squat`, `high bar squat`, `low bar squat`, `sentadilla` |
| `Bench` | `bench press`, `bench`, `barbell bench press`, `flat bench press`, `press de banca` |
| `Deadlift` | `deadlift`, `conventional deadlift`, `barbell deadlift`, `peso muerto` |

**`sumo deadlift` is deliberately not an alias.** Its mechanics differ enough
that bar-drift-from-ankle and torso-angle are not comparable to a conventional
pull; matching it would be a silent substitution. `None` means "no logged load
found", which R-0045 AC3 already handles by reporting bodyweight-only mechanics
and saying so.

### 2.11 Calibration (AC7) — the seam is built, the capture is deferred

**Not silently dismissed, and it needs an owner amendment.**

What calibration would supply is a `Calibration` of the lifter's **own** segment
ratios: `shoulder_span / torso_length` (which collapses §2.5.3's deliberately
wide band), `thigh / shank`, `femur / torso`, and — most valuably — the
**hip-crease offset relative to the joint centre**, which is the systematic bias
that costs `SquatDepth` its severity (§2.7.3, §2.14).

The seam that makes it non-gating, present from day one:

```rust
pub fn analyze(series: &LiftSeries, calib: Option<&Calibration>) -> AnalysisOutcome;

/// Every ratio in §2.7 is read through exactly one of these.
impl Normalizers {
    pub fn population() -> Self;                 // v1: always this
    pub fn from_calibration(c: &Calibration) -> Self;
}
```

`None` ⇒ population defaults ⇒ **wider `uncertainty`**. `Some` ⇒ the person's
own ⇒ narrower. **Absence never changes a code path — only the numbers and the
interval.** That is exactly AC7's "improves precision, never blocks analysis",
and it is a property of the design, not a promise.

**Deferred in v1:** no capture UI, no `Calibration` storage, no endpoint. The
`Option` parameter and `Normalizers` ship now so adding it later is purely
additive.

> **R-0044 AC7 needs an owner amendment.** As written — *"A user may mark joints
> on a still frame once to supply limb-length ratios"* — AC7 describes a
> **capture flow** that v1 does not ship, and the qa agent will fail the
> requirement on it. Either (a) re-scope AC7 to "the analysis accepts optional
> calibration and degrades gracefully in its absence", which this spec fully
> satisfies, or (b) move the capture flow to a follow-on requirement. This must
> be decided before QA sign-off, not during it.

### 2.12 `FakePoseEstimator` — required additions

Today's fake has **one canned result and no call counter**, which makes it
unable to support two things §8 depends on: asserting that inference did **not**
run when bounds rejected a request, and driving a multi-frame series where each
frame has a *different* pose (every segmentation test).

```rust
impl FakePoseEstimator {
    pub fn returning(keypoints: PoseKeypoints) -> Self;   // unchanged
    pub fn failing(error: PoseError) -> Self;             // unchanged
    /// The i-th call returns the i-th pose. Past the end: `Err(Inference)`.
    pub fn scripted(poses: Vec<PoseKeypoints>) -> Self;   // new
    /// How many times `estimate` has been called.
    pub fn calls(&self) -> usize;                         // new
}
```

- `calls: AtomicUsize` — `estimate` takes `&self`, so an atomic is the right
  tool; `Ordering::SeqCst` (the test needs monotonicity, not performance).
- **Past the end of a script is an error, not a repeat of the last pose.**
  Padding would let a test that scripts 10 poses silently pass while the code
  requested 40 — the failure would be invisible in exactly the tests that exist
  to catch it.
- Internally `result` becomes a private `enum { Fixed(Result<…>),
  Scripted(Vec<PoseKeypoints>) }`. `Default` is unchanged.
- **Practical wrinkle, worth stating:** `AppState` holds `Arc<dyn
  PoseEstimator>`, which does not downcast conveniently. Tests keep a second
  `Arc<FakePoseEstimator>` and pass `Arc::clone(&fake) as Arc<dyn
  PoseEstimator>`, reading `calls()` through their own handle.
- Scripted fixtures **must be authored in the isotropic canvas space** of
  §2.2.2, or every synthetic test measures a different geometry from production.
  This goes in the fake's doc comment.

### 2.13 Module factoring

`core::technique` is not one file. Dependencies point inward; per-lift modules
know nothing about each other:

```
core/src/technique/
  mod.rs        the pipeline, the public types, the refusal ordering (§2.4)
  view.rs       the four cues, thresholds, median classification, stability (§2.5)
  segment.rs    quiet stance, centred smoothing, prominence, reps, tempo (§2.6)
  geometry.rs   near-side resolution, angles, normalizers, uncertainty (§2.7.1-2, §2.8)
  faults.rs     Fault, label(), cue() — the AC14-reviewable copy (§2.7.6)
  squat.rs
  bench.rs
  deadlift.rs   per-lift findings only; each depends on geometry.rs, nothing else
```

Public surface: `analyze`, the domain types, `Fault::label`/`cue`,
`Lift::canonical_key`/`aliases`/`from_exercise_name`, and the bound constants.
Everything else is `pub(crate)` or private. `FindingSeverity` is exported from
`technique` and **not** re-exported at the crate root (§2.8).

### 2.14 Thresholds (OQ-4) — still deferred, and what the UI does about it

**No fault has a threshold in v1 — including depth.** `severity` is `None` for
every finding.

**The rejected draft's claim that "depth is well-defined" was half right and the
wrong half was load-bearing.** The *threshold* is well-defined: hip crease level
with the top of the knee. The *measurement's relationship to that threshold* is
not — MoveNet gives the joint centre, not the crease, with a systematic 4–7°
offset in the optimistic direction (§2.7.3), and its random error is comparable
to the decision margin. A crisp pass/fail on a biased measurement is a false
verdict, which is AC10c's entire point. Depth therefore ships as a **measured
value with an uncertainty and no severity** until either AC7 calibration (§2.11)
pins the offset per person, or a labelled validation set pins it in the
population.

**The UI contract for `severity: None`** — specified here so mobile cannot
invent one:

- Render the value and its uncertainty in the **neutral** style: the typography
  of an `Ok` finding, but **no colour, no icon, and no pass/fail word**.
- Show the fixed caption: **"No accepted threshold — the number is reported
  without a verdict."**
- Do **not** sort severity-less findings above or below others, and do **not**
  count them toward any summary badge or score.

Without this, mobile picks amber "because it looks unresolved" and the
judgement the deferral exists to withhold is reintroduced by a stylesheet.

**Resolution path for OQ-4:** each threshold must cite a source — a federation
depth standard, or a labelled validation set of clips with expert-scored depth,
knee travel and bar drift. Numbers are not to be invented; that is what the
severity-less path is for.

### 2.15 The framing guide (AC4)

Shown **before** recording, per lift, and restated in any `WrongView` refusal.

| Lift | View | Phone position | Frame must contain |
|---|---|---|---|
| **Squat** (depth, torso) | **Side**, perpendicular to the bar | Hip height, ~3 m away, landscape | Whole body incl. ankles, plus headroom at the top |
| **Squat** (knee travel) | **Front**, straight on | Hip height, ~3 m, landscape | Both ankles and both knees |
| **Bench** | **Side**, perpendicular to the bench | Bench height, ~2.5 m, landscape | Bar, both wrists, shoulder, hip |
| **Deadlift** | **Side**, perpendicular to the bar | Knee height, ~3 m, landscape | Bar, whole body, both ankles |

> Changed from the rejected draft: the deadlift row said *"feet flat in frame"*,
> which implied the feet were an input. **They are not** — COCO-17 has no foot
> landmark (§2.7.5). The requirement is that the **ankles** are in frame and
> confidently visible, since drift is measured from the near-side ankle.

Universal rules, stated as such: **static phone** (tripod or propped — never
handheld), **whole body in frame for every rep**, **even lighting from the
front**, **plain background**, **one person in frame**, and **film one set, not
a montage**. Squat depth and knee travel need *different* views: two clips, not
a compromised 45° that measures neither well — and which §2.5.3 refuses outright.

The mobile capture screen renders a translucent framing overlay per lift and
requires the user to confirm the view before recording. It also states, before
the first recording, the §2.7.5 list of what cannot be measured — most
importantly that **this is not a back-rounding check**.

## 3. Code outline

```rust
// core/src/technique/mod.rs — pure, no I/O, no model, no clock
pub fn analyze(series: &LiftSeries, calib: Option<&Calibration>) -> AnalysisOutcome {
    match run(series, calib) {
        Ok(analysis) => AnalysisOutcome::Analyzed(analysis),
        Err(refusal) => AnalysisOutcome::Refused(refusal),
    }
}

fn run(series: &LiftSeries, calib: Option<&Calibration>) -> Result<LiftAnalysis, Refusal> {
    check_frame_count(series)?;                        // §2.4 step 1
    check_timestamps(series)?;                         //      step 2
    check_framing(series)?;                            //      step 3
    check_confidence(series)?;                         //      step 4

    let norms = Normalizers::from(calib);              // §2.11 — never a code path
    // Near side is chosen from keypoint scores alone, so it needs no view;
    // `Front` simply ignores it (§2.7.1).
    let side  = geometry::near_side(series);           // §2.7.1 — once, whole series

    let start = segment::quiet_stance(series, side)?;     // §2.6.1 — step 5
    view::classify_and_check_stability(series, start)?;   // §2.5.4 — step 6
    segment::check_camera_static(series, side, start)?;   // §2.6.4 — step 7
    let reps = segment::reps(series, side, start)?;       // §2.6.2-5 — step 8

    let measurements = match series.lift {
        Lift::Squat    => squat::measure(series, side, &reps, &norms),
        Lift::Bench    => bench::measure(series, side, &reps, &norms),
        Lift::Deadlift => deadlift::measure(series, side, &reps, &norms),
    };

    Ok(LiftAnalysis {
        view: series.view,
        rep_count: reps.len() as u32,
        tempo: segment::tempo(series, &reps),
        measurements,
        not_measurable: NOT_MEASURABLE,                // §2.7.5, carried to the UI
    })
}
```

The api layer's only job is: bounds (streaming) → permit → `202` → spawn →
`PoseEstimator` per frame → build `LiftSeries` → call `analyze` → store.
**No geometry in the api crate.**

## 4. Non-goals

Per R-0044 §4: no on-device inference, no live coaching, no 3D or multi-camera,
no lifts beyond the three, no auto-logging of weight or reps, no injury-risk
score, and no retention of raw frames (§2.10.4). Additionally out of scope here
and noted as follow-ups: rate limiting (§2.9.5), Spanish fault copy (§2.7.6),
keyset pagination (§2.10.4), calibration capture (§2.11), and the pre-existing
photo-path decode bound (§2.3.2).

## 5. Open questions

### Resolved here

- **OQ-1:** bounds table + the compile-time invariant, §2.3 / §2.3.1.
- **OQ-2:** quiet stance → centred smoothing → prominence → excursion from the
  reference, with touch-and-go handled, §2.6.
- **OQ-3:** `shoulder_span / torso_length` with hip corroboration and ear/nose as
  an orthogonal cue; two thresholds and a wide refusal band; median
  classification plus a stability check, §2.5. *(The rejected draft's
  shoulder÷hip ratio was mathematically incapable of the job — §2.5.)*
- **OQ-5:** **asynchronous** — 202 + poll, three-state machine, §2.9. *(Reversed
  from the rejected draft; now AC0.)*
- **OQ-6:** nullable `session_id`, §2.10.4 — and R-0045 AC3 depends on it, so
  the mobile flow should default to attaching.
- **OQ-7:** frames discarded after extraction, never spooled, §2.9.2 / §2.10.4.

### Still open

- **OQ-4 — thresholds.** Deferred with an explicit contract for the
  severity-less state (§2.14), including the correction that depth's *threshold*
  is well-defined but its *measurement's bias against that threshold* is not.
  Resolution needs a cited source or a labelled validation set.
- **The §2.5 and §2.6 constants** (`VIEW_*`, `QUIET_TOL`, `MIN_REP_*`,
  `MAX_STATIC_DRIFT`, `KEYPOINT_SIGMA_FRACTION`) are **initial values with
  stated derivations, not measurements.** They must be fitted against a small
  labelled sample of framing and rep clips before launch. They are honest about
  their provenance in the same way OQ-4 is: unlike a fault threshold, a wrong
  value here produces a *refusal*, not a wrong verdict — the safe direction.
- **minSdk 27** (§2.1.1) — owner product decision.
- **R-0044 AC7** (§2.11) — needs an owner amendment before QA sign-off.

## 6. Acceptance criteria

| AC | Where |
|---|---|
| AC0 async | §2.9, §2.9.1–2.9.3 |
| AC1 server-side inference | §2.1 |
| AC2 bounds before decoding | §2.3, §2.3.1, §2.3.2, §2.3.3 |
| AC3 sampled at a named rate | §2.3 (`SAMPLE_HZ`), §2.1.3, §2.2.1 |
| AC4 per-lift camera requirement, view stated | §2.15, `LiftAnalysis.view` |
| AC5 the numeric series (+ tempo) | §2.2, §2.7.4 |
| AC6 rep segmentation, per-rep assessment | §2.6, `Finding.rep` |
| AC7 calibration optional | §2.11 — **and the amendment flagged there** |
| AC8 squat: depth, knee travel, torso-lean change | §2.7.3 |
| AC9 bench: bar path, forearm angle at touch, touch consistency | §2.7.3 |
| AC10 deadlift: hip rise, torso-lean change, drift from ankle | §2.7.3 |
| AC10b say what cannot be measured | §2.7.5, `Unmeasurable`, `not_measurable`, §2.15 |
| AC10c uncertainty; `Borderline` = interval straddles threshold | §2.8 |
| AC11 measured, not adjectival | `Finding`, §2.7.6 |
| AC12 refuse rather than mis-score | §2.4 ordering, §2.6 `Refusal`, §2.9.4 |
| AC13 uncertainty to the surface, view always stated | §2.8, §2.14, `LiftAnalysis.view` |
| AC14 no medical or injury claims | §2.7.6 — exact strings, asserted in §8 |
| AC15 pure core | §3, §2.13 |
| AC16 ownership, 404 never 403, JWT | §2.9 |
| AC17 tests | §8 |

## 7. `Refusal` — the closed set

Every variant carries its evidence, so the user is told *why*, with numbers.

```rust
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum Refusal {
    TooFewFrames      { have: usize, required: usize },
    IrregularSampling { median_gap_ms: f64, expected_gap_ms: f64, max_gap_ms: u32 },
    OutOfFrame        { landmark: Landmark, missing_frames: u32, total_frames: u32 },
    LowConfidence     { mean: f64, required: f64 },
    WrongView         { expected: CameraView, looks_like: ViewClass,
                        shoulder_ratio: f64, hip_ratio: f64, ear_score_ratio: f64 },
    UnstableView      { side: u32, front: u32, indeterminate: u32 },
    CameraMoved       { landmark: Landmark, drift: f64, allowed: f64 },
    NoStableStart,
    NoRepsDetected,
}

#[serde(rename_all = "snake_case")]
pub enum ViewClass { Side, Front, Indeterminate }
```

`IrregularSampling` covers all three sampling failures — too slow, too fast,
jittery — and its three fields say which.

## 8. Test plan

**Core (no video, no database, no model)** — synthetic `LiftSeries` built from
generated joint positions in the §2.2.2 canvas space.

*Segmentation — the cases the rejected draft got wrong:*

1. **Walkout.** A squat series with 3 s of lateral ankle and hip motion before
   rep 1 → the walkout is excluded, `rep_count` is correct, and the standing
   reference comes from the quiet window, **not** frame 0.
2. **Paused rep.** A 2 s bottom hold with ±4 % jitter → **1** rep, not 3.
3. **Mid-ascent stall (grinder).** A concentric with a plateau and a small
   reversal → **1** rep.
4. **Touch-and-go deadlift.** Reps 2..N reference their own start, not rep 1's
   setup; assert per-rep excursions are all measured.
5. **Sub-excursion wobble.** A weight shift while re-racking → not a rep.
6. **Centred vs trailing.** Assert the detected extremum index equals the
   ground-truth index (a trailing average would offset it by `w`).

*View and stability:*

7. **Yaw sweep.** Synthetic poses at yaw 0°→90° in 5° steps → `view_ratio`
   monotone decreasing; Front below ~28°; Side above ~69°; **everything between
   refused**.
8. **The old rule's failure, pinned.** Assert `shoulder_span / hip_span` is
   ~constant across the same sweep — the regression test that stops anyone
   reintroducing it.
9. **UnstableView.** First half Side, second half Front → refusal.
10. **CameraMoved.** Ankles drifting 30 % of shank length across the
    segmentation window → refusal; the *same* drift confined to the pre-quiet
    walkout → **no** refusal.

*Timestamps:*

11. Uniform 5 Hz → `IrregularSampling`; uniform 10 Hz with one 800 ms gap →
    `IrregularSampling`; non-monotonic / `frames[0].t_ms != 0` → 4xx at the api
    edge (§2.9.4), never a refusal.

*Measurements:*

12. Clean squat below parallel; high squat; a series straddling the (future)
    threshold → `Borderline` by the **interval** rule, with `uncertainty`
    asserted non-zero and including the crease offset.
13. Knee travel, Front view → per-side findings; the same series declared `Side`
    → `WrongView`.
14. Bench bar-path drift → normalized value; **single-rep bench → touch-point
    consistency is `Unavailable { SingleRep }`**, never 0.0.
15. `BenchForearmAngleAtTouch` at the touch frame, signed both ways.
16. **Deadlift hip-rise as a ratio** — hips-shoot-up → ≫ 1; clean pull → ≈ 1;
    **bar never breaks the floor → `Unavailable { BarDidNotBreak }`**, never a
    division by ~0. Plus the degeneracy pin: assert the *correlation* of the same
    near-constant bar signal is meaningless, so nobody restores it.
17. `DeadliftTorsoAngleChange` — start vs frame-of-max-change; assert the frame
    index is reported and is **not** the start frame.
18. `DeadliftBarDriftFromAnkle` normalized by shank length; assert no code path
    references a foot landmark or a foot-length normalizer.
19. **Occluded far side is not averaged in.** A Side series with the far hip
    scored 0.05 and displaced 20 % of torso length → the depth value equals the
    near-side-only computation **exactly**, and moving the far hip changes
    **nothing**.
20. Per-rep tempo in seconds; a dropped last deadlift → `eccentric_s: None`.

*Properties:*

21. **Determinism** — same series twice → identical `AnalysisOutcome`.
22. **Noise stability** — jitter of **±3–5 % of torso length** (MoveNet's real
    keypoint error, expressed as a fraction of the lifter's own body, never in
    pixels) must not move any finding's `value` by more than its reported
    `uncertainty`, and must not change `rep_count`.
23. **Bounds invariant** — the §2.3.1 relation, as a runtime `#[test]` beside
    the `const` assertions.
24. **Fault copy** — every `Fault`'s `label()` and `cue()` asserted against the
    exact §2.7.6 strings; plus a scan asserting no string contains `injur`,
    `pain`, `damage`, `danger`, `risk`, or `valgus`. This is what makes AC14
    reviewable.
25. **`Lift` keys** — `lift_key(canonical_key()) == canonical_key()` and the
    same for every alias; `from_exercise_name` round-trips; `"sumo deadlift"` →
    `None`.
26. Every `Refusal` variant reached deliberately.
27. Total function — `analyze` on an empty series, a 1-frame series, and an
    all-zero-confidence series returns a typed outcome, never a panic.

**Integration (`#[sqlx::test]`)** — the existing `FakePoseEstimator`, extended
per §2.12, supplies scripted keypoints, so **no fixture video is required
anywhere**.

28. Auth on all four routes, including a malformed id → 401.
29. `POST` → **202** + `{id, state:"pending"}`; immediate `GET /:id` →
    `pending`; after the task completes → `done` with the outcome.
30. **Over-`MAX_FRAMES` → 4xx and `fake.calls() == 0`** — no inference ran.
31. **Over-`MAX_FRAME_BYTES` → 4xx**, streaming: assert the request aborts
    without reading the remaining parts.
32. **Oversize decoded dimensions** — a small JPEG declaring 8000×8000 → 4xx
    with `fake.calls() == 0` (the header check ran, the decode did not).
33. Under-`MIN_FRAMES` → 4xx (§2.9.4), not a refusal.
34. A refusal stored and returned as `done { outcome: refused }`, HTTP 200.
35. An estimator error → `failed`, **not** `refused` (§2.9.1).
36. **Saturated semaphore → 503** with `Retry-After`, body unread,
    `fake.calls()` unchanged.
37. A `pending` row aged past `ANALYSIS_TIMEOUT` → reported `failed { timed_out }`
    **with no write** (assert the row is still `pending` afterwards).
38. **List excludes `series`** — assert the response body has no `series` key
    *and* that the SQL projection does not name it.
39. Ownership 404 on `GET`/`DELETE`; `DELETE` → 204 then 404.
40. **Round-trip** — a stored `LiftSeries` deserializes equal, with
    `schema_version == 1` and `frame_width`/`frame_height` preserved (the
    R-0045 contract, §2.10.2).

## 9. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-25 | Client extracts frames; server runs inference | Avoids a video decoder (large native dep, CVE surface) in the container and keeps AC1's server-side inference intact. Frame extraction is not inference. |
| 2026-08-25 | Raw frames are discarded after pose extraction, and never spooled | The series is all the analysis needs and is far less sensitive than body footage; resolves the retention question instead of deferring it. The async path must not reintroduce a disk copy. |
| 2026-08-25 | Self-scaling normalizers (segment-length ratios) | Pixel constants break with camera distance and body size. |
| 2026-08-25 | Squat needs two clips for depth and knee travel | Perpendicular views; a 45° compromise measures neither, and §2.5.3 now refuses it. |
| **2026-08-25** | **Analysis is asynchronous: 202 + poll** | *Owner decision (AC0).* 400 serialized inferences on 1 shared CPU is tens of seconds against a ~5 s client timeout. Reverses the rejected draft's OQ-5. |
| **2026-08-25** | **`Failed` and `Done{Refused}` are separate states** | A server fault must never read to the user as "your video was bad" — AC12's dishonesty, inverted. |
| **2026-08-25** | **View from `shoulder_span / torso_length`, corroborated, with a wide refusal band** | The shoulder÷hip ratio cancels the very cosine it was meant to measure and is 0/0 side-on. Euclidean torso is yaw-invariant and survives a pitched deadlift setup. |
| **2026-08-25** | **Prominence-based peaks + quiet-stance start + excursion-from-reference** | Bare reversal detection counts a paused squat as 3 reps, a grinder as 2, and the walkout as several. |
| **2026-08-25** | **Centred moving average, window from `SAMPLE_HZ`** | A trailing average phase-shifts every turning point, so depth and touch angle are read at the wrong instant. |
| **2026-08-25** | **Hip-rise as a ratio, not a correlation** | Correlation against a near-constant bar signal is degenerate exactly in the case it must detect. |
| **2026-08-25** | **`TorsoAngleChange`, and back rounding declared unmeasurable** | *AC8/AC10/AC10b.* COCO-17 has no spine landmark; and for a deadlift the setup **is** the bottom, so "setup vs bottom" was identically zero. |
| **2026-08-25** | **Bar drift from the ankle, normalized by shank; ankle bias disclosed** | *AC10.* COCO-17 ends at the ankle — mid-foot and foot length do not exist. The bias over-flags, which is the safe direction, and it is stated. |
| **2026-08-25** | **`ForearmAngleAtTouch` replaces elbow flare** | *AC9.* Flare is a frontal-plane angle; from the prescribed side view the upper arm is along the lens axis and the far arm is occluded. |
| **2026-08-25** | **`uncertainty` per finding; `Borderline` = interval straddles threshold** | *AC10c.* "Close to" is a second arbitrary constant; a straddling interval is a statement about what the measurement supports. |
| **2026-08-25** | **No thresholds in v1 — including depth** | The depth *threshold* is well-defined; its *measurement's* 4–7° optimistic bias against it is not. A crisp pass/fail on a biased measurement is a false verdict. |
| **2026-08-25** | **Near side resolved once per series; far side never averaged in** | Per-frame selection injects step discontinuities; a near/far midpoint is a fabricated centre-line whose error peaks at the bottom of a squat. |
| **2026-08-25** | **Geometry in frame-index space; `t_ms` only validates and converts tempo** | Otherwise every measurement depends on the client's clock. A bad clock is refused, never resampled. |
| **2026-08-25** | **JSONB, not packed BYTEA** | The row is read whole by R-0045 and nothing else; BYTEA would be a second hand-rolled stable format for ~250 KiB, and TOAST compression closes most of the gap. |
| **2026-08-25** | **`AnalysisOutcome` tagged enum, not a serialized `Result`** | A Rust-internal type name has no business in a stable on-disk format or on a Dart client's wire. |
| **2026-08-25** | **Decoded-dimension bound before decode** | A 2 MiB JPEG can declare dimensions that decode to gigabytes; the container has 1 GiB. A byte bound does not bound a decode. |
| **2026-08-25** | **Semaphore with a fast 503, sized by memory** | 1 shared CPU with a 5 s `/health` timeout every 15 s; an unbounded pile of 20 s inferences gets the machine restarted mid-analysis. |
| **2026-08-25** | **Calibration seam built, capture deferred, AC7 amendment flagged** | AC7 as written describes a capture flow v1 does not ship; the qa agent would fail the requirement. The `Option` parameter makes the addition purely additive. |
| **2026-08-25** | **`PoseEstimator` must emit isotropic coordinates — stated as an invariant** | The letterbox in `preprocess` is the only reason every angle and ratio here is correct, and nothing documented it. |

## 10. Architect review — REJECT (2026-08-25), all changes applied

27 findings plus two owner decisions. Every one is folded in above.

**Owner decisions (non-negotiable):**

| | Decision | Landed |
|---|---|---|
| A | Analysis is **asynchronous** — 202 + poll, three-state machine | §2.9, §2.9.1–2.9.3; R-0044 AC0; §9 |
| B | Measurements amended to what COCO-17 can produce; a plain "not measurable" section | §2.7.3, **§2.7.5**, §2.15 |

**Blocking fixes:**

| # | Finding | Landed |
|---|---|---|
| 1 | Deadlift drift from the **ankle**, normalized by **shank**, ankle bias disclosed; "foot length" and "feet flat" removed | §2.7.3, §2.7.5, §2.15 |
| 2 | `TorsoAngleChange`; deadlift defined start-frame vs frame-of-max-change ("setup vs bottom" was incoherent — the setup *is* the bottom) | §2.7.3 |
| 3 | Bench: `ForearmAngleAtTouch` replaces elbow flare | §2.7.3 |
| 4 | View detection rebuilt: `shoulder_span / torso_length` (Euclidean, yaw-invariant), hip corroboration, ear/nose orthogonal cue, two thresholds with a wide band, median classification, instability is a refusal | §2.5.1–2.5.4, test 8 pins the old rule's failure |
| 5 | Segmentation: quiet stance (walkout/rack-out skipped), prominence, excursion from the reference, **centred** moving average, touch-and-go | §2.6.1–2.6.5 |
| 6 | Bounds arithmetic fixed; invariant asserted at compile time; `DefaultBodyLimit`; **decoded-dimension bound before decode**; streaming multipart; photo path's shared hole noted | §2.3, §2.3.1, §2.3.2, §2.3.3 |
| 7 | `core::pose` serde + accessor named; stable format + `schema_version`; `AnalysisOutcome` instead of a `Result`; JSONB decided against the budget; list route excludes `series` | §2.10.1–2.10.4 |
| 8 | Client extraction named (`AVAssetImageGenerator` / `getScaledFrameAtTime`), minSdk 27 flagged, ≤480 px output, sub-`SAMPLE_HZ` fallback, `ffmpeg_kit_flutter` retirement noted, **"50×" corrected to ~3–7×** | §2.1, §2.1.1–2.1.3 |
| 9 | Geometry in frame-index space; `t_ms` relative to clip start, `u32` rationale, strict validation, `Refusal::IrregularSampling` | §2.2.1, §7 |
| 10 | `uncertainty: f64` on `Finding`; `Borderline` redefined as a straddling interval; hip joint-centre vs crease offset disclosed | §2.8, §2.7.3 |
| 11 | Near-side resolved once per series for Side; both sides for Front; far side never averaged in | §2.7.1, test 19 |
| 12 | Hip-rise as a **ratio** over the early pull, with a typed `BarDidNotBreak` guard; correlation rejected with the reason | §2.7.3, §2.8.1, test 16 |
| 13 | Per-rep tempo added to `LiftAnalysis` | §2.7.4 |
| 14 | The 4xx-vs-refusal rule stated once; `MIN_FRAMES` placed on the 4xx side; the ambiguous timestamp case split explicitly | §2.9.4, §2.2.1, §2.4 |
| 15 | `Lift::canonical_key()` matching `periodize::lift_key`, plus aliases and `from_exercise_name` | §2.10.5, test 25 |
| 16 | `FakePoseEstimator`: `AtomicUsize` counter + `calls()`, scripted `Vec<PoseKeypoints>`, fail-past-end, isotropic-fixture contract | §2.12 |
| 17 | AC7 designed as a seam, capture deferred, **owner amendment explicitly flagged** so qa does not fail silently; noted as the natural fix for depth's bias | §2.11, §2.14 |
| 18 | `Refusal::CameraMoved` from landmark variance — ankles for squat/deadlift, hips for bench, over the post-walkout window only | §2.6.4, test 10 |
| 19 | Touch-point variance normalized by forearm length; **no finding at one rep** | §2.7.3, §2.8.1, test 14 |
| 20 | Source frame width/height stored once per series | §2.2, §2.2.2, §2.10.4 |
| 21 | `Fault` variants **and their exact user-facing strings** enumerated; "valgus" → `KneeTravelInward` | §2.7.6, test 24 |
| 22 | Semaphore around whole analyses with a fast 503, justified from `fly.toml`'s real health-check numbers | §2.9.5, test 36 |
| 23 | The R-0045 note: 10 Hz is too coarse for double differentiation, pre-resolving its OQ-2 toward quasi-static | §2.2.3 |
| 24 | Rate limiting noted as a follow-up, with why the semaphore is not it | §2.9.5, §4 |
| 25 | Tests added: walkout, paused rep, mid-ascent stall, irregular timestamps, oversize dimensions, occluded far side, list excludes `series`; jitter restated as ±3–5 % of torso length | §8, tests 1–3, 10–11, 19, 22, 32, 38 |
| 26 | `core::technique` factored into `view` / `segment` / `geometry` / `faults` + per-lift modules | §2.13 |
| 27 | OQ-4 kept deferred **with a UI contract for a severity-less finding**; the "depth is well-defined" claim corrected | §2.14 |

**Kept verbatim, as the review endorsed:** §2.1's client-extracts-frames
rationale (minus the false size claim, per finding 8), pure-core analysis with a
thin api edge (§3, §2.13), refusal-before-verdict ordering (§2.4),
frames-discarded-after-extraction retention (§2.10.4), and the framing guide
(§2.15 — with the "feet flat in frame" implication removed per finding 1).

**Found while applying the above, and resolved here:** the coordinate space of
`PoseKeypoints` is the model's **letterboxed** canvas, which is isotropic only
because `preprocess` scales uniformly. Every angle and ratio in §2.7 depends on
that, and nothing documented it — §2.2.2 now states it as an invariant of the
`PoseEstimator` seam and extends it to the test fake's fixtures.

## Changelog

- _2026-08-25 — created (Draft)._
- _2026-08-25 — architect review REJECT; reworked in full against the amended
  R-0044 (AC0, AC8, AC9, AC10, AC10b, AC10c). All 27 findings and both owner
  decisions applied (§10). Open for the owner: minSdk 27 (§2.1.1) and the AC7
  amendment (§2.11)._
