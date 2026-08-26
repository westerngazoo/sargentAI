# SPEC-0044 — Lift technique video analysis (`core::technique` + `api::technique`)

- **Status:** Draft
- **Realizes:** R-0044
- **Author:** Claude (main session)
- **Created:** 2026-08-25
- **Depends on:** SPEC-0013 (`PoseEstimator`, `PoseKeypoints`, `Landmark`,
  MoveNet/ONNX — all shipped), SPEC-0041 (owner-scoped persistence pattern),
  SPEC-0042 (typed-refusal precedent).
- **Module(s):** `backend/crates/core/src/technique/` (new — pure);
  `backend/crates/api/src/technique/{mod,handlers}.rs` (new);
  `backend/migrations/00011_lift_analyses.sql` (new);
  `mobile/lib/src/technique/**` (new — capture, framing guide, results).

## 1. Motivation

Realizes [R-0044](../requirements/0044-lift-technique-video-analysis.md). The
pose infrastructure exists and is idle: R-0013 shipped COCO-17 `Landmark`,
`PoseKeypoints`, a `PoseEstimator` trait with a test fake, and MoveNet embedded
as ONNX — all for still photos. **Video is that pipeline with a time axis.**

## 2. Design

### 2.1 Frames, not video — the decision that shapes everything

`PoseEstimator::estimate` takes **encoded image bytes**
(`api/src/pose/mod.rs:36-44`), not video. Rather than add a video decoder to
the API container, **the client extracts sampled frames and uploads those**:

```
phone: record → sample N frames as JPEG → upload multipart
server: PoseEstimator per frame → PoseKeypoints series → core::technique
```

Consequences, all favourable:

- **No `ffmpeg` in the container.** A video decoder is a large native
  dependency and a well-known source of parser CVEs; the image path is already
  built, bounded, and tested.
- **Upload shrinks by ~50×.** Ten 1080p JPEGs ≈ 2 MB versus a 30-second
  1080p clip ≈ 60 MB — decisive on the target market's mobile data.
- **AC1 holds.** Frame extraction is not inference; the model still runs
  server-side. This is exactly the split `project-specifics.md` asks for: the
  phone captures, the server thinks.
- **The clip need never leave the phone** (R-0044 §4's retention concern
  dissolves — see §2.8).

### 2.2 The vector structure (AC5)

```rust
/// One sampled frame: when it was taken and where the joints were.
pub struct PoseFrame { pub t_ms: u32, pub pose: PoseKeypoints }

/// A lift's full sampled series — the "video converted to its basics".
pub struct LiftSeries { pub lift: Lift, pub view: CameraView, pub frames: Vec<PoseFrame> }

#[serde(rename_all = "snake_case")]
pub enum Lift { Squat, Bench, Deadlift }

#[serde(rename_all = "snake_case")]
pub enum CameraView { Side, Front }
```

A 30-second set at `SAMPLE_HZ = 10` is 300 frames × 17 joints — a few thousand
floats. Nothing downstream ever sees a pixel.

### 2.3 Bounds (AC2) — enforced before any work

| Bound | Value | Why |
|---|---|---|
| `MAX_FRAMES` | 400 | 40 s at 10 Hz; each frame is one inference |
| `MIN_FRAMES` | 20 | below this no rep can be segmented |
| `MAX_FRAME_BYTES` | 2 MiB | reuses `photo::MAX_BYTES` discipline |
| `MAX_TOTAL_BYTES` | 24 MiB | request-level ceiling |
| `SAMPLE_HZ` | 10 | a rep lasts 2–4 s; 10 Hz resolves the turnaround |

Frame count is checked **before** the first inference — N inferences is the
expansion an attacker controls, exactly the R-0041 `sets` lesson.

### 2.4 View validation and refusal (AC12, AC13)

Refusal is computed **before** any verdict:

```rust
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum Refusal {
    WrongView { expected: CameraView, looks_like: CameraView },
    LowConfidence { mean: f64, required: f64 },
    OutOfFrame { landmark: Landmark, frames: u32 },
    NoRepsDetected,
    TooFewFrames { have: usize, required: usize },
}
```

**View detection (OQ-3, resolved):** the ratio of shoulder-width to hip-width in
image space is near its anatomical value from the front and collapses toward
zero side-on, because the shoulders foreshorten. Threshold `VIEW_RATIO` with a
band; inside the band → `WrongView` rather than a guess. Ankle and hip must be
visible in ≥ 90 % of frames or `OutOfFrame`.

**Confidence floor:** mean keypoint confidence over the joints each lift
actually uses (not all 17) must clear `MIN_CONFIDENCE`. MoveNet already reports
per-keypoint confidence.

This is the strictest application of the codebase's honesty precedent: **a
confident wrong verdict causes the injury the feature exists to prevent.**

### 2.5 Rep segmentation (AC6, OQ-2 resolved)

