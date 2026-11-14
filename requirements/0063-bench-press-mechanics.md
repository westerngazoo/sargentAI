# R-0063 — Bench press mechanics on the 3D garust kernel

- **Status:** Draft
- **Milestone:** M6 (the lift model)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-10-23
- **Depends on:** R-0045 as amended 2026-10-23 (the garust kernel, AC17's
  checks) and SPEC-0045 slice E (the squat on the kernel). R-0062 is not a
  dependency, but sequencing it first means two lifts have proven the kernel
  before the first 3D one relies on it.
- **Realized by:** SPEC-0063 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The model computes the flat barbell bench press for a given lifter, load and
grip width: a range of balanced postures from the bar on the chest to
lockout. Each posture carries the shoulder and elbow moments of **both
arms**, in three dimensions, with their stated uncertainty and assumptions.

It is the first lift that needs the third dimension. Grip width and how far
the elbows flare are frontal and transverse quantities a sagittal model
cannot express. That gap is why the bench was struck from R-0045 on
2026-09-12, and the garust tree closes it (ADR 0001).

## 2. Rationale

- **The bench's coaching questions are about width.** Grip width and elbow
  flare move load between the shoulders and the triceps. Both are 3D, and the
  answer depends on the lifter's own arm and shoulder proportions.
- **It is the hard test of "real physics".** A sagittal shortcut would give
  a number here that looks fine and is wrong. The 3D tree either holds, or
  the requirement says why it does not.
- **Owner hard requirement (2026-10-23):** the main lifts, with real physics.

## 3. Acceptance criteria

### The model

- **AC1. A 3D tree on the kernel.** Trunk fixed to the bench as the root,
  then for each arm a shoulder with the degrees of freedom the spec names and
  an elbow, and the bar held in both hands. Moments come from
  `garust::physics::multibody` as 3D vectors; nothing is projected onto a
  plane before the moment is computed.
- **AC2. The bar is shared by both hands.** The bar load is applied at both
  hands. A symmetric grip splits it evenly, and the test proves the split
  from the geometry rather than assuming it.
- **AC3. Grip width is an input,** with a stated plausible range. Width
  outside the range, or a grip the arms cannot reach, is rejected, never
  extrapolated.
- **AC4. The range.** A family of balanced postures from the bar touching the
  chest to lockout, with one stated convention for the bar path and the
  elbow angle along it. As in R-0045 AC4, each posture is a single balanced
  position and is never a measured movement.
- **AC5. Moments and groups.** For each arm: the shoulder moment, decomposed
  on named anatomical axes (horizontal flexion and abduction at least), and
  the elbow extension moment, in N·m with stated signs. Attributed to groups
  only (pectorals and anterior deltoid, triceps), never to single muscles
  (R-0045 AC5).

### Checks (R-0045 AC17 applied to this lift)

- **AC6. Two derivations agree** on every posture the tests produce, for
  every degree of freedom of both arms.
- **AC7. Oracles.** Hand-computed literals, including:
  - forearms vertical in both planes, which gives an elbow moment from the
    bar of zero;
  - a grip wider than that, where the elbow moment is a lever arm read off a
    front-view drawing;
  - lockout with the arms vertical, where the shoulder moment from the bar
    is zero.
- **AC8. Symmetry.** A symmetric grip gives mirror-image moments on the two
  arms. An asymmetric grip breaks the symmetry in the direction the
  geometry predicts.
- **AC9. The width claim.** Widening the grip from the forearms-vertical
  width increases the elbow moment, and the test states its size for a
  reference lifter.

### Honesty

- **AC10. Assumptions on every result.** Quasi-static, rigid segments, a
  fixed trunk (no arch modelled beyond a stated touch height), no leg drive,
  and table-derived masses. Shoulder width comes from a measurement or a
  cited table, never from a guess.
- **AC11. No safety claims.** Load distribution, not risk (R-0045 AC14).

## 4. Constraints & non-goals

- **Out of scope:** leg drive; arch as a modelled spine; the scapula as its
  own joint (a stated convention places the shoulder centre); incline,
  decline, dumbbells and machines (each is its own load or root on the same
  kernel, later); acceleration terms (quasi-static, as the squat); bar flex.
- No physics written outside garust; no new dependency beyond ADR 0001's.
- API, persistence and the screen follow the squat's path.

## 5. Open questions

- **OQ-1: Shoulder width.** Winter's Table 4.1 gives no biacromial breadth.
  Options: a measurement (extend R-0058), or another published table such as
  ANSUR II, cited with its population. This must be decided before
  acceptance: it sets every grip-width answer.
- **OQ-2:** How many degrees of freedom at the shoulder: two (flexion,
  abduction) or three, with rotation? And how is the redundancy resolved,
  since a bar path and grip do not fix the elbow's height alone?
- **OQ-3:** The elbow-flare convention along the range: fixed angle, or solved
  from a stated rule?
- **OQ-4:** Where the bar touches the chest, and its path to lockout
  (straight, or the J-curve coaching texts describe). This needs a citation.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-23 | The bench returns as its own requirement, in 3D on the garust kernel | ADR 0001. The 2026-09-12 strike (R-0045) was for want of elbow and shoulder joints and a frontal plane; a garust tree has both. |

## Changelog

- _2026-10-23 — created (Draft)._
