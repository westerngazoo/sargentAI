# 0002 — Pose estimation from video runs on the phone

- **Date:** 2026-11-10
- **Status:** accepted
- **Deciders:** Gustavo Delgadillo (owner), Claude
- **Touches:** `project-specifics.md` (domain notes), R-0044 (amended), R-0065, SPEC-0044 (to rework)

## Context

`project-specifics.md` kept every model on the server: "No on-device
inference", so one model improves without app releases and behaves the same on
every phone in a market where Android hardware varies widely. R-0044 (lift
technique from video) followed it: the phone uploads sampled frames and the
server runs R-0013's `PoseEstimator`.

A lifting video is body footage, the most sensitive data the app would touch.
R-0065 (the lifter's squat against the model) settled that the video stays on
the phone and only marked points travel (owner, 2026-11-08). With the video on
the phone, automatic pose estimation can only run on the phone. On 2026-11-10
the owner chose to move R-0044 there too.

## Decision

Pose estimation from **video** runs on the phone. The video and its frames
never leave the device. The phone sends only the keypoint series (joint
coordinates per sampled frame) plus the name and version of the pose model
that produced them. Everything downstream (rep segmentation, angles, faults,
refusals, the physics) stays on the server, as pure functions over that
series (R-0044 AC15).

The server accepts only a pose model version it knows, so every series it
analyses comes from a model whose accuracy is known. Still photos (R-0013)
are not covered by this decision and keep their server path.

## Alternatives considered

- **Keep server-side pose and upload frames (R-0044 as accepted).** One model,
  improved without releases. Rejected: it puts body footage on the server and
  contradicts R-0065's privacy call, and it needs M8's retention rules before
  any clip can be held.
- **Manual marking only (no automatic pose).** No model at all, the R-0065 v1
  path. Rejected as the only path: marking every frame of a set is too slow
  for routine technique checks. It stays as R-0065's input and as a fallback.
- **Upload, extract, delete at once.** Shrinks retention but the footage still
  crosses the network and lands on the server. Rejected for the same reason
  as the first.

## Consequences

- **Better:** no video on the server, nothing to retain or breach; much less
  upload (a few kilobytes of numbers instead of frames), which matters on the
  target market's mobile data; analysis can answer synchronously, since the
  server only does geometry (R-0044 AC0 changes).
- **Worse:** improving the pose model needs an app release, and old app
  versions keep sending the old model's series. The version check bounds
  that, but the server must keep thresholds per known model version.
- **Worse:** results now depend on the phone. Slow or old Android devices
  may not run the model. SPEC-0044 must set a hardware floor; below it the
  feature is unavailable with a clear message, as AC0b already does for
  frame extraction.
- **Worse:** the mobile app gains an inference runtime and a model file
  (size, a new dependency). SPEC-0044 names it and the alternative it
  rejected.
- SPEC-0044 (Draft) describes the upload path and must be reworked before
  its next slice. Slice A (the pure core, #104) is unaffected.