Track the **hip midpoint's vertical position** for squat and deadlift, the
**wrist midpoint's** for bench. Smooth with a small moving average (camera
shake), then find turning points where the direction of travel reverses and the
excursion exceeds `MIN_REP_EXCURSION` (a fraction of the lifter's own
hip-to-shoulder length — self-scaling, so no pixel constants).

Each rep is `(start, bottom, end)` frame indices. Fewer than one → `NoRepsDetected`.

### 2.6 Per-lift analysis (AC8–AC11)

Every finding carries a measured value, its unit, the rep, and a confidence.

**Squat — side:**
- *Depth*: signed angle of the hip→knee segment at the bottom frame; hip crease
  below knee is the reference. Reported in degrees relative to parallel.
- *Torso angle*: shoulder→hip vector versus vertical, at the bottom.

**Squat — front:**
- *Knee valgus*: signed horizontal offset of the knee from the ankle→hip line,
  normalized by the lifter's own shank length.

**Bench — side:**
- *Bar-path deviation*: horizontal excursion of the wrist midpoint over the rep,
  normalized by forearm length.
- *Elbow flare*: elbow angle relative to the torso line at the touch frame.
- *Touch-point consistency*: variance of the wrist position at the bottom across
  reps.

**Deadlift — side:**
- *Hip rise before bar*: correlation of hip-vertical and wrist-vertical velocity
  over the first `EARLY_PULL_FRACTION` of the rep — hips rising while the bar
  does not is the fault.
- *Spinal flexion change*: shoulder→hip angle at setup versus at the bottom.
- *Bar drift*: horizontal distance of the wrist from the mid-foot, peak over the
  rep, normalized by foot length.

```rust
pub struct Finding {
    pub fault: Fault,          // typed, not free text
    pub rep: u32,
    pub value: f64,
    pub unit: Unit,            // Degrees | NormalizedLength | Ratio
    pub severity: Severity,    // Ok | Borderline | Flagged
    pub confidence: f64,
}
```

`Borderline` exists so a value near a threshold is never reported as a crisp
pass/fail (AC13).

### 2.7 The framing guide (AC4)

Shown **before** recording, per lift, and restated in any `WrongView` refusal.

| Lift | View | Phone position | Frame must contain |
|---|---|---|---|
| **Squat** (depth, torso) | **Side**, perpendicular to the bar | Hip height, ~3 m away, landscape | Whole body incl. feet, plus headroom at the top |
| **Squat** (valgus) | **Front**, straight on | Hip height, ~3 m, landscape | Both feet and both knees |
| **Bench** | **Side**, perpendicular to the bench | Bench height, ~2.5 m, landscape | Bar, both wrists, shoulder, hip |
| **Deadlift** | **Side**, perpendicular to the bar | Knee height, ~3 m, landscape | Bar, whole body, feet flat in frame |

Universal rules, stated as such: **static phone** (tripod or propped — never
handheld), **whole body in frame for every rep**, **even lighting from the
front**, **plain background**, **one person in frame**, and **film one set, not
a montage**. Squat depth and valgus need *different* views: two clips, not a
compromised 45° that measures neither well.

The mobile capture screen renders a translucent framing overlay per lift and
requires the user to confirm the view before recording.

### 2.8 Persistence and retention

`00011_lift_analyses.sql` stores the **analysis and the keypoint series**, not
the frames:

```sql
CREATE TABLE lift_analyses (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    session_id UUID REFERENCES workout_sessions (id) ON DELETE SET NULL,  -- OQ-6
    lift       TEXT NOT NULL,
    view       TEXT NOT NULL,
    series     JSONB NOT NULL,   -- the LiftSeries: the "basics"
    analysis   JSONB NOT NULL,   -- Result<LiftAnalysis, Refusal>
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_lift_analyses_user_created
    ON lift_analyses (user_id, created_at DESC, id DESC);
```

**Uploaded frames are discarded once poses are extracted** — never written to
disk, never to R2. The series is all the analysis needs, and it is
non-identifying in a way body footage is not. This resolves R-0044 §4's
retention concern outright rather than deferring it to the M8 legal review.

`session_id` is nullable so a clip can stand alone, but linking technique to
load is the far more valuable path (OQ-6).

