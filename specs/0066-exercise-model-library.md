# SPEC-0066 — The exercise model library on the garust kernel

- **Status:** Draft, revision 2 — the architect's REJECT of 2026-11-21 applied
  (§9); for re-review, then owner acceptance
- **Realizes:** R-0066 (Accepted 2026-11-21)
- **Author:** Claude (main session), for owner review
- **Created:** 2026-11-21
- **Depends on:** ADR 0001 (garust as the kernel), SPEC-0045 (slices A and
  B1: the squat solve and its range), SPEC-0059 (the range endpoint, its
  validation table and wire shape), and one garust change (§2.1.1)
- **Module(s):** garust `garust-physics` (one fix, §2.1.1);
  `backend/crates/core/src/biomech/` (kernel glue, the squat move);
  `backend/crates/core/src/exercise/` (new: the library);
  `backend/crates/api/src/exercise/` (new: the endpoints)

## 1. Motivation

R-0066: every modelled exercise is a tree on the garust kernel, with its
loads, its equipment and a motion convention, served by one endpoint. R-0067
builds an interactive screen on it, with controls the server describes. The
owner asked to start with five exercises (2026-11-21).

Four facts shape this spec:

1. **garust is not wired in yet.** ADR 0001 decided the kernel and the parity
   spike proved it on the squat (1.3e-12 N·m over 2 992 postures). But
   `fitai-core` still computes moments with its hand-written chain
   (`chain.rs`). So the squat moves first, behind a parity gate (R-0045
   AC17(c), R-0066 AC10).
2. **garust has the joints and loads five exercises need**: `Tree` with
   `Revolute` and `Prismatic` joints, `Weight` (a mass under a gravity vector,
   which also serves as any constant force) and `Cable` (a tension toward a
   fixed pulley), `joint_torque`, `generalized_torque` and
   `twolink::two_link_joint`, all at rev `55e91407`.
3. **But garust's virtual work is wrong for a cable.** `generalized_torque`
   differentiates `W(q) = −Σ F(q)·x(q)`, which is right only for a force that
   does not depend on the posture. A cable's direction does, so the result
   gains a spurious `−x·∂F/∂q` term. The architect's check, reproduced in the
   main session: one link, 200 N, `q = 0.4`, pulley `(0.3, 2.0)`. The cross
   product gives 54.97 N·m, garust's virtual work 58.48, and virtual work with
   `F` held at the nominal posture 54.97. AC5's cross-check could never pass
   for a cable. The fix belongs in garust (§2.1.1). Computing virtual work in
   fitAI would put a moment rule outside the kernel, which ADR 0001 §1
   forbids.
4. **Guided machines are statically indeterminate.** The seat or pad and the
   platform share the load in a way statics alone does not fix. §2.7 states
   the assumption that makes them determinate and makes it a setting.

## 2. Design

### 2.1 The kernel

#### 2.1.1 The garust change (slice 66-0, in the garust repository)

`generalized_torque` evaluates the loads once at the nominal posture and
differences only their application points:

```
τ_j = Σ_loads F(q) · (x(q + h·e_j) − x(q − h·e_j)) / 2h
```

This is the principle of virtual work as stated: the forces are held, the
displacement is virtual. For a constant force it is identical to today's
formula, so every existing garust test still passes unchanged. The change
adds a cable agreement test (`joint_torque` against `generalized_torque` for a
`Cable` at three pulley positions, with literals). Then the pin moves to the
new rev. ADR 0001 requires fitAI's parity tests to be green before the pin
moves. The bump is recorded in ADR 0001's consequences and in §7 here.

#### 2.1.2 `biomech/kernel.rs` — the only module that imports garust

```rust
/// A planar tree for garust: links parent-first, which reported joint each
/// link is, and which anatomical side is distal at that joint in this tree.
pub struct PlanarTree {
    links: Vec<TreeLink>,           // revolute about z, axis through the link's frame origin
    reported: Vec<Reported>,        // { link, joint, distal: Side }
    loads: Vec<LoadSpec>,           // Weight { link, at, kg } | Force { link, at, newtons } | Cable { link, at, pulley, newtons }
}

/// Moments at every reported joint, each cross-checked by virtual work.
pub fn moments(tree: &PlanarTree, q: &[f64]) -> Result<Vec<JointMoment>, KernelError>;

/// The force and moment the root takes: minus the sum of every applied load.
pub fn root_reaction(tree: &PlanarTree, q: &[f64]) -> Reaction;
```

