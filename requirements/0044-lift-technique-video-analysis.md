# R-0044 — Lift Technique Video Analysis

- **Status:** Draft
- **Milestone:** M6 (photo/pose pipeline) — the movement half
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-08-25
- **Depends on:** R-0013 (pose estimation — `PoseEstimator`, `Landmark`,
  MoveNet/ONNX, all already shipped), R-0004 (workout log — the session a clip
  attaches to)
- **Realized by:** SPEC-0044 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

## 1. Statement

Let a user film a working set of a **squat, bench press, or deadlift** and get
an honest, specific technique assessment: which faults are present, how large
they are, and — when the footage cannot support a verdict — an explicit refusal
rather than a guess.

The method, in the owner's words, is to *"convert the video to its basics"*:
per-frame joint positions reduced to a **vector structure of the lift** — a
short time series of joint angles, bar-proxy path, and tempo. Every judgement
is then made on those numbers by a fixed, deterministic algorithm, not by a
model.

## 2. Rationale

Technique is where a lifter gets hurt and where a remote coach is least able to
help. It is also the one thing this app can assess that a spreadsheet cannot.

The infrastructure already exists and is idle: R-0013 shipped `Landmark`
(COCO-17), `PoseKeypoints`, a `PoseEstimator` trait with a test fake, and
MoveNet Lightning embedded as ONNX — all built for still physique photos.
**Video is that same pipeline with a time axis**, and the analysis on top is
pure geometry that belongs in `fitai-core` beside `goals::assess`.

## 3. Acceptance criteria

### Capture and extraction

- **AC0. Asynchronous analysis.** *(Added 2026-08-25.)* Upload returns
  immediately with an analysis id; the result is polled. Synchronous analysis
  does not fit: the frame budget implies tens of seconds of serialized
  inference against a 5-second client read timeout.
- **AC1. Server-side inference.** The clip is uploaded and analyzed on the
  server, reusing R-0013's `PoseEstimator`. No on-device inference — the
  `project-specifics.md` decision stands (owner-confirmed): one model, improved
  without app releases, behaving identically on every phone in a market where
  Android hardware varies widely.
- **AC2. Bounded input.** Clip duration, file size, resolution and frame count
  all carry explicit caps, enforced **before** decoding. A clip is
  attacker-controlled input driving an expensive expansion — the R-0041/R-0042
  magnitude-bound lesson applies directly.
- **AC3. Sampled, not exhaustive.** Analysis samples frames at a stated rate
  (a rep lasts seconds; 60 fps is waste). The rate is a named constant, not a
  literal, and the sampled series is what the analysis sees.
- **AC4. Per-lift camera requirement.** Each lift declares the view it needs —
  side-on for squat depth and deadlift back angle, front-on for knee valgus,
  side-on for bench bar path. The app tells the user where to put the phone
  **before** filming, and the analysis states which view it assumed.

### The vector structure

- **AC5. Video reduces to a small numeric series.** The output of extraction is
  a time series of `PoseKeypoints` plus derived per-frame quantities (joint
  angles, segment orientations, a wrist-midpoint bar proxy). A 30-second clip
  becomes a few hundred numbers; nothing downstream ever sees pixels.
- **AC6. Rep segmentation.** The series is split into individual reps by
  detecting the movement's turning points. Rep count is reported, and each rep
  is assessed separately — a set is not a single verdict.
- **AC7. Optional calibration improves, never gates.** A user may mark joints on
  a still frame once to supply limb-length ratios, normalizing angles to their
  own body. Absence of calibration degrades precision, never blocks analysis.

### The verdicts

- **AC8. Squat.** Depth (hip relative to knee at the bottom position), knee
  valgus (front view only), and **torso-lean change** through the rep.
  *Amended 2026-08-25:* originally "spine angle change" — not measurable. The
  pose model is COCO-17, which has **no spine landmark**; the shoulder→hip line
  measures torso inclination, and a neutral spine and a rounded spine at the
  same hip angle produce an identical line.
- **AC9. Bench press.** Bar-proxy path deviation, **forearm angle at the touch
  frame**, and touch-point consistency across reps.
  *Amended 2026-08-25:* originally "elbow flare angle" — not measurable from
  the prescribed side view. Flare is humerus abduction, a frontal-plane angle;
  from a camera perpendicular to the bench the upper arm points along the lens
  axis and the far arm is occluded. Forearm-vertical-under-the-bar is the
  sagittal-plane equivalent and a real coaching cue.
- **AC10. Deadlift.** Hip-rise-before-bar (the "stripper deadlift"),
  **torso-lean change** through the pull, and bar drift from the **ankle**.
  *Amended 2026-08-25:* originally "spinal flexion change ... and bar drift
  from the mid-foot". Neither is measurable: COCO-17 has no spine landmark (see
  AC8) and no foot landmark at all — it ends at the ankle, so mid-foot and foot
  length do not exist. Drift is measured from the near-side ankle and
  normalized by shank length, with the ankle disclosed as a posteriorly-biased
  proxy for mid-foot.

- **AC10b. Say what cannot be measured.** The analysis must never report a
  fault it cannot observe. **Back rounding is not detectable** with this pose
  model, and the spec and the UI must say so plainly rather than substituting
  torso lean and letting a lifter read it as a spine check. A silent
  substitution here is the exact failure AC12 forbids, on the fault most likely
  to injure someone.
