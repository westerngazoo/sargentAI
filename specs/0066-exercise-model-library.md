# SPEC-0066 — The exercise model library on the garust kernel

- **Status:** Accepted (owner, 2026-11-21) — architect ACCEPT (§11); OQ-C and OQ-D answered yes
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
   gains a spurious `−x·∂F/∂q` term. The main session's reproduction: one
   link pivoting at the origin, the cable attached 0.3 m along it at angle
   `q = 0.4` from `+x`, 200 N toward a pulley at `(0.3, 2.0)`. The cross
   product gives 54.97 N·m, virtual work with `F` varying gives 58.48 in
   magnitude, and virtual work with `F` held at the nominal posture gives
   54.97. (The architect's own setup, in garust's rotation sense, gave 55.52
   against 52.54.) The new garust test's literals are the reference. AC5's cross-check could never pass
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
τ_j = −Σ_loads F(q) · (x(q + h·e_j) − x(q − h·e_j)) / 2h
```

The minus is garust's existing sign (`W = −F·x`), which makes the result
equal `joint_torque`'s `τ_z`, as today. This is the principle of virtual work
as stated: the forces are held, the displacement is virtual. For a constant
force it is identical to today's formula, so every existing garust test still
passes unchanged. The change adds a cable agreement test (`joint_torque`
against `generalized_torque` for a `Cable` at three pulley positions, with
literals).

- **garust's process.** 66-0 follows garust's own flow (its PM ticket and PR,
  since `e518fc4`), not fitAI's.
- **The new rev must be on the GitHub remote** fitAI's `Cargo.toml` points to,
  or public CI cannot fetch it.
- **The bump carries more than the fix.** garust has 34 commits after
  `55e91407`, including fixes in `world.rs` and `contact.rs`. fitAI's parity
  tests cover fitAI's paths. ADR 0001 requires them green before the pin
  moves, and its consequences record the bump, the commit range and the
  parity run that justified it.

#### 2.1.2 `biomech/kernel.rs` — the only module that imports garust

```rust
/// A planar tree for garust: links parent-first, which reported joint each
/// link is, and which anatomical side is distal at that joint in this tree.
pub struct PlanarTree {
    links: Vec<TreeLink>,           // revolute about z, axis through the link's frame origin
    reported: Vec<Reported>,        // { link, joint, distal: Side }
    loads: Vec<LoadSpec>,           // Weight { link, at, kg } | Force { link, at, newtons } | Cable { link, at, pulley, newtons }
}

/// Signed moments (§2.1.3) at every reported joint, each cross-checked by
/// virtual work. The caller adds `resisting` and the uncertainty.
pub fn moments(tree: &PlanarTree) -> Result<Vec<(Joint, NewtonMetres)>, KernelError>;