- **The world origin is the tree's root.** That keeps `|F·x|` small and the
  cross-check's rounding bounded.
- **The cross-check (R-0066 AC5).** `joint_torque` per reported joint, then
  `generalized_torque` with `h = 1e-6`. They must agree within
  `1e-4 + 1e-9·|τ| N·m`, a relative term for heavy loads. One test pins the
  largest load each model allows. A disagreement is `KernelError::Disagree`.
  The API maps it to `500`, logs the exercise id and the joint, and logs no
  body value. It is a modelling bug, not a refusal, and it is never served
  (R-0066 AC7).
- **A constant force** (a platform's push, §2.7) is a garust `Weight` with
  `mass = 1.0` and `gravity = F`, built by `kernel::force(link, at, F)`.
  `Weight` is documented for loads on an incline. No new garust type is
  needed.
- **Invariant:** every revolute axis passes through its link's frame origin,
  since `joint_point` is that origin. A test checks that FK joint centres
  match the posture to 1e-12 m, the spike's 6.7e-16 m check.
- **The dependency:** `garust-geo` and `garust-physics` by git, pinned by rev
  (no umbrella crate, so no `physics` feature to forget). `cargo tree -p
  fitai-core` shows no I/O crate (R-0045 AC15).

#### 2.1.3 The sign convention, for every joint

SPEC-0045 §2.4.1 generalized: **a moment is positive when the external loads
tend to flex the joint**. So its extensors are the group resisting.
Negative means they tend to extend it, and its flexors resist. The squat's
values keep their signs. `resisting` follows the sign:

| Joint | Flexion is | Positive → resisting | Negative → resisting |
|---|---|---|---|
| ankle | dorsiflexion | plantar flexors | dorsiflexors |
| knee | knee bending | knee extensors | knee flexors |
| hip | thigh toward trunk | hip extensors | hip flexors |
| lumbar | trunk forward | spinal erectors | abdominals |
| shoulder | arm forward and up | shoulder extensors | shoulder flexors |
| elbow | forearm toward upper arm | elbow extensors | elbow flexors |

So a dumbbell curl's elbow is negative (the weight tends to extend it, the
flexors work). A pushdown's elbow is positive, and so is a pulldown's
shoulder.

**From garust's `z` to that sign** there are two factors, never a
per-joint table: `sign = flexion_dir(joint) × side(tree, joint) × τ_z`.
- `flexion_dir(joint)`: the rotation sense, about garust's `z`, that flexes
  the joint when the distal anatomical segment moves. One row per joint,
  derived once from the side view (`+x` toward the toes).
- `side`: `+1` when the tree's distal side at that link is the anatomical
  distal segment (rooted at the shoulder, the elbow's distal side is the
  forearm), `−1` when it is the proximal one (rooted at the foot, the knee's
  distal side is the thigh and everything above).

