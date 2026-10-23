# R-0062 — Deadlift mechanics on the garust kernel

- **Status:** Draft
- **Milestone:** M6 (the lift model)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-10-23
- **Depends on:** R-0045 as amended 2026-10-23 (the garust kernel, AC17's
  checks, the Winter scaling) and SPEC-0045 slice E (the squat moved onto the
  kernel, which proves the move before a second lift relies on it). R-0058
  (guided measurement) only to supply measured arm and leg lengths. It is not
  a blocker.
- **Realized by:** SPEC-0062 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The model computes the conventional deadlift for a given lifter and load: a
solved start position with the bar on the floor, and a range of balanced
postures from the floor to lockout. Each posture carries the hip, knee, ankle
and L5/S1 moments with their stated uncertainty and assumptions, in the same
result shape the squat uses.

The lift is built from the same kernel as the squat (ADR 0001): a kinematic
tree, its loads and a deadlift-specific posture solver. No moment rule is
written for the deadlift itself.

## 2. Rationale

- **The deadlift is where limb proportions show most.** Arm length sets how
  high the hips start, and femur-to-trunk ratio sets how far the trunk leans.
  Those decide whether the lift loads mostly the hips and back or shares it
  with the knees. That is the per-lifter answer a textbook cannot give.
- **The second lift tests the abstraction.** If the deadlift needs its own
  moment code, the kernel did not do its job.
- **Owner hard requirement (2026-10-23):** real, checkable physics for the
  main lifts, not only the squat.

## 3. Acceptance criteria

### The model

- **AC1. Same kernel.** The deadlift is a `garust` tree with the same segments
  as the squat plus the arms hanging from the shoulders and the bar in the
  hands. Moments come from `garust::physics::multibody`; the deadlift adds a
  posture solver and nothing else to the physics.
- **AC2. The start position is solved, not drawn.** Under stated conventions:
  bar over the mid-foot, arms straight and vertical, bar touching the shins,
  and the combined centre of mass of lifter and bar over the mid-foot. The
  one unknown the solve finds is named in the spec. Arm length is an input
  and changes the solved start.
- **AC3. The range.** A family of balanced postures from the floor to lockout
  under one stated convention for how the joints progress, with the bar path
  vertical. As in R-0045 AC4, each posture is a single balanced position;
  anything read across the family holds under that convention only and is
  never a measured movement.
- **AC4. Moments and groups.** Hip, knee, ankle and L5/S1 moments in N·m,
  signed as resisting extension, attributed to muscle groups only (R-0045
  AC5).
- **AC5. Real load.** An explicit `LoadKg`, or bodyweight-only stated as such
  (R-0045 AC3).

### Checks (R-0045 AC17 applied to this lift)

- **AC6. Two derivations agree** on every posture the tests produce: the
  distal-rule moment and the virtual-work moment, within the tolerance the
  spec states.
- **AC7. Oracles.** At least one hand-computed case per joint with the
  expected value as a literal and its arithmetic beside it. One of them is
  the lockout: standing straight with the bar hanging, where the hip and knee
  moments from the bar reduce to a lever arm read off the drawing.
- **AC8. The arm-length claim.** Two lifters equal in everything except arm
  length: the longer arms start with the hips higher and less trunk lean, and
  the test states the direction and the size of the change in the L5/S1
  moment.

### Honesty

- **AC9. Rejections.** A typed `Rejection` when the start position does not
  balance or the lifter cannot reach the bar with straight arms. Never
  numbers for a posture a human cannot hold.
- **AC10. Assumptions on every result.** Sagittal plane, quasi-static, rigid
  trunk (cannot see rounding), vertical bar path, table-derived masses.
- **AC11. No safety claims.** Load distribution, not risk (R-0045 AC14).

## 4. Constraints & non-goals

- **Out of scope:** sumo and wide stances (frontal plane, a later
  requirement on the 3D tree); the bar flexing or leaving the floor late;
  grip strength; acceleration terms (quasi-static, as the squat; full inverse
  dynamics is its own requirement); spine flexion.
- No physics written outside garust; no new dependency beyond ADR 0001's.
- API, persistence and the app screen are not here; they follow the squat's
  path (SPEC-0045 slice C, R-0059).

## 5. Open questions

- **OQ-1:** Which unknown does the start solve find: hip height, trunk angle
  or shin angle? And which conventions fix the other two?
- **OQ-2:** Where do the shoulders sit relative to the bar at the start:
  directly over it, or slightly ahead, as coaching texts describe? Needs a
  citation, not a choice.
- **OQ-3:** How do the joints progress from floor to lockout: knees extend
  first, then hips, or together? The answer changes every reading across the
  range and must be stated as a convention.
- **OQ-4:** Arm length when R-0058 has not measured it: the Winter table
  value, stated on the result as such?

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-23 | Built on the garust kernel, as R-0045 is | ADR 0001. One kernel for every lift; the deadlift adds a posture solver, not physics. |

## Changelog

- _2026-10-23 — created (Draft)._
