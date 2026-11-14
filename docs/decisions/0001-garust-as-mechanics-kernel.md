# 0001 — garust is the mechanics kernel for lift biomechanics

- **Date:** 2026-10-23
- **Status:** accepted
- **Deciders:** Gustavo Delgadillo (owner), Claude
- **Touches:** R-0045 (amended), R-0062, R-0063, SPEC-0045 slice E

## Context

The product's claim is *lift mechanics you can check*. R-0045 built the squat as
a hand-rolled sagittal chain in `fitai-core::biomech` (four joints, fixed
load-side lists, a per-joint sign rule), and R-0045 AC1 forbade any physics
dependency. That model cannot grow: the bench needs shoulder and elbow joints,
two arms and grip width (frontal plane), which is why it was struck from R-0045
on 2026-09-12. Each new lift would be another hand-written chain.

The owner's own `garust` already provides the general machinery: a 3D
kinematic `Tree` over PGA motors with forward kinematics and IK
(`garust-geo`), external `Load`s, and quasi-static inverse dynamics
(`garust-physics::multibody::joint_torque`) with an independent virtual-work
derivation of the same moment (`generalized_torque`). `physics-lab/oraculos`
pins garust's conventions against answers computed outside the engine.

Owner hard requirement (2026-10-23): the physics must be real and checkable,
not a stand-in.

**Evidence.** A parity spike rebuilt every `squat_range` posture as a garust
tree — 2 992 postures across four bodies (1.60–1.95 m), bar 0–250 kg,
dorsiflexion 30°/38°/45° — and compared moments:

| Check | Worst error |
|---|---|
| FK joint centres vs the posture | 6.7e-16 m |
| garust moment vs `fitai-core` moment | 1.3e-12 N·m (moments up to 1 077 N·m) |
| `joint_torque` vs `generalized_torque` (signed) | 1.5e-6 N·m (finite-difference step) |
| Cost, FK + four moments | ~10 µs per posture |

The spike took the centre-of-mass placements from `fitai-core`; it proves the
kernel, not the anthropometry.

## Decision

1. Lift mechanics are computed on `garust`: a lift is a kinematic `Tree`, its
   `Load`s and a lift-specific posture solver. Gravity, lever arms, the
   distal rule and the moment live in garust, not in `fitai-core`.
2. `fitai-core` keeps what is ours: the Winter table and its scaling, the
   placement conventions, the per-lift posture solvers, the result types and
   their stated assumptions.
3. Every reported moment is cross-checked by virtual work, and every lift has
   oracle cases whose expected values are literals computed by hand or cited
   (the `physics-lab/oraculos` rule).
4. The dependency is `garust` by git, pinned to rev
   `55e91407ba77c781f18020e2e329c5c44e86c4b6`, until a crates.io release
   containing `tree` and `multibody` exists (the published `0.1.0` predates
   them). Then switch to the version.
5. `physics-lab/mecanica` is prior art, not a dependency: its anthropometry is
   the one that disagreed with ours by 35% on the squat.

## Alternatives considered

- **Keep the hand-rolled chain, add lifts by hand.** Rejected: a second and
  third copy of the moment rule is where sign errors hide, and the bench
  needs 3D the chain cannot express.
- **Depend on `mecanica`.** Rejected: teaching models tied to the reels, with
  the anthropometry R-0045 corrected.
- **A general engine (Rapier and similar).** Rejected: they integrate bodies
  forward, which is the opposite problem; inverse dynamics is not their API.
- **Forward simulation of the lifter.** Rejected: driving a body forward needs
  per-muscle forces nobody knows; inventing them breaks "you can check".

## Consequences

- R-0045 AC1 and AC15 are amended; the bench returns as R-0063 and the
  deadlift arrives as R-0062.
- `fitai-core` gains its first non-trivial dependency. garust is `no_std`,
  zero-dependency and `MIT OR Apache-2.0`, so the purity boundary (no I/O
  crates) still holds and is checked with `cargo tree`.
- A git-pinned dependency must be bumped deliberately; garust changes now need
  fitAI's parity tests green before the pin moves.
- The squat must move onto garust with **identical** slice A and B1 golden
  values, which is the regression gate for the move (SPEC-0045 slice E).
- garust's rotation sign (positive about +z carries +x toward −y) differs from
  the right-hand rule; the mapping to the "resisting extension" sign lives in
  one place, as it does today.
- Full inverse dynamics with acceleration terms is not in garust's
  `multibody` yet; that is a later requirement and a garust change.
