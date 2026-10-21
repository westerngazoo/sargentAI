# R-0059 — The squat, animated in the app

- **Status:** Draft
- **Milestone:** M6 (the lift model, shown to the lifter)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-10-20
- **Depends on:** R-0045 (the range of solved postures, merged: slices A and
  B1). R-0058 (guided measurement) only to fill the request from the profile
  later. It is not a blocker.
- **Realized by:** SPEC-0059 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The app shows a lifter their own squat as an animation, from standing to the
bottom. The R-0045 model computes it on the server from their measurements,
ankle dorsiflexion and load. The app draws:

- a side-view figure with the bar and the balance line;
- per frame, the joint angles and trunk lean;
- per frame, the hip, knee, ankle and L5/S1 moments with their stated
  uncertainty.

The model's assumptions are shown on the same screen.

## 2. Rationale

- **A table of numbers is not the product** (owner, 2026-10-20). The model's
  output is a sequence of postures. Watching it is how a lifter understands
  where their body does the work.
- **It is the reference the video comparison will be laid against.** The
  next step is the lifter's own video (R-0044's measured source). It needs a
  picture of the model to compare with.
- **The design exists.** A scratch prototype was drawn from `squat_range`
  for the owner's own numbers: figure, playback, a load selector and moment
  bars. It is not committed; SPEC-0059 makes it the screen design.
- **Intelligence stays on the server** (`project-specifics.md`). The app
  draws what the server returns and computes no physics.

## 3. Acceptance criteria

- **AC1. A compute endpoint.** An authenticated endpoint takes:
  - height and body mass;
  - optional thigh, shank and trunk lengths;
  - dorsiflexion, required, in `0..=90` degrees;
  - an optional bar load (high-bar, the only placement the model has);
  - a bounded frame count.

  It returns one entry per frame: progress, the joint points (ankle, knee,
  hip, shoulder, bar), knee and hip flexion, trunk lean, and the four
  moments, each with its uncertainty and resisting group. It also returns how
  the range ended and every stated assumption. No session gets **401**; an
  invalid field gets **422** with a typed error naming the field.
- **AC2. Measurements travel in the request** (owner, 2026-10-20). The
  endpoint stores nothing. When R-0058 lands, the app fills the request from
  the profile. Missing lengths come from the table, and the result says so.
- **AC3. A depth the lifter cannot reach is a result, not an error.** When
  the range stops before the bottom, the response is a success. It carries
  the frames up to where the lifter got, plus the reason. The app says the
  reason in plain words, for example "without load your ankle stops you at
  57% of the way down". If the model cannot place the lifter at all, the app
  shows the typed reason, never an empty animation with no explanation.
- **AC4. The screen.**
  - The figure: feet, shank, thigh, trunk, head, arms, and the bar with a
    plate drawn to scale.
  - The mid-foot balance line.
  - Playback: play, pause, and scrubbing frame by frame.
  - A load choice: bodyweight plus the loads the lifter enters.
  - Per-frame readouts of knee flexion, hip flexion and trunk lean.
  - Per-frame moment bars with their N·m values. A negative moment (the load
    assisting extension) is drawn distinctly.
- **AC5. The caveats, on the screen:**
  - the stated assumptions;
  - which lengths came from the table;
  - that arms hang and are not used as a counterweight;
  - that the legs move between standing and the bottom by a stated
    convention, so where a moment peaks across frames belongs to the
    convention, not to the lifter (SPEC-0045 §2.6.6).
- **AC6. No injury or safety claims** (R-0045 AC14). Colours identify
  joints, never good and bad. No wording calls a torque dangerous or safe.
- **AC7. Thin client.** The app may interpolate between returned frames for
  smooth playback; that is display only. It computes no posture or moment.
- **AC8. Privacy.** The request carries body measurements. The endpoint
  neither stores them nor logs them. R-0024's inventory records them as
  "processed, not stored".
- **AC9. Tests.**
  - Backend: a valid request, each invalid field, no session, a stopped
    range, and a lifter who cannot be placed.
  - Flutter widget tests: frames render, playback and scrubbing, the
    stopped-range message, and a negative moment's rendering.
  - The e2e test is owned by the `qa` agent.

## 4. Constraints & non-goals

- **The high-bar squat only,** because that is what the model covers.
  Low-bar and other lifts are out until R-0045 has them.
- **No video comparison here.** That is the step after this one.
- **No saved variants.** SPEC-0045 slice C (persistence and export) is
  separate. This endpoint is stateless and is the first route in the API's
  `biomech` module that slice C will extend.
- **No new mobile dependency is expected.** Drawing is Flutter's own
  `CustomPainter`. Any dependency SPEC-0059 adds is named together with the
  alternative it rejected.
- **The reel video (the guion bridge) is a separate path.**

## 5. Open questions

- **OQ-1. Where the screen lives.** The app has no per-exercise guide screen
  yet, and R-0058 AC7 assumes one. SPEC-0059 either creates the squat's
  guide screen with the animation as its first section, or places the
  animation elsewhere until that screen exists.
- **OQ-2. Which loads are offered.** Recommended for v1: typed in by the
  lifter. Later, from their logged working sets.
- **OQ-3. Screen language.** The app's strings are English today, and the
  target market is Mexico and LATAM. Localisation is out of scope here;
  noted so it is not forgotten.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-20 | The animation is the product, not the table | Owner. |
| 2026-10-20 | Measurements travel in the request until R-0058 stores them | Owner. The animation does not wait for R-0058; the profile fills the request later. |
| 2026-10-20 | The model runs on the server; the app only draws | `project-specifics.md` domain note: intelligence server-side, thin client. |
