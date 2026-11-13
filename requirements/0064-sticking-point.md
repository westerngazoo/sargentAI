# R-0064 — The sticking point: demand against muscle capacity

- **Status:** Draft
- **Milestone:** M6 (the lift model)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-10-29
- **Depends on:** R-0045 (the demand: joint moments over a range of
  postures) and, if OQ-2 is answered "garust", ADR 0001 and SPEC-0045 slice E
  (#160). R-0059 (the squat check) to show the result. R-0058 (guided
  measurement) only for measured lengths. It is not a blocker.
- **Realized by:** SPEC-0064 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

For a lift range the model already computes (the squat first), the model also
computes, at every posture, **how much moment the muscles crossing each joint
can produce there**: the capacity. Each muscle's capacity comes from its
moment arm, which changes with the joint angle, and its force, which changes
with its length. Both come from a published, cited musculoskeletal model,
scaled to the lifter.

Dividing the demand (R-0045's joint moment) by the capacity, posture by
posture, gives the **sticking point**: the posture and the muscle group where
the lift is closest to the group's limit. The app marks it on the squat
check and names the group, with the model's assumptions on the same screen.

## 2. Rationale

- **"Where do I get stuck, and why" is the coaching question** the demand
  alone cannot answer. The demand peaks where the levers are longest; the
  lifter stalls where the demand is largest *relative to what the muscles can
  do at that angle*, and those are not the same posture (owner, 2026-10-29,
  after a post on moment arm × force against knee angle).
- **It is the next layer of the same physics.** R-0045 is the outside of the
  body: loads, segments, levers. This requirement is the inside: muscle
  paths, lengths and forces, on the same postures. No new kind of claim, only
  a second half of the same moment balance.
- **It keeps the product's promise only if every number is traceable.**
  Muscle parameters are where invented constants hide. A cited model, a
  pinned version and checks against its published curves are what make this
  "comprobable" rather than an illustration.

## 3. Acceptance criteria

### The model

- **AC1. One cited source.** Muscle parameters (maximum isometric force,
  optimal fibre length, tendon slack length, pennation angle) and muscle
  paths come from one published musculoskeletal model, cited, with its
  version pinned and its licence compatible with the product. Every
  parameter used is traceable to that source; none is invented. A muscle
  group the source does not model is not reported (see OQ-3), never filled
  with a guess.
- **AC2. Scaled to the lifter.** Paths and fibre lengths scale with the same
  segment lengths R-0045 uses (measured or table-derived); the scaling rule
  for maximum force is stated. The result says which inputs were
  table-derived, as R-0045 does.
- **AC3. Moment arms per muscle, per posture.** Each muscle's moment arm
  about each joint it crosses, at each posture of the range.
- **AC4. Capacity per joint group, per posture.** Each muscle's maximum
  isometric force at its length at that posture (the source's force–length
  relation, pennation included), times its moment arm, summed over the
  muscles that extend the joint. Stated assumptions: full activation,
  isometric (no velocity term), no fatigue.
- **AC5. Two-joint muscles are stated, not hidden.** A muscle crossing two
  joints (the hamstrings, rectus femoris, gastrocnemius) is counted at each
  joint under a stated convention, and the result names that convention.
- **AC6. The sticking point.** Per posture and per group, the ratio
  demand / capacity. The result names the posture and the group with the
  highest ratio. A ratio above 1 is reported as "the model's capacity is
  exceeded here", a statement about the model, never a prediction that the
  lifter will fail.

### Checks (R-0045 AC17 applied to this layer)

- **AC7. Two derivations agree.** Each moment arm computed geometrically from
  the muscle path agrees with the tendon-excursion derivation
  (`r = −dL/dθ`) on every posture the tests produce.
- **AC8. The source's curves.** For the source's own generic model (unscaled),
  moment arms and force–length values reproduce the source's published values
  within a stated tolerance.
- **AC9. One hand-computed case** per joint group: a single muscle in a fixed
  posture, computed by hand outside the engine, pinned as a literal.

### Honesty and the app

- **AC10. Assumptions on every result**: the source and version, population
  parameters (not the lifter's own strength), full activation, isometric,
  the two-joint convention, the groups not modelled.
- **AC11. Shown on the squat check.** The sticking point is marked on the
  range (the scrubber and the live graph) with the group's name and the
  ratio; demand and capacity are drawn against each other for the selected
  group. Computed on the server; the app computes nothing (as R-0059 AC7).
- **AC12. No injury or safety claims.** Capacity is a model of what muscles
  can produce, not of what is safe (R-0045 AC14).

## 4. Constraints & non-goals

- **Squat first.** The deadlift (R-0062) and bench (R-0063) get capacity once
  their demand exists.
- **Not a muscle-force distribution.** How the nervous system shares a moment
  among muscles (static optimisation, EMG) is not needed for capacity and is
  out of scope.
- **No forward simulation**, no muscles driving the body, no velocity or
  fatigue terms.
- **Not the lifter's own strength** until calibrated (OQ-4).
- **Not every bone.** The skeleton needed here is the segments the lift's
  muscles cross; a full-body skeleton is OQ-5.

## 5. Open questions

- **OQ-1. The source model.** Candidates: Rajagopal et al. 2016 (lower limb,
  80 muscle–tendon units, OpenSim), Arnold et al. 2010, Delp et al. 1990.
  Choice by coverage of the squat's groups, published validation, and
  licence.
- **OQ-2. Kernel.** Build the muscle layer on garust (needs #160 and
  SPEC-0045 slice E first), or on `fitai-core`'s chain and move it with the
  squat. Recommendation: garust, so it is built once.
- **OQ-3. The lumbar group.** Lower-limb models carry no back extensors, and
  the squat's L5/S1 demand needs one (candidates: Christophy et al. 2012,
  Raabe & Chaudhari 2016). Add a second cited source for the lumbar group, or
  report the squat's sticking point over hip, knee and ankle only at first.
- **OQ-4. Personal calibration.** Scale maximum forces so the model's
  capacity matches the lifter's logged heavy sets. Later, or part of v1?
- **OQ-5. The full-body skeleton.** A full-body multibody skeleton on garust
  (pelvis, spine in segments, scapulae, arms), which the bench (R-0063) also
  needs: a stage of this requirement, or its own requirement.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-29 | The sticking point (demand against capacity) comes before showing muscles at work | Owner: the coaching answer first, the visual second. |
| 2026-10-29 | Segments the muscles cross, not every bone | Bones that don't change a lift's numbers add parameters with no data behind them. |

## Changelog

- 2026-10-29 — created (Draft), from the owner's skeleton-machine proposal.