/// The force and moment the root takes: minus the sum of every applied load.
pub fn root_reaction(tree: &PlanarTree) -> Reaction;   // arrives with 66-d
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
- **The dependency:** `garust-core`, `garust-geo` and `garust-physics` by git,
  pinned by rev (no umbrella crate, so no `physics` feature to forget).
  `garust-core` is needed for `Pga3`, the joint axis: `garust-geo` re-exports
  only `Motor` and `Conformal`. `cargo tree -p
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
- `flexion_dir(joint)`: the rotation sense that flexes the joint when the
  distal anatomical segment moves, **in the right-hand sense of
  `joint_torque`'s `τ_z`**, not garust's `q` sense (the two agree only
  because two sign flips cancel). One row per joint, derived once from the
  side view. **Every tree has the lifter facing `+x`.**
- `side`: `+1` when the tree's distal side at that link is the anatomical
  distal segment (rooted at the shoulder, the elbow's distal side is the
  forearm), `−1` when it is the proximal one (rooted at the foot, the knee's
  distal side is the thigh and everything above).

Each `PlanarTree` carries `side` per reported link. **A test pins every
(exercise, joint) pair** with a posture whose sign is obvious by inspection
(a held dumbbell loads the curl's elbow flexors; a platform pushing ahead of
the knee loads the leg press's knee extensors).

**The enums grow** (66-a2 for the squat's, 66-b1 onward for the rest):
- `Joint` gains `Shoulder` and `Elbow`.
- `ResistingGroup` gains `Dorsiflexors`, `KneeFlexors`, `HipFlexors`,
  `Abdominals`, `ShoulderFlexors`, `ShoulderExtensors`, `ElbowFlexors` and
  `ElbowExtensors`.
- The squat's `Mechanics.joints: [JointMoment; 4]` keeps its shape.

**The squat's wire changes in one field.** Today `resisting` is fixed per
joint (`analysis.rs:273`). Under one rule it follows the sign, so a squat
frame whose knee moment is negative now reads `knee_flexors`, where SPEC-0059's
example (`moment: -71.49`) reads `knee_extensors`. The moments are unchanged
to 1e-9. 66-a2 makes this change, amends SPEC-0059's example and golden, and
updates the endpoint's e2e. The app parses `resisting` as a string and never
shows it (`squat_range.dart:166`), so no installed build is affected. One rule
for every exercise, rather than two.

### 2.2 The squat on the kernel (slices 66-a1, 66-a2)

**What stays.** `ChainSegment` stays, possibly renamed, as the squat's list
of lumped segments and their placements (`com_with(body, p, Placement)`). It
is the single source for both the balance solve (`squat.rs`, which relies on
`Σ m·x` being affine in the sines, `chain.rs:160-170`) and the tree builder.
`Body::mass_kg(ChainSegment)` stays. What goes are the moment functions:
`above`, `moment_about`, `moment_about_with`, `body_moment_about`,
`segments_sum`, `load_sum`, `raw_sum`, `extension_sign`.

**The tree** (`squat_tree(body, posture) -> PlanarTree`, and
`tree_with(body, posture, placement)` for the uncertainty corners):
- Four links, **rooted at the ankle** (the foot is not a link): ankle → knee
  → hip → L5/S1 (placed by `placement.l5s1`, SPEC-0045 §2.5). Each link is
  the segment its joint rotates: the ankle's carries the shank, the knee's
  the thigh, the hip's the pelvis, L5/S1's the thorax. A foot link, for a
  ground reaction with a meaningful moment, is 66-d's to decide when
  `root_reaction` arrives.
- Each `ChainSegment` is a `Weight` at its `com_with` point on the link that
  `above()` puts it on today: shank on the shank, thigh on the thigh, pelvis
  on the pelvis link (on the hip's side, not the lumbar's), and thorax-abdomen,
  head and neck, arms and the bar on the thorax. **The head and neck sit on
  the trunk line beyond the shoulder, and the arms at the shoulder-to-bar
  midpoint or hanging**, exactly as `com_with` places them. Nothing is lumped
  into a HAT weight.
- `side = −1` at the ankle, knee and hip (rooted at the foot, the tree's
  distal side is the anatomical proximal one), and `side = +1` at L5/S1: its
  tree-distal side is the thorax, which is the joint's anatomical distal
  segment. *(Corrected in 66-a1: revision 4 said −1 at every joint, which
  would have flipped the lumbar sign. Found while writing 66-a1; the sign test pins it.)*

**66-a1: both paths alive.**
- Adds the dependency, `kernel.rs`, its unit tests, and `squat_tree`.
- **The pin.** By the owner's call of 2026-11-21 ("arranca ya"), 66-a1 starts
  before 66-0 at the ADR rev `55e91407`. The squat uses only weights, which
  that rev handles correctly. The pin moves to 66-0's rev before 66-c, with
  this slice's parity tests re-run green (ADR 0001).
- **Trees are built at a posture** (`q = 0`): each link's frame sits at its
  joint's centre, so `moments` and `root_reaction` take no `q`. `force`,
  `Cable` and `root_reaction` arrive with the first slice that uses them
  (66-c to 66-e), so no unused code lands early.
- Adds a parity test: every posture of a dense grid (the spike's four bodies,
  four bars, three dorsiflexions, every frame of each range: 48 cases, 1 916
  postures, the rest not reaching depth) and all eight `Placement` corners of
  one lifter's range, through both paths, asserting `|kernel − chain| ≤ 1e-9
  N·m` at every joint.
- The same PR snapshots the kernel's values over that grid at full precision
  into a fixture (`tests/fixtures/squat_kernel_parity.json`). That is what
  survives `chain.rs`. The fixture is generated (1 916 postures × 4
  joints, 148 kB), excluded from the line budget, and marked as regenerable only with
  both paths alive. After 66-a2 that means never.
- Amends SPEC-0045's slice E row: recast by ADR 0001 as "the squat on the
  garust kernel", realized here.
- Nothing else changes, and the endpoints are untouched.

**66-a2: the switch.**
- `analysis.rs`: the stated moments and the 8 uncertainty corners call
  `kernel::moments(&tree_with(body, p, corner))`, one tree per corner, as
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
- The moment functions are deleted. What is left of `chain.rs`
  (`ChainSegment`, `com_with`) is renamed `segments.rs`, and the module docs
  (`mod.rs:20–27, 63–66`, which say `chain` is the only place gravity is
  multiplied) now name `kernel`.
- `resisting` follows the sign (§2.1.3). `POST /biomech/squat/range` answers
  as before except for that field, with the moments equal to 1e-9. Its e2e
  test and SPEC-0059's example are updated in the same PR.

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
  description. An unknown key is `400` with that key, and so is a `lengths`
  key the exercise does not use. A missing key takes its
  default. A number outside `min..=max` or a choice not in `options` is `400`.
  A broken pair rule is `400`.
- **`ExerciseRange`**: `{ end, frames, settings, table_lengths, assumptions,
  reactions? }`.
  - `end` generalizes SPEC-0059's: `complete`, `stopped { reached, reason }`
    (a prefix of frames that the model stands behind, then the reason), or
    `rejected { reason }` (no frames).
  - `Reason` is a closed library enum: the squat's `Rejection` wrapped
    unchanged, `ConventionPassesHorizontal` (the squat's `Stopped { bottom:
    None }`: the bottom balances but this convention passes horizontal on the
    way down), `OutOfReach { frame }`, `NoLegSolution { frame }`. A 66-b1 test
    covers the `ConventionPassesHorizontal` mapping.
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
- **Oracles test the production tree.** Each model has a `pub(crate)`
  builder, `tree_at(body, values, angles) -> PlanarTree`, from explicit
  angles. `range()` uses it for every frame, and the oracles call it for
  postures the motion convention never produces (an arm straight and
  horizontal, a force line through the knee).
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
    the shoulder's height comes from the lifter's scaled body, upright:
    ankle height + shank + thigh + trunk, the same segment sums the squat's
    upright uses, so "my body" moves the cable's angle too);
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
- **Motion:** the hand (its Winter COM, where the cable attaches) moves on a
  straight path in the plane, from `hand_top` to `hand_bottom`.
  - `hand_top` lies on the line from the shoulder to the pulley, at 0.98 of
    reach. Full reach is singular for `two_link_joint` (its own doc says so).
  - `hand_bottom` is the upper chest, a stated point on the thorax.
  - The shoulder and elbow are solved by `two_link_joint` with `l1 =
    upper_arm` and `l2 = L_fh` (forearm + hand COM distance; the hand is
    rigid beyond the forearm), the elbow hint `−x̂` (behind the bar).
  - garust clamps an unreachable target, so `pulldown.rs` checks the reach
    itself, with a stated tolerance of 1e-9 m. A hand beyond `upper_arm +
    L_fh` would end the range as `stopped { reason: OutOfReach { frame } }`.
    **That variant is unreachable under the stated motion.** Both ends lie
    inside the reach disc, the disc is convex, and the inner bound `|l1 − l2|`
    (about 0.02 m) is far smaller than the shoulder-to-chest distance. It is
    kept as a guard and documented as unreachable, as SPEC-0045 documents
    `AnatomicallyImplausible`.
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

**The foot, a stated convention.** Winter's foot row runs from the ankle to
the second metatarsal head and gives no heel point, so the foot's geometry is
a convention with its own spread. It reuses the squat's (SPEC-0045 §2.6.1)
rather than adding a second one:
- the foot's length, heel to toe, is Winter's `0.152 H`;
- the ankle sits `ANKLE_BEHIND_MID_FOOT` (0.26 of the foot) behind the
  mid-foot, so 0.24 of the foot's length forward of the heel;
- on the platform the sole is flat on it. The ankle lies `ankle_height`
  (`WINTER.ankle_height`) off the platform along `−û`, toward the lifter. The
  toes point along `+n̂`.
- `foot_height_m` places the **ankle's** projection on the platform, along
  `n̂` from the platform centre.

The `cop` nudge (±0.02 m) carries this convention's error. The result states
it as `Assumption::FootGeometryConvention`.

**A rail-fixed frame.**
- `û` points up the rail, and `n̂` is `û` rotated +90° in the side view,
  toward the lifter's chest.
- The platform is the plane perpendicular to `û`. Its centre is the origin of
  the machine's geometry.
- The hip sits at `−hip_offset_m · n̂` from the rail line through the platform
  centre, with the platform's travel `s` along `û` as the motion.
- The foot's position on the platform is `foot_height_m`, along `n̂` (the
  ankle's projection, as above).
- The leg solve passes `+n̂` to `two_link_joint` as the knee's side, toward
  the chest.
- **No ankle limit is applied.** A flat foot fixes the ankle angle, and deep
  knee flexion can ask for more dorsiflexion than a lifter has. The result
  says so (`Assumption::NoAnkleLimit`). A cited limit would become a `Reason`
  later.

**45° leg press (`leg_press_45`).**
- Tree: pelvis (root, the seat) → hip (revolute) → thigh → knee (revolute) →
  shank → ankle (revolute) → foot. `side = +1` throughout (rooted at the
  pelvis, the distal side is anatomical).
- Motion: knee angle from `knee_extended_deg` to `knee_flexed_deg`, solved
  in two named steps per frame.
  1. **Circle–line.** The knee angle fixes the hip–ankle distance `d` (law of
     cosines over thigh and shank). The ankle moves on a line parallel to `û`
     (the platform's travel). So `s` is the intersection of that line with
     the circle of radius `d` about the hip. Of its two roots, take the one
     with the platform up the rail from the hip. With no root, the range ends
     as `NoLegSolution { frame }`.
  2. **Two-circle.** With hip and ankle placed, `two_link_joint` gives the
     knee, on the `+n̂` side.
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
  the plates and **the whole body**. With the whole body inside, the platform
  force is external to it. Along the rail the external forces are gravity,
  rail friction and the platform force:
  - `F = (carriage_kg + plates_kg + M)·g·sin α ± μ·N`;
  - `N = (carriage_kg + plates_kg + M)·g·cos α`. The platform force has no
    component perpendicular to the rail (the assumption above), so the whole
    system's perpendicular weight, legs included, reaches the rail through
    the pelvis and the pad.
- Parameters: as the leg press, with `carriage_kg` (machine, 0..=150,
  default 60) in place of `sled_kg`. `hip_offset_m` is the pad's distance
  from the rail line.
- Reported: as the leg press, plus the carriage's reaction.

**Oracles for both:**
- **Zero lever**: a posture where the force line passes through the knee
  gives `τ_knee` = the shank and foot weight terms only. This is the test
  that the platform force is along the rail.
- **Hip lever**: the force line a stated distance d from the hip, passing on
  the `+n̂` side of it (the chest side), gives `τ_hip = F·d` plus the segment
  terms. The force tends to flex the hip there, so the moment is positive and
  the hip extensors resist (§2.1.3). The sign is checkable by inspection.
- **The force sum** (both machines): the reported platform force `F` and the
  rail's normal force `N` equal the free-body totals above, computed by hand
  for the standard body. This is the oracle that catches a missing mass.
- **`cop`**: for one stated posture P, the hip and knee moments with `cop` at
  `heel` and at `toes`, all four computed by hand, as literals. The spec does
  not pre-state which joint gains. The hand computation decides, and the
  model must agree with it.
- **Friction**: `phase: up` minus `phase: down` equals `2μN`, by hand.

### 2.8 Slicing

| Slice | Content | Depends on | Size (est.) |
|---|---|---|---|
| **66-0** | garust (own repo, own PR): `generalized_torque` holds the forces, cable agreement test; release the rev | — | ~80 |
| **66-a1** | garust dependency; `kernel.rs` (weights, `moments`, the sign mapping) and its tests (signs, FK invariant, cross-check, heaviest bar); `squat_tree`; dense parity against `chain.rs`, both paths alive; parity fixture; SPEC-0045 slice E row amended | — (owner: before 66-0, pinned at `55e91407`) | ~400 |
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
  kernel.rs      PlanarTree (links, weights, reported joints with Side),
                 moments(), flexion_dir() — the ONLY garust importer          [66-a1]
                 + Cable                                                      [66-c]
                 + root_reaction()                                            [66-d]
                 + force()                                                    [66-e]
  squat.rs       + squat_tree(body, posture), tree_with(.., placement)        [66-a1]
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
  joint. *Answered 2026-11-21 (owner):* yes; R-0066 AC4 amended.
- **OQ-D. The garust change (owner).** 66-0 changes the owner's library
  (`generalized_torque`), in its own repository and PR, before 66-a1.
  Proposal: yes. It fixes a real error for every non-constant load, not only
  fitAI's. *Answered 2026-11-21 (owner):* yes; the fix is garust BUG PM-20 (project "Goose Garust library", sprint "sprint doc").

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
| 2026-11-21 | Accepted. OQ-C: AC4 amended. OQ-D: the garust fix goes ahead as PM-20. | Owner. |

## 8. Changelog

- 2026-11-21 — created (Draft).
- 2026-11-21 — revision 2: the architect's REJECT applied (§9).
- 2026-11-21 — revision 3: the architect's ACCEPT WITH CHANGES applied (§10).
- 2026-11-21 — revision 4: the architect's ACCEPT; its five text minors applied (§11).
- 2026-11-21 — Accepted by the owner; OQ-C and OQ-D answered.
- 2026-11-22 — 66-b1 as built (owner approved the outline; architect ACCEPT WITH CHANGES on PR #196, applied):
  - `range(&self, body, values, frames)` takes the validated settings and the scaled body directly (no `Input` struct). It returns `Result<_, LibraryError { Invalid, Kernel }>`, so settings validated against another description are a typed error, not a panic. 66-b2 maps `Invalid` there to `500`: after validation it is a programming error.
  - `validate` returns `Validated { settings, lengths }`, kept apart so `settings` restates exactly what the model used. Only `validate` builds `Values`. The settings serialize as one flat object, and points as `[{name, at}]`.
  - A repeated key, or an unknown one, is `400` with field `values` or `lengths`. The API's field is a fixed token, and request text is never echoed. A known length the exercise does not use is named.
  - **A parameter's `default` is optional; `None` makes it required.** It is allowed only with a cited reason, never for a machine setting. Its one use is the squat's `dorsiflexion_deg`: SPEC-0045 §2.6.1.1 commits no uncited population default. **This conflicts with R-0066 AC1/AC2 and R-0067 AC1/AC5 (a default for every control): pending the owner's decision, see below.**
  - `served_in_plan` became `in_plan_as: Option<Lift>`, the value the plan emits (it keeps `"squat"`). When SPEC-0067 exposes a second exercise, the plan's `physics` becomes the library id or a string, because `technique::Lift` names the lifts a video shows, not every modelled exercise.
  - `model_for` moved to `exercise`, so `biomech` does not depend on the library.
  - `Reason` is tagged `source`, because the squat's `Rejection` is tagged `reason` and one object must not repeat a key. Its other variants arrive with their slices.
  - Each frame carries its own assumptions, as `squat_range` does.
  - **Deferred to 66-b2's wire, and to be specified there:** `table_lengths`; the per-exercise uncertainty spreads (the squat's `Uncertainty`); and **solved angles per frame**. R-0067 OQ-2 makes solved angles read-outs, and R-0059's screen shows them, but §2.3's `Frame` has no place for them yet.
- **Open, for the owner (66-b1):** the squat's dorsiflexion default. The options:
  - (a) cite a normative weight-bearing value (SPEC-0045 names Bennell 1998 and a normative-range paper as the candidates) and make it the default;
  - (b) amend R-0067 AC1/AC5: a control with no cited default is asked once, as SPEC-0059 does, and has no reset;
  - (c) move dorsiflexion to the body, since it is a measurement of the lifter, so the standard body carries it (which still needs (a)'s citation for the standard body's value).
- 2026-11-21 — 66-a2 as built (owner approved the outline): the error type is `ModelError { Rejected, Kernel }`, shared by `squat_range` and `baseline_mechanics` (the outline called it `RangeError`); `joint_moment` is public beside `body_moment`, for postures that did not come from the solver (the reel check); `above()` gives way to `ChainSegment::ALL` (the balance solve's list) and `tree_with`'s links (the load sides), checked by hand sums per joint; `KernelError::NotReported` for a joint the tree does not report; the fixture's writer is removed.
- 2026-11-21 — 66-a1 as built: the pin stays at `55e91407` until before 66-c (owner); the lumbar `side` corrected to `+1`; trees built at a posture (no `q`); `force`, `Cable` and `root_reaction` deferred to the slices that use them; the grid is 1 916 postures; the squat tree is rooted at the ankle (no foot link); `garust-core` is a third dependency (architect review of PR #193).

## 10. Architect re-review — ACCEPT WITH CHANGES (2026-11-21), applied in revision 3

| # | Finding | Where applied |
|---|---|---|
| 1 | Sign-driven `resisting` changes the squat's wire; enum growth unlisted | §2.1.3 (option a: one rule; SPEC-0059's example amended in 66-a2), §2.2 |
| 2 | Hack squat free body: feet subtracted, `N` too small | §2.7: whole body in the system; `F` and `N` both from `carriage + plates + M` |
| 3 | The `cop` oracle pre-stated a direction | §2.7: four hand literals, no direction stated |
| 4 | No foot model | §2.7 foot convention, `FootGeometryConvention` |
| 5 | Pushdown shoulder height; leg-press side; ankle | §2.5 shoulder from the scaled body; §2.7 `n̂`, knee hint, `NoAnkleLimit` |
| 6 | 66-0 formula's minus; the check's citation | §2.1.1, §1 |
| 7 | The bump's size, garust's process, the remote | §2.1.1 |
| 8 | The squat's `Stopped { bottom: None }` | §2.3 `ConventionPassesHorizontal` |
| 9 | Pulldown `hand_top`, singular reach, `l2` | §2.6 |
| 10 | Oracle postures unreachable through `range()` | §2.3 `tree_at` |
| 11 | `moments()` return type, `flexion_dir` sense, facing `+x` | §2.1.2, §2.1.3 |
| 12 | Fixture, `chain.rs` rename, unused lengths key | §2.2, §2.3 |

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

## 11. Architect review — ACCEPT (2026-11-21), minors applied in revision 4

| # | Finding | Where applied |
|---|---|---|
| 1 | The foot convention disagreed with the squat's (0.25 vs 0.24 from the heel) | §2.7: reuses `ANKLE_BEHIND_MID_FOOT`, SPEC-0045 §2.6.1 |
| 2 | Foot orientation and reference point unstated | §2.7: toes along `+n̂`, ankle `ankle_height` off the platform along `−û`, `foot_height_m` places the ankle |
| 3 | The leg solve mixed two intersections | §2.7: circle–line for `s` (root up the rail), then two-circle for the knee |
| 4 | `OutOfReach` cannot fire | §2.6: kept as a guard, documented unreachable; elbow hint `−x̂` |
| 5 | The hip-lever oracle's sign | §2.7: force line on the `+n̂` side, positive, hip extensors |
| 6 | 66-a2's PR must name the `resisting` wire change | 66-a2's PR description (process, not spec text) |