Each `PlanarTree` carries `side` per reported link. **A test pins every
(exercise, joint) pair** with a posture whose sign is obvious by inspection
(a held dumbbell loads the curl's elbow flexors; a platform pushing ahead of
the knee loads the leg press's knee extensors).

### 2.2 The squat on the kernel (slices 66-a1, 66-a2)

**What stays.** `ChainSegment` stays, possibly renamed, as the squat's list
of lumped segments and their placements (`com_with(body, p, Placement)`). It
is the single source for both the balance solve (`squat.rs`, which relies on
`Σ m·x` being affine in the sines, `chain.rs:160-170`) and the tree builder.
`Body::mass_kg(ChainSegment)` stays. What goes are the moment functions:
`above`, `moment_about`, `moment_about_with`, `body_moment_about`,
`segments_sum`, `load_sum`, `raw_sum`, `extension_sign`.

**The tree** (`squat::tree(body, posture, placement) -> PlanarTree`):
- Links: foot (root, on the floor) → ankle → shank → knee → thigh → hip →
  pelvis → L5/S1 (placed by `placement.l5s1`, SPEC-0045 §2.5) → thorax.
- Each `ChainSegment` is a `Weight` at its `com_with` point on the link that
  `above()` puts it on today: shank on the shank, thigh on the thigh, pelvis
  on the pelvis link (on the hip's side, not the lumbar's), and thorax-abdomen,
  head and neck, arms and the bar on the thorax. **The head and neck sit on
  the trunk line beyond the shoulder, and the arms at the shoulder-to-bar
  midpoint or hanging**, exactly as `com_with` places them. Nothing is lumped
  into a HAT weight.
- `side = −1` at every squat joint (rooted at the foot, the distal side is
  the anatomical proximal one).

**66-a1: both paths alive.**
- Adds the dependency, `kernel.rs`, its unit tests, and `squat::tree`.
- Adds a parity test: every posture of a dense grid (the spike's four bodies,
  four bars, three dorsiflexions, every frame of each range, about 3 000
  postures, and all nine `Placement` corners of a subset) through both paths,
  asserting `|kernel − chain| ≤ 1e-9 N·m` at every joint.
- The same PR snapshots the kernel's values over that grid at full precision
  into a fixture (`tests/fixtures/squat_kernel_parity.json`). That is what
  survives `chain.rs`.
- Amends SPEC-0045's slice E row: recast by ADR 0001 as "the squat on the
  garust kernel", realized here.
- Nothing else changes, and the endpoints are untouched.

**66-a2: the switch.**
- `analysis.rs`: the stated moments and the 8 uncertainty corners call
  `kernel::moments(squat::tree(body, p, corner), q)`, one tree per corner, as
  `moment_about_with` is called per corner today. About 10 µs per tree and
  nine trees per frame keep a 41-frame range under 4 ms.
- `api/biomech/lever.rs` (SPEC-0059's *La cuenta*): `body_nm` becomes the
  kernel's moment over the segment loads only, the same tree without the bar.
  Its postcondition is kept and tested: `bar_nm + body_nm == hip moment` to
  1e-9.
- The slice-A oracle tests (`tests/mod.rs`, R-0045 AC17(b)) and
  `tests/range.rs` call the kernel path with their tolerances unchanged.
  `examples/squat.rs` too.
- The parity test reads the 66-a1 fixture: `≤ 1e-9 N·m` against literals
  that came from a green run with both paths alive.
- The moment functions are deleted.
- `POST /biomech/squat/range` answers byte-for-byte as before. Its e2e test
  is the proof.

### 2.3 The library (slice 66-b1)

`core::exercise` (new). One trait, six implementations on day one:

```rust
/// One exercise: what can be changed, and its range of postures.
pub trait ExerciseModel: Sync {
    /// The controls, the equipment, the joints, and the stable id.
    fn describe(&self) -> Description;
    /// The range for validated inputs: complete, stopped part-way, or rejected.
    fn range(&self, input: &Input) -> Result<ExerciseRange, KernelError>;
}

/// Every modelled exercise. The only list.
pub fn library() -> &'static [&'static dyn ExerciseModel];
```

- **`Description`** (built by value, cheap): `id` (stable, never renamed),
  names (es and en), aliases (for `model_for`), `Equipment` (R-0066 AC2's
  four kinds; lever/cam is in the enum and has no entry yet, per R-0066
  OQ-3), the reported joints, the parameters, the pair rules and the
  assumptions.
- **`Parameter`**: `{ key, group, kind }`. `group` is `motion | machine |
  load`. The kind is either `Number { unit, min, max, step, default }` or
  `Choice { options, default }` (for example `cop: heel | mid | toes`).
- **Pair rules**: `Ordered { low, high }` means `low < high`, else `400` with
  `field` = the `high` key (the range of a motion: `knee_extended_deg <
  knee_flexed_deg`). The frames always run in the model's stated direction.
  The *keys* say which end is which, so no default runs backwards.
- **The body is not a parameter.** `height_m`, `mass_kg` and `lengths` travel
  at the request's top level for every exercise, as SPEC-0059 does. The
  description lists which `lengths` keys the exercise uses, so R-0067 can
  show the right *Cuerpo* controls.
- **`scale` and `MeasuredLengths` grow** `upper_arm` and `forearm` (Winter
  rows already in `WINTER`), so the accepted `lengths` keys are `thigh`,
  `shank`, `trunk`, `upper_arm` and `forearm`. Each is bounded `0.1..=1.5 m`,
  SPEC-0059's bound.
- **Validation is generic**: one function checks `values` against the
  description. An unknown key is `400` with that key. A missing key takes its
  default. A number outside `min..=max` or a choice not in `options` is `400`.
  A broken pair rule is `400`.
- **`ExerciseRange`**: `{ end, frames, settings, table_lengths, assumptions,
  reactions? }`.
  - `end` generalizes SPEC-0059's: `complete`, `stopped { reached, reason }`
    (a prefix of frames that the model stands behind, then the reason), or
    `rejected { reason }` (no frames).
  - `Reason` is a closed library enum: the squat's `Rejection` wrapped
    unchanged, `OutOfReach { frame }`, `NoLegSolution { frame }`.
  - Each frame is `{ progress, points, moments }`. `points` are named
    (`ankle`, `knee`, `hip`, `l5s1`, `shoulder`, `elbow`, `hand`, `pulley`,
    `platform`…), and `moments` uses the SPEC-0059 shape `{ joint, resisting,
    moment, uncertainty }`.
  - `settings` restates every value used, defaults included (R-0066 AC2).
  - `reactions` holds the root's reaction and the platform force for a
    supported exercise (R-0066 AC4).
- **Assumptions** extend SPEC-0045's `Assumption` with values that stay `Eq`
  and `Hash` (no `f64` in them):
  - `GenericMachine { differs: &'static [&'static str] }` (R-0066 AC3, for
    example `["cam_profile", "pulley_ratio"]`);
  - `PlatformForceAlongRail`;
  - `RailFriction` (μ itself is in `settings`);
  - `FixedRoot { at: &'static str }`;
  - `BalanceNotModelled` (§2.5).
- **`back_squat`** is an adapter over `squat_range`: `dorsiflexion_deg`
  (motion) and `bar_kg` (load, `0..=1000`). `0` maps to `BodyweightOnly`,
  because `LoadKg` is `(0, 1000]`. `bottom` maps to `complete`, `stopped` to
  `stopped`, and a slice-A rejection to `rejected`, with the frames
  unchanged.
- **`model_for`** reads the library's names and aliases instead of its
  `matches!`. **The plan's `physics` wire value for the squat stays
  `"squat"`**, an alias of `back_squat` on the wire, so installed app builds
  keep their *Física* button. The new exercises' ids are not emitted in the
  plan until SPEC-0067's screen can open them. A `served_in_plan` flag on
  the description says which entries the plan exposes, and only `back_squat`
  has it until then.
- **Uncertainty**: each model names the placements it nudges and their
  spreads. The corner walk runs through the same code path as the stated
  placement (SPEC-0045 §2.8). The spreads reuse `COM_PLACEMENT_SPREAD_M`
  (0.02) and `L5S1_PLACEMENT_SPREAD_M` (0.03) where the placement is of that
  kind, and §2.5–2.7 name them per model.

### 2.4 Serving (slice 66-b2)

```
GET  /exercise-models               → [{ id, name, equipment, joints }]
GET  /exercise-models/{id}          → Description (R-0066 AC9)
POST /exercise-models/{id}/range    → ExerciseRange (R-0066 AC8)
```

- **The request body** is `{ height_m, mass_kg, lengths?: {key: m}, values?:
  {key: number | string}, frames? }`. Height, mass and `frames` use SPEC-0059's
  table and bounds unchanged.
- **Status codes:**
  - an invalid field is `400 {"error":"validation","field":…}`;
  - no session is `401`;
  - an unknown id is `404`;
  - a `stopped` or `rejected` end is `200`, as SPEC-0059;
  - a kernel disagreement is `500` (§2.1.2).
- **Privacy** follows SPEC-0059 AC8: the request's `Debug` prints which fields
  are present, never a value, and nothing is stored.
- **Range serialization** lives in the api crate. Core returns core types
  only.
- `POST /biomech/squat/range` stays until the app moves to the new endpoint.
  Its removal is SPEC-0067's (R-0066 AC10). A test checks that the squat
  through the new endpoint equals the old one within 1e-9 N·m at every frame
  and joint.

### 2.5 The standing exercises with a fixed root (slice 66-c)

The curl and the pushdown are done standing, but their tree is rooted at a
fixed shoulder, and they report only the elbow and the shoulder. Those
moments are the moments of the arm and its load, which do not depend on
where the rest of the body stands. So no balance is solved, and the result
carries `FixedRoot { at: "shoulder" }` and `BalanceNotModelled`. **This is
R-0066 AC4's "where the body must stay in balance, the solver keeps it in
balance", read as applying only to joints whose moment depends on balance.
The owner decides it (§5, OQ-C).**

The upper-arm angle `upper_arm_deg` is positive with the elbow forward of the
shoulder (shoulder flexion). The dumbbell, or the cable's attachment, sits
at the hand's Winter COM, so the forearm-and-hand lever is `L_fh = forearm +
hand COM distance`. Both are stated in the result.

**Dumbbell curl (`dumbbell_curl`, free weight).**
- Tree: shoulder (root) → upper arm (revolute, its angle a parameter) → elbow
  (revolute, the motion) → forearm and hand.
- Loads: the upper arm, forearm and hand as `Weight`s at their Winter COMs;
  the dumbbell at the hand COM.
- Parameters:
  - `upper_arm_deg` (motion, −20..=45, default 0);
  - `elbow_extended_deg < elbow_flexed_deg` (motion, `5..=60` and
    `60..=150`, defaults 10 and 140); frames run from extended to flexed;
  - `dumbbell_kg` (load, 0..=100, default 10).
- Reported: elbow and shoulder.
- Nudged: the grip point along the hand (±0.02 m).
- Oracles (literals for the standard body, 1.75 m and 75 kg, beside the
  arithmetic):
  - upper arm vertical, forearm horizontal, 10 kg:
    `τ_elbow = −(10·9.81·L_fh + m_f·9.81·d_f + m_h·9.81·L_fh)`, which is
    negative, so the flexors resist;
  - forearm hanging: `τ_elbow = 0`.

**Triceps pushdown (`triceps_pushdown`, cable).**
- Tree: as the curl.
- Loads: the segment `Weight`s, and a `Cable` from the hand COM to the
  pulley, with tension `stack_kg · 9.81`.
- Parameters:
  - `upper_arm_deg` (motion, −30..=30, default 0);
  - `elbow_extended_deg < elbow_flexed_deg` (motion, `0..=40` and
    `60..=130`, defaults 5 and 100); frames run from flexed to extended (the
    press);
  - `pulley_height_m` (machine, 1.5..=2.6, default 2.20, from the floor;
    the shoulder sits at the standard body's shoulder height);
  - `pulley_ahead_m` (machine, −0.3..=0.8, default 0.10, ahead of the
    shoulder);
  - `stack_kg` (load, 0..=100, default 20).
- Reported: elbow and shoulder.
- Nudged: the grip point (±0.02 m).
- Oracles:
  - forearm horizontal, pulley straight above the hand: `τ_elbow = T·L_fh`
    plus the segment terms (the weights tend to extend the elbow, so they
    carry the opposite sign);
  - **the cable's direction**: the pulley placed so the cable makes a stated
    angle φ with the forearm gives `τ = T·L_fh·sin φ` plus the segment terms.
    This fails if the cable is treated as a weight.

### 2.6 The lat pulldown (slice 66-d)

- **Tree:** pelvis (root: seat and thigh pad) → L5/S1 (revolute, placed by
  SPEC-0045 §2.5) → thorax (its lean a parameter) → shoulder (revolute) →
  upper arm → elbow (revolute) → forearm and hand. The hip is not reported,
  because the seat pushes on the pelvis and the distal rule would be wrong
  there. The model carries `RigidTrunkLumbar` (SPEC-0045) and
  `FixedRoot { at: "pelvis" }`.
- **Motion:** the hand moves on a straight path in the plane, from
  `hand_top`, at full reach along the cable line, to `hand_bottom`, the
  upper chest (a stated point on the thorax). The shoulder and elbow are
  solved by `two_link_joint` with the elbow behind the bar. garust clamps an
  unreachable target, so `pulldown.rs` checks the reach itself. A hand
  beyond `upper_arm + forearm` ends the range as `stopped { reason:
  OutOfReach { frame } }`.
- **Loads:** the thorax, head and neck and the arms as `Weight`s, and a
  `Cable` from the hand COM to the pulley.
- **Parameters:**
  - `trunk_lean_deg` (motion, 0..=40, default 10, positive leaning back,
    away from the toes);
  - `pulley_height_m` (machine, 1.8..=2.6, default 2.30, from the floor; the
    seat is at 0.45 m);
  - `pulley_ahead_m` (machine, −0.2..=0.4, default 0.20, ahead of the hip);
  - `stack_kg` (load, 0..=150, default 40).
- **Reported:** shoulder, elbow and lumbar, plus the pelvis's reaction.
- **Nudged:** L5/S1 (±0.03 m) and the grip (±0.02 m).
- **Oracles:**
  - arm straight, horizontal forward, cable vertical: `τ_shoulder = T·L_arm`
    plus the arm's weight term (opposite sign); `τ_elbow` = the forearm and
    hand weight terms only;
  - the cable's direction, as for the pushdown;
  - the reaction: the pelvis's reaction equals minus the sum of the loads,
    computed by hand.

### 2.7 The guided machines (slice 66-e)

**The assumption, stated once.** The sled or carriage moves on a rail at α
to the horizontal. The force between the feet and the platform acts **along
the rail, through the centre of pressure `cop`** on the foot (`heel`, `mid`
or `toes`, at 0.15, 0.50 and 0.85 of the foot's length from the heel). The
seat or pad takes everything else. Rail friction opposes the motion: on the
press (`phase: up`) the platform force is `W_along + μ·N`, and on the descent
(`phase: down`) it is `W_along − μ·N`. `N` is the rail's normal force on the
carriage. `μ` defaults to 0 (frictionless) and ranges `0..=0.35`, after NASA
RP-1228 as the reel cites.

**A rail-fixed frame.** The rail direction `û` points up the rail. The
platform is the plane perpendicular to `û`. Its centre is the origin of the
machine's geometry. The hip is placed by two settings: `hip_offset_m`, its
distance from the rail line through the platform centre, perpendicular to
it; and the platform's travel `s` along `û`, which is the motion. The foot's
position on the platform is `foot_height_m`, along the platform's plane.

**45° leg press (`leg_press_45`).**
- Tree: pelvis (root, the seat) → hip (revolute) → thigh → knee (revolute) →
  shank → ankle (revolute) → foot. `side = +1` throughout (rooted at the
  pelvis, the distal side is anatomical).
- Motion: knee angle from `knee_extended_deg` to `knee_flexed_deg`. At each
  frame `s` and the thigh angle are solved by the two-circle intersection
  (hip fixed, sole on the platform), knee on the side away from the rail.
  When no intersection exists the range ends as `NoLegSolution { frame }`.
- Loads: the leg segments as `Weight`s, and the platform force
  `kernel::force` at the foot's `cop`, of magnitude `(sled_kg + plates_kg)·g·
  sin α ± μ·(sled_kg + plates_kg)·g·cos α`, along `−û`. The legs are not on
  the sled, so their weight is carried as segment `Weight`s and not in that
  force.
- Parameters:
  - `rail_deg` (machine, 30..=60, default 45);
  - `hip_offset_m` (machine, 0.0..=0.40, default 0.20);
  - `foot_height_m` (machine, −0.15..=0.15, default 0);
  - `cop` (machine, choice, default `mid`);
  - `phase` (motion, choice `up | down`, default `up`);
  - `friction` (machine, 0..=0.35, default 0);
  - `sled_kg` (machine, 0..=150, default 50, a generic machine's sled);
  - `plates_kg` (load, 0..=500, default 100);
  - `knee_extended_deg < knee_flexed_deg` (motion, `5..=60` and `60..=150`,
    defaults 15 and 100).
- Reported: hip, knee and ankle; the platform force and the seat's reaction.
- Nudged: `cop` (±0.02 m along the foot).

**Hack squat (`hack_squat`).**
- Same frame and tree. The platform is fixed and the pelvis rides the
  carriage on the pad.
- **The force along the rail, from a free body.** The system is the carriage,
  the plates and the whole body except the feet, which rest on the fixed
  platform. Along the rail its external forces are gravity, rail friction and
  the platform force. So the force is `(carriage_kg + plates_kg + M −
  m_feet)·g·sin α ± μ·N`, where `N = (carriage_kg + plates_kg + m_upper)·g·
  cos α` is what the pad presses into the rail, and `m_upper` is the pelvis,
  trunk, head and arms.
- Parameters: as the leg press, with `carriage_kg` (machine, 0..=150,
  default 60) in place of `sled_kg`. `hip_offset_m` is the pad's distance
  from the rail line.
- Reported: as the leg press, plus the carriage's reaction.

**Oracles for both:**
- **Zero lever**: a posture where the force line passes through the knee
  gives `τ_knee` = the shank and foot weight terms only. This is the test
  that the platform force is along the rail.
- **Hip lever**: the force line a stated distance d from the hip gives
  `τ_hip = F·d` plus the segment terms.
- **The force sum** (both machines): the reported platform force equals the
  free-body total above, computed by hand for the standard body. This is the
  oracle that catches a missing mass.
- **`cop`**: moving `cop` from heel to toes, with everything else fixed,
  moves load from the hip to the knee. Both values are literals for one
  posture.
- **Friction**: `phase: up` minus `phase: down` equals `2μN`, by hand.

### 2.8 Slicing

| Slice | Content | Depends on | Size (est.) |
|---|---|---|---|
| **66-0** | garust (own repo, own PR): `generalized_torque` holds the forces, cable agreement test; release the rev | — | ~80 |
| **66-a1** | garust dependency; `kernel.rs` and its tests (sign table, invariant, cross-check, reaction); `squat::tree`; dense parity against `chain.rs`, both paths alive; parity fixture; SPEC-0045 slice E row amended | 66-0 | ~400 |
| **66-a2** | the switch: `analysis.rs` corners, `lever.rs`, oracle tests and example on the kernel; parity against the fixture; moment functions deleted | 66-a1 | ~350 |
| **66-b1** | `core::exercise`: trait, `Description`, `Parameter`, pair rules, generic validation, `ExerciseRange`, `Reason`; `back_squat`; `MeasuredLengths` arms; `model_for` from the library | 66-a2 | ~450 |
| **66-b2** | api: the three endpoints, request validation, serialization, the new-vs-old squat test | 66-b1 | ~350 |
| **66-c** | `dumbbell_curl`, `triceps_pushdown`, their oracles and sign tests | 66-b1 | ~350 |
| **66-d** | `lat_pulldown`: two-link motion, reach check, L5/S1, reactions, oracles | 66-c | ~350 |
| **66-e** | `leg_press_45`, `hack_squat`: rail frame, leg solve, platform force, friction, oracles | 66-b1 | ~450 |

Every slice is a PR with tests first (red), the architect's review and the
owner's merge. The mobile side (the app accepting new `physics` ids, the
screen) is SPEC-0067's, with the Flutter gates.

## 3. Code outline

```
garust/crates/garust-physics/src/multibody.rs   generalized_torque holds F     [66-0]
backend/crates/core/src/biomech/
  kernel.rs      PlanarTree, Reported{side}, LoadSpec, force(), moments(),
                 root_reaction(), flexion_dir() — the ONLY garust importer    [66-a1]
  squat.rs       + tree(body, posture, placement)                              [66-a1]
  analysis.rs    stated + corners through kernel::moments                     [66-a2]
  chain.rs       ChainSegment + com_with only (moment functions deleted)      [66-a2]
  tests/         parity (dense grid, fixture), sign pairs, oracles on kernel  [66-a1/a2]
backend/crates/core/src/exercise/
  mod.rs          ExerciseModel, Equipment, Reason, library()                 [66-b1]
  description.rs  Description, Parameter{Number|Choice}, Ordered, Group       [66-b1]
  input.rs        Input, validate()                                           [66-b1]
  range.rs        ExerciseRange, End, Frame, named points, reactions          [66-b1]
  back_squat.rs                                                               [66-b1]
  curl.rs, pushdown.rs                                                        [66-c]
  pulldown.rs                                                                 [66-d]
  guided.rs (rail frame, leg solve, platform force), leg_press.rs, hack_squat.rs [66-e]
backend/crates/api/src/exercise/  handlers.rs, request.rs, response.rs        [66-b2]
backend/crates/api/src/biomech/lever.rs   body_nm from the kernel              [66-a2]
```

## 4. Non-goals

- The interactive screen and every mobile change: SPEC-0067.
- Lever and cam machines (R-0066 OQ-3): the `Equipment` variant exists, and
  its first entry waits for a measurable machine. Also out: frontal-plane and
  3D exercises, and acceleration terms.
- Saving anything. The deadlift and bench join through R-0062 and R-0063.

## 5. Open questions

- **OQ-A. Machine defaults.** The machine defaults are illustrative and the
  result says so (`GenericMachine`). Proposal: accept as illustrative; R-0066
  OQ-2 already chose this.
- **OQ-B. The squat's old endpoint.** Keep it through SPEC-0067 and remove it
  there. Proposal: yes.
- **OQ-C. Balance for the curl and the pushdown (owner).** §2.5 reads R-0066
  AC4 as applying only to joints whose moment depends on balance, and states
  `BalanceNotModelled`. Proposal: amend AC4's wording to say so, rather than
  solving whole-body balance for exercises that never report a balance-bound
  joint.
- **OQ-D. The garust change (owner).** 66-0 changes the owner's library
  (`generalized_torque`), in its own repository and PR, before 66-a1.
  Proposal: yes. It fixes a real error for every non-constant load, not only
  fitAI's.

## 6. Acceptance criteria (this spec)

- **66-0:** garust's existing tests unchanged and green; the cable agreement
  test green; the pin bumped with fitAI's tests green.
- **66-a1:** parity ≤ 1e-9 N·m on the dense grid and the placement corners,
  both paths alive; the FK invariant; every squat sign pair; `cargo tree`
  shows garust and no I/O crate.
- **66-a2:** parity against the fixture ≤ 1e-9 N·m; the slice-A oracles and
  the range tests green on the kernel with their tolerances unchanged;
  `bar_nm + body_nm == hip` to 1e-9; the squat endpoint's e2e unchanged; the
  moment functions gone.
- **66-b1/b2:** the generic validation (unknown key, out of range, bad
  choice, broken pair, missing key defaulted); the three endpoints with
  `400`/`401`/`404`, a `stopped` and a `rejected` end; the squat through the
  new endpoint equal to the old within 1e-9 N·m; the plan still emitting
  `"squat"`.
- **66-c to 66-e:** per exercise, the oracles of §2.5–2.7, every
  (exercise, joint) sign pair, the AC5 cross-check on every test posture and
  at the largest load, each parameter at both ends of its range, one out of
  range rejected, and the default posture's golden values.
- **All:** `cargo fmt`, `clippy -D warnings` and `cargo test --workspace`
  green.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-11-21 | The squat moves onto garust first, in two slices: parity with both paths alive, then the switch. | ADR 0001; one engine. The architect: a parity gate proven only in a state CI never sees is no gate. |
| 2026-11-21 | `generalized_torque` is fixed in garust, not worked around in fitAI. | The cross-check is impossible for a cable otherwise. A virtual-work formula in fitAI would be a moment rule outside the kernel (ADR 0001 §1). |
| 2026-11-21 | One sign convention for every joint: positive when the loads tend to flex it; `resisting` follows the sign; garust's `z` is mapped by joint × tree side. | The architect: a per-joint table gives the wrong sign as soon as a tree has a different root. |
| 2026-11-21 | Guided machines: the platform force acts along the rail through the foot's centre of pressure; `cop`, friction and the phase are settings; the hack squat's force comes from a stated free body. | The machine is indeterminate otherwise. A stated assumption the lifter can move stays checkable. |
| 2026-11-21 | The plan keeps `"squat"` as the squat's `physics` value; new ids reach the plan only with SPEC-0067. | Installed app builds must not lose the squat's *Física* button, nor gain buttons that open nothing. |
| 2026-11-21 | The five first exercises: dumbbell curl, triceps pushdown, lat pulldown, 45° leg press, hack squat. | Owner asked for five. They cover free weight, cable and guided rail, from one joint to three. |

## 8. Changelog

- 2026-11-21 — created (Draft).
- 2026-11-21 — revision 2: the architect's REJECT applied (§9).

## 9. Architect review — REJECT (2026-11-21), applied in revision 2

| # | Finding | Where applied |
|---|---|---|
| 1 | `generalized_torque` wrong for a cable at the pinned rev | §1, §2.1.1, slice 66-0, OQ-D |
| 2 | A per-joint sign table breaks with another root; upper-body conventions undefined | §2.1.3 |
| 3 | 66-a ignored `chain.rs`'s dependents; the HAT statement was wrong | §2.2 |
| 4 | The refusal shape could not carry the squat's `stopped` end | §2.3 `End`, `Reason` |
| 5 | The hack squat's force left out the legs | §2.7 free body, force-sum oracle |
| 6 | AC4: reactions not reported; balance deviation implicit | §2.3 `reactions`, §2.5, OQ-C |
| 7 | Guided geometry underdetermined; `seat_back_deg` a no-op; knee bounds; friction phase | §2.7 rail frame, `phase`, bounds; `seat_back_deg` dropped |
| 8 | The parameter schema could not express choices, pairs, missing keys, arm lengths | §2.3 |
| 9 | Parity did not survive its PR; slices too big | §2.2 a1/a2, §2.8 |
| 10 | Pulldown root and lumbar | §2.6 pelvis root, L5/S1 |
| 11 | Preacher remark; signs and lever definitions | §2.5 |
| 12 | Uncertainty per exercise undefined | §2.3, per model "Nudged" |
| 13 | Tolerance at heavy loads; cross-check failure handling | §2.1.2 |
| 14 | `Push` needs a crate-private helper | §2.1.2 `kernel::force` over `Weight` |
| 15 | Revolute axes through the frame origin | §2.1.2 invariant |
| 16 | The squat's `physics` wire value | §2.3, §7 |
| 17 | `&'static Description`, duplicate `id()`, features, `Assumption: Eq`, AC3 value, bar 0, SPEC-0045 row, the rail "direction oracle" | §2.3, §2.1.2, §2.2, §2.7 |