### 2.9 Endpoints

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/technique/analyses` | multipart: `lift`, `view`, `session_id?`, N frames → `201` with the analysis **or** the refusal |
| `GET` | `/technique/analyses` | caller's analyses, newest first |
| `GET` | `/technique/analyses/:id` | one analysis |
| `DELETE` | `/technique/analyses/:id` | `204` |

Owner-scoped through one `load_owned` (404-never-403), `AuthenticatedUser`
before `Path`/`Multipart`, `:id` route syntax, exhaustive token mapping — all
per the R-0041/R-0042 conventions.

**A refusal is a `201`, not an error.** It is a *result*: the analysis ran and
honestly declined to score. Same shape as R-0042 returning `InsufficientData`
inside a 200. Malformed input (bad lift name, no frames, over bounds) is a 4xx.

**OQ-5, resolved:** synchronous. 10 frames × MoveNet Lightning is well inside a
request timeout, and the container is already warm from `/health`. If frame
counts grow, the job seam is a later change.

## 3. Code outline

```rust
// core/src/technique/mod.rs — pure, no I/O, no model
pub fn analyze(series: &LiftSeries) -> Result<LiftAnalysis, Refusal> {
    check_frames(series)?;          // §2.3 counts
    check_view(series)?;            // §2.4 — before any verdict
    check_confidence(series)?;
    let reps = segment_reps(series)?;   // §2.5
    let findings = match series.lift {
        Lift::Squat    => squat::findings(series, &reps),
        Lift::Bench    => bench::findings(series, &reps),
        Lift::Deadlift => deadlift::findings(series, &reps),
    };
    Ok(LiftAnalysis { rep_count: reps.len() as u32, findings, view: series.view })
}
```

The api layer's only job is: bounds → `PoseEstimator` per frame → build
`LiftSeries` → call `analyze` → store. **No geometry in the api crate.**

## 4. Non-goals

Per R-0044 §4: no on-device inference, no live coaching, no 3D or multi-camera,
no lifts beyond the three, no auto-logging of weight or reps, no injury-risk
score, and no retention of raw frames (§2.8).

## 5. Open questions — resolved here

- **OQ-1:** bounds table, §2.3.
- **OQ-2:** turning-point segmentation on the hip/wrist vertical, self-scaled by
  the lifter's own segment lengths, §2.5.
- **OQ-3:** shoulder-to-hip width ratio for view detection, §2.4.
- **OQ-5:** synchronous, §2.9.
- **OQ-6:** nullable `session_id`, §2.8.
- **OQ-7:** frames discarded after extraction, §2.8.

**Still open — OQ-4 (thresholds).** Depth is well-defined (hip crease below
knee). Valgus, bar drift, and flare thresholds are judgement calls; the spec
must cite a source per threshold rather than inventing numbers, and until then
those faults report a **measured value with no severity**. Flagged for the
architect and owner.

## 6. Acceptance criteria

AC1 server-side inference (§2.1); AC2 bounds before work (§2.3); AC3 sampling
(§2.1, §2.3); AC4 the guide (§2.7); AC5 the series (§2.2); AC6 segmentation
(§2.5); AC7 calibration is optional and absent from v1's required path;
AC8–AC10 per-lift findings (§2.6); AC11 measured values (`Finding`); AC12
refusal (§2.4); AC13 `Borderline` + stated view; AC14 no medical claims — copy
reviewed; AC15 pure core (§3); AC16 ownership (§2.9); AC17 tests (§7).

## 7. Test plan

**Core (no video, no database)** — synthetic `LiftSeries` built from generated
joint positions:

1. Clean squat: depth below parallel → no `Flagged` depth finding
2. High squat: hip above knee at bottom → `Flagged`, value in degrees
3. Borderline squat within the band → `Borderline`, never pass/fail
4. Knee valgus, front view → flagged; the same series in `Side` → `WrongView`
5. Bench bar-path drift → flagged with normalized value
6. Deadlift hips-shoot-up → flagged via the velocity correlation
7. Deadlift back rounding: setup-vs-bottom angle change
8. Multi-rep set → per-rep findings, correct `rep_count`
9. Each `Refusal` variant reached deliberately
10. Determinism: same series twice → identical analysis
11. Noise stability: ±2 % jitter on keypoints must not flip a `Flagged` verdict

**Integration (`#[sqlx::test]`)** — auth on every route incl. malformed id →
401; upload → 201; over-`MAX_FRAMES` → 4xx before any inference (assert the
fake estimator was **not** called); a refusal stored as a 201 result; ownership
404; delete → 204 then 404. The existing `FakePoseEstimator` supplies scripted
keypoints, so **no fixture video is required anywhere**.

## 8. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-25 | Client extracts frames; server runs inference | Avoids a video decoder (large native dep, CVE surface) in the container, cuts upload ~50×, and keeps AC1's server-side inference intact. Frame extraction is not inference. |
| 2026-08-25 | Raw frames are discarded after pose extraction | The series is all the analysis needs and is far less sensitive than body footage; resolves the retention question instead of deferring it. |
| 2026-08-25 | A refusal is a `201` result, not an error | The analysis ran and honestly declined; R-0042's `InsufficientData`-inside-200 precedent. |
| 2026-08-25 | Self-scaling thresholds (segment-length normalized) | Pixel constants break with camera distance and body size; normalizing by the lifter's own segments is scale-free. |
| 2026-08-25 | Squat needs two clips for depth and valgus | They require perpendicular views. A 45° compromise measures neither well and would produce confident wrong verdicts. |

## Changelog

- _2026-08-25 — created (Draft)._