- **AC10c. Uncertainty, not just confidence.** Every reported value carries an
  uncertainty in its own unit, and `Borderline` means *the uncertainty interval
  straddles the threshold* — not "close to it". MoveNet's keypoint error is
  comparable to the decision margin for depth, and the model reports the hip
  **joint centre** while the depth standard is the hip **crease**, a systematic
  offset of several centimetres. A crisp pass/fail on a biased measurement is a
  false verdict.
- **AC11. Findings are measured, not adjectival.** Every fault carries a number
  and its unit (`hip 8° above parallel at the bottom`), the rep it occurred on,
  and a confidence. "Your depth is bad" is not an acceptable output.

### Honesty — the load-bearing criteria

- **AC12. Refuse rather than mis-score.** When the camera angle is wrong, the
  lifter is partly out of frame, keypoint confidence is below the floor, or no
  rep could be segmented, the response is a typed refusal naming the reason.
  **A confident wrong verdict is worse than no verdict**: telling someone their
  depth is fine when it is not actively causes the injury the feature exists to
  prevent. (The R-0041 `null`-load / R-0042 `InsufficientData` precedent, in the
  setting where it matters most.)
- **AC13. Uncertainty is carried to the surface.** 2D pose from a phone is
  angle-sensitive; a 45° camera makes a legal squat read high. Every measured
  angle carries an uncertainty, findings near a threshold are reported as
  borderline rather than pass/fail, and the assumed camera view is always
  stated.
- **AC14. No medical or injury claims.** Output describes movement, never
  diagnoses. No "this will hurt your back" — a fault is reported as a
  deviation, with the coaching cue as a suggestion.

### Structure

- **AC15. Analysis is pure core.** Everything downstream of pose extraction —
  segmentation, angles, fault detection, verdicts — is deterministic pure
  functions in `fitai-core` over the keypoint series, with no I/O and no model.
  Unit-testable with synthetic joint series and **no video whatsoever**.
- **AC16. Ownership and auth.** Clips and analyses are per-user; another user's
  is **404**, never 403. Every route requires a JWT.
- **AC17. Tests.** Synthetic keypoint series per lift covering: a clean rep, one
  clearly-faulty rep per fault type, a borderline case at each threshold, a
  multi-rep set, and every refusal reason. Plus ownership, auth, and the AC2
  bounds. Fixture clips are permitted for the extraction seam only — the
  analysis suite must not require video.

## 4. Constraints & non-goals

- **Out of scope:** on-device inference (AC1); real-time / live-camera coaching;
  3D pose reconstruction or multi-camera; lifts beyond the three named;
  automatic weight or rep-count entry into the workout log; comparing a user
  against another person's video; storing clips longer than the analysis needs
  (see below); any injury-risk score.
- **Video retention is a privacy decision, not a technical one.** Clips are
  body footage and fall under the same health-data sensitivity as progress
  photos (`project-specifics.md`, M8 legal review). The spec must state a
  retention rule; the default should be to discard the clip once the keypoint
  series is extracted, since the series is all the analysis needs.
- Ephemeral container disk is **not** durable storage — the same trap the
  photo pipeline already has.

## 5. Open questions (deferred to SPEC-0044)

- **OQ-1:** The concrete bounds for AC2 (max seconds, MB, resolution) and the
  AC3 sampling rate.
- **OQ-2:** How is rep segmentation done — a vertical-displacement turning-point
  detector on hip/wrist height, or something more robust to camera shake?
- **OQ-3:** How is a wrong camera view *detected* (AC12)? Candidate: shoulder-
  width-to-hip-width ratio distinguishes front from side, and torso-segment
  visibility flags a partly-framed lifter.
- **OQ-4:** Thresholds per fault, and their source. Depth ("hip crease below
  knee") is well-defined; valgus and bar drift are judgement calls, and the
  spec must say where each number came from rather than inventing them.
- **OQ-5:** Is analysis synchronous on upload, or a job with a result polled
  later? Cold-start plus decode plus N-frame inference may exceed a request
  timeout.
- **OQ-6:** Does a clip attach to a logged set (linking technique to load), or
  stand alone? Linking is far more valuable and costs a foreign key.
- **OQ-7:** Retention default and whether the user can opt into keeping clips.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-25 | Server-side inference | Owner decision, upholding `project-specifics.md`: one model improvable without app updates, identical behaviour across highly variable Android hardware. |
| 2026-08-25 | All three big lifts in v1 | Owner decision. Noted at the time that each lift needs a different camera view and a wrong verdict on one costs trust in all three; per-lift camera requirements (AC4) and refusal (AC12) are the mitigations. |
| 2026-08-25 | Analysis is pure geometry in core, not a model | The ML answers "where are the joints"; everything after is deterministic trigonometry, which is testable, explainable, and fixable without retraining. |
| 2026-08-25 | Refusal is a first-class output | A confident wrong technique verdict causes the injury the feature exists to prevent. This is the strictest application of the codebase's existing honesty precedent. |
| 2026-08-25 | Calibration optional, never gating | The owner's marked-joints idea genuinely improves normalization, but pose still runs per frame — a marked still cannot track a moving body, so it must not be a prerequisite. |

## Changelog

- _2026-08-25 — created (Draft)._
- _2026-08-25 — amended after SPEC-0044 architect review (REJECT): AC8/AC9/AC10
  rewritten to measurements that COCO-17 can actually produce; AC10b (state
  what is unmeasurable) and AC10c (uncertainty) added; AC0 (async) added.
  Three of the original criteria described physically impossible measurements._
