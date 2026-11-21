# SPEC-0066 — The exercise model library on the garust kernel

- **Status:** Draft — for architect review, then owner acceptance
- **Realizes:** R-0066 (Draft, 2026-11-21; acceptance pending with this spec)
- **Author:** Claude (main session), for owner review
- **Created:** 2026-11-21
- **Depends on:** ADR 0001 (garust as the kernel, rev `55e91407`), SPEC-0045
  (slices A and B1: the squat solve and its range), SPEC-0059 (the range
  endpoint, its validation table and wire shape)
- **Module(s):** `backend/crates/core/src/biomech/` (kernel glue, the squat
  move), `backend/crates/core/src/exercise/` (new: the library),
  `backend/crates/api/src/exercise/` (new: the endpoints),
  `backend/crates/core/src/biomech/model.rs` (`model_for` reads the library)

## 1. Motivation

R-0066: every modelled exercise is a tree on the garust kernel, with its
loads, its equipment and a motion convention, served by one endpoint. R-0067
then builds an interactive screen on top of it, with controls described by
the server. The owner asked to start with five exercises (2026-11-21).

Three facts shape this spec:

1. **garust is not wired in yet.** ADR 0001 decided the kernel, and the parity
   spike proved it (1.3e-12 N·m over 2 992 squat postures), but
   `fitai-core` still computes moments with its hand-written chain
   (`chain.rs`). A library of six exercises on a hand-written chain would be
   six chains. So the squat moves first, with every golden value unchanged
   (R-0045 AC17(c), R-0066 AC10). That is SPEC-0045 slice E as ADR 0001
   recast it, and it is slice 66-a here.
2. **garust already has what five exercises need.** `Tree` with `Revolute`
   and `Prismatic` joints, the loads `Weight` (a mass under a gravity vector)
   and `Cable` (a tension toward a fixed pulley), `joint_torque` (the moment of
   everything distal) and `generalized_torque` (the same by virtual work). The
   lever-or-cam kind needs a resistance profile garust does not have. No first
   exercise needs it (R-0066 OQ-3).
3. **Guided machines are statically indeterminate.** On a leg press or a hack
   squat, the seat or pad and the platform share the load in a way statics
   alone does not fix. §2.6 states the assumption that makes them
   determinate and makes it a setting, rather than hiding it.

## 2. Design

### 2.1 Coordinates, signs and the one kernel

The coordinates are SPEC-0045 §2.1's: sagittal plane, `+x` toward the toes,
`+y` up, metres. Every tree is built in garust's 3D space with `z` out of the
plane and every revolute joint about `z`. Nothing leaves the plane in v1
(R-0066 §4).

**Signs.** garust's rotation sign (positive about `+z` carries `+x` toward
`−y`) differs from the right-hand rule (ADR 0001). Reported moments keep
SPEC-0045 §2.4.1's convention: **resisting extension**, signed, never `.abs()`.
The mapping from garust's `z` component to that convention lives in one
function, `kernel::resisting(joint, tau_z)`, with one row per joint. It
replaces `Joint::extension_sign()` in `chain.rs`. A test pins every row with a
posture whose sign is known by inspection: a weight held in front of a
flexed elbow loads the elbow flexors.

**The kernel glue** (`biomech/kernel.rs`, new) is the only module that
touches garust:

```rust
/// A planar tree ready for garust: links in parent-first order, the joint
/// each link's variable belongs to, and the loads on it.
pub struct PlanarTree {
    links: Vec<TreeLink>,       // garust_geo::tree::TreeLink, revolute about z or prismatic
    joints: Vec<Option<Joint>>, // which reported joint (if any) each link is
    loads: Vec<LoadSpec>,       // Weight { link, at, kg, g } | Cable { link, at, pulley, newtons } | Push { link, at, force }
}

/// Moments at every reported joint, cross-checked by virtual work.
/// Errors (never panics) when the two derivations disagree beyond `tol`:
/// a disagreement is a modelling bug, and serving either number would hide it.
pub fn moments(tree: &PlanarTree, q: &[f64], tol: NewtonMetres)
    -> Result<Vec<(Joint, NewtonMetres)>, KernelError>;
```

- `moments` runs `fk` once, calls `joint_torque` per reported joint, then
  `generalized_torque` per joint, and compares them (R-0066 AC5). The step `h`
  is a stated constant, `1e-6` rad, the step the parity spike used. The
  tolerance is `1e-4 N·m`, two orders above the spike's worst case
  (1.5e-6 N·m) and far below the 0.01 N·m the screen shows.
- `Push` is a fixed force at a point of a link. It is a garust `Load`
  implemented in `kernel.rs`: about ten lines, since `Load` is a public trait.
  It models a platform's or a pad's reaction (§2.6). No garust change is
  needed.
- The glue builds borrowed `Tree`s from owned `Vec`s per request. garust
  allocates nothing (`MAX_LINKS = 64`). No exercise here has more than 8
  links.

### 2.2 Slice 66-a — the squat on the kernel

`squat::pose()` keeps producing the `Posture` (SPEC-0045 §2.6). What changes
is how its moments are computed: `analysis.rs` and `range.rs` call
`kernel::moments` over a tree built from the posture (`squat::tree`), instead
of `chain::moment_about`.

- **The tree.** Root at the foot on the floor, then ankle, shank, knee,
  thigh, hip and trunk. The lumbar row is a revolute joint at L5/S1 with the
  same placement convention (SPEC-0045 §2.5). The head and arms are carried as
  a `Weight` at the HAT's COM, exactly as `chain.rs` carries them today. The
  bar is a `Weight` at the posture's load point.
- **Parity, the gate.** A test runs every posture of the SPEC-0045 slice A
  and B1 golden suites through both paths and asserts
  `|kernel − chain| ≤ 1e-9 N·m` at every joint (R-0045 AC17(c)). The suite
  covers the parity spike's grid: four bodies, four bars, three
  dorsiflexions.
- **Then `chain.rs` goes.** Once parity is green in the same PR,
  `moment_about`, `body_moment_about`, `above` and `ChainSegment` are removed,
  and the parity test keeps the golden literals rather than the old function.
  One engine, not two (ADR 0001). `ChainSegment::com` moves to `anthro.rs` as
  the segment-COM helper the trees use.
- **The endpoint does not move in 66-a.** `POST /biomech/squat/range`
  answers byte-for-byte as before. Its e2e test is the proof.

### 2.3 The library (slice 66-b)

`core::exercise` (new) holds the library. One trait, because six models
implement it (R-0066 AC1; "every abstraction pays for itself" — it has six
callers on day one):

```rust
/// One exercise: what it is, what can be changed, and its range of postures.
pub trait ExerciseModel: Sync {
    /// Stable id, e.g. `"dumbbell_curl"`. Never renamed once served.
    fn id(&self) -> ExerciseId;
    /// The controls R-0067 builds, the equipment and the joints reported.
    fn describe(&self) -> &'static Description;
    /// The range of postures for validated inputs. A posture the convention
    /// cannot reach is a typed refusal, never a figure drawn anyway.
    fn range(&self, input: &Input) -> Result<ExerciseRange, Refusal>;
}

/// Every modelled exercise. The only list; `model_for` and the plan read it.
pub fn library() -> &'static [&'static dyn ExerciseModel];
```

- **`Description`**: the exercise's name (es/en), its `Equipment` (one of
  R-0066 AC2's four kinds), the joints it reports, and its parameters. Each
  parameter is `{ key, group, unit, default, min, max, step }`. `group` is
  one of `body | motion | machine | load`, the groups of R-0067 AC4. The
  body group is the same for every exercise (height, mass, the segment
  lengths the tree uses), so it is built by one function, not repeated.
- **`Input`**: the validated values by key, plus `Body` (from `scale`, as
  SPEC-0059) and a `FrameCount`. Validation against the description is
  generic: one function checks every key against its `min..=max`. An unknown
  key or a missing required key is `400` with that key as `field`.
- **`ExerciseRange`**: frames of `{ progress, points, moments }`. `points` is
  the named joint and equipment points (`shoulder`, `elbow`, `hand`,
  `pulley`, `platform`…) the app draws. `moments` is
  `[{ joint, resisting, moment, uncertainty }]`, the SPEC-0059 shape. It also
  carries the assumptions (SPEC-0045 `Assumption`, extended per §2.5–2.6),
  the settings restated (R-0066 AC2), and which lengths came from the table.
- **The squat** joins as `back_squat`, a thin adapter over `squat_range`. Its
  parameters are dorsiflexion (motion) and the bar (load).
- **Uncertainty.** Each exercise states its COM-placement spread from the
  table, as SPEC-0045 does. The two numbers already in `analysis.rs` are
  reused, not re-derived.

### 2.4 Serving (slice 66-b)

```
GET  /exercise-models               → [{ id, name, equipment, joints }]
GET  /exercise-models/{id}          → Description (R-0066 AC9)
POST /exercise-models/{id}/range    → ExerciseRange (R-0066 AC8)
```

- The request body is `{ height_m, mass_kg, lengths?: {segment: m}, values:
  {key: number}, frames? }`. Height, mass, the length range and `frames` use
  SPEC-0059's table and bounds, unchanged.
- **Errors.** Invalid field: `400 {"error":"validation","field":…}`. No
  session: `401`. Unknown id: `404`. A refusal is `200` with
  `{ "refusal": { "reason": …, "at": … } }` in place of the frames, as
  SPEC-0059 answers `end: rejected` with `200` (R-0067 AC8 reads it).
- **Privacy.** As SPEC-0059 AC8: the request's `Debug` prints which fields
  are present, never a value. Nothing is stored.
- **The plan** (R-0066 AC11): `model_for(name)` looks the name up in the
  library's names and aliases instead of its hard-coded `matches!`, and
  `PlanExercise.physics` carries the exercise id. The wire field stays
  `physics`. Its values gain the new ids, and the app's `Lift.fromWire`
  accepts them (an unknown value is already `null`, i.e. *Pronto*).
- `POST /biomech/squat/range` stays until the app moves to the new endpoint
  (R-0066 AC10). Its removal is SPEC-0067's.

### 2.5 The free-weight and cable exercises (slices 66-c, 66-d)

All three are rooted at a fixed proximal point, so no balance solve is
needed: the trunk is held by the lifter's stance or by a seat, and the
moments asked for are distal to it.

**Dumbbell curl (`dumbbell_curl`, free weight, 66-c).**
- Tree: shoulder (fixed root) → upper arm (revolute, its angle a parameter)
  → elbow (revolute, the motion) → forearm and hand.
- Loads: upper arm, forearm and hand as `Weight`s at their Winter COMs; the
  dumbbell as a `Weight` at the hand's COM.
- Parameters: `upper_arm_deg` (motion; 0° = hanging, −60..=60, default 0;
  a preacher curl is about 45°); `elbow_from_deg`/`elbow_to_deg` (motion,
  0..=150, defaults 0 and 140); `dumbbell_kg` (load, 0..=100, default 10).
- Reported: elbow (flexors), shoulder (flexors or extensors, signed).
- Oracles (R-0066 AC6):
  - upper arm vertical, forearm horizontal, 10 kg: `τ_elbow = 10·9.81·L_fh +
    m_f·9.81·d_f + m_h·9.81·d_h`, the literals computed from Winter's rows for
    the standard body and written beside the arithmetic;
  - forearm vertical (hanging): `τ_elbow = 0`.

**Triceps pushdown (`triceps_pushdown`, cable, 66-c).**
- Tree: as the curl, upper arm at its parameter angle.
- Loads: segment `Weight`s, and a `Cable` from the hand to the pulley, of
  tension `stack_kg · 9.81`.
- Parameters: `upper_arm_deg` (motion, −30..=30, default 0);
  `elbow_from_deg`/`elbow_to_deg` (motion, defaults 100 and 0, the reel's
  range); `pulley_height_m` (machine, 1.5..=2.6, default 2.20);
  `pulley_ahead_m` (machine, −0.3..=0.8, default 0.10, measured from the
  shoulder); `stack_kg` (load, 0..=100, default 20).
- Reported: elbow (extensors), shoulder.
- Oracles:
  - forearm horizontal, pulley straight above the hand: `τ_elbow = T·L_fh`
    plus the segment terms;
  - **direction oracle**: the pulley moved so the cable makes a stated angle φ
    with the forearm: `τ = T·L_fh·sin φ`. This tests that the cable is not
    treated as a weight (garust's own warning, `load.rs`).

**Lat pulldown (`lat_pulldown`, cable, 66-d).**
- Tree: hip (fixed root, the seat and thigh pad) → trunk (revolute, its lean
  a parameter) → shoulder (revolute) → upper arm → elbow (revolute) →
  forearm and hand.
- Motion: the hand moves on a straight path from overhead (`hand_top`) to the
  upper chest (`hand_bottom`) in the plane. Shoulder and elbow are solved
  from the hand by two-link IK at each frame, elbow behind the bar, as the
  reels do (`garust::twolink`, already in the pinned rev). A hand outside the
  arm's reach is the refusal `out_of_reach` at that frame.
- Loads: segment `Weight`s (trunk, head, arms), and a `Cable` from the hand to
  the pulley.
- Parameters: `trunk_lean_deg` (motion, 0..=40, default 10);
  `pulley_height_m` (machine, 1.8..=2.6, default 2.30); `pulley_ahead_m`
  (machine, −0.2..=0.4, default 0.20, from the hip); `stack_kg` (load,
  0..=150, default 40).
- Reported: shoulder (extensors), elbow (flexors), lumbar (signed).
- Oracles: arm straight and horizontal forward, cable vertical: `τ_shoulder
  = T·L_arm` plus the segment terms, `τ_elbow` = the forearm and hand terms
  only; a direction oracle as for the pushdown.

### 2.6 The guided machines (slice 66-e)

**The assumption, stated once.** The sled or carriage moves on a rail at
angle α to the horizontal. The force between the feet and the platform acts
**along the rail, through the centre of pressure** (`cop`) on the foot. `cop`
is a setting: heel, mid-foot or toes, at stated fractions of the foot's
length. The seat or pad takes everything else. With that, each joint's moment
is the moment of the platform force and the distal segments' weights, and it
is determinate. The result carries a new `Assumption::platform_force_along_rail
{ cop }`, and the screen shows it (R-0067 AC12). Friction is a second stated
assumption, a coefficient on the rail's normal force (default 0, a
frictionless rail, range 0..=0.35 after NASA RP-1228 as the reel cites).

**45° leg press (`leg_press_45`, guided).**
- Tree: hip (fixed root: the seat, at a stated back angle) → thigh (revolute)
  → knee (revolute) → shank → ankle (revolute) → foot.
- Motion: the platform slides along the rail (a prismatic variable in the
  geometry), and the legs are solved by the two-circle intersection (hip
  fixed, foot on the platform at the `foot_height_m` setting). The parameter
  is knee angle from `knee_from_deg` to `knee_to_deg`.
- Loads: leg segment `Weight`s; a `Push` at the foot's `cop` of magnitude
  `(sled_kg + plates_kg)·g·(sin α + μ cos α)`, along the rail toward the body.
- Parameters: `rail_deg` (machine, 30..=60, default 45); `seat_back_deg`
  (machine, 20..=60, default 40); `foot_height_m` (machine, position on the
  platform, −0.15..=0.15, default 0); `cop` (machine, heel | mid | toes,
  default mid); `friction` (machine, 0..=0.35, default 0); `sled_kg`
  (machine, 0..=150, default 50, the sled's own mass, stated as a generic
  machine's); `plates_kg` (load, 0..=500, default 100);
  `knee_from_deg`/`knee_to_deg` (motion).
- Reported: hip (extensors), knee (extensors), ankle (plantar flexors).

**Hack squat (`hack_squat`, guided).**
- The same assumption, with the roles swapped: the platform is fixed and the
  body rides the carriage on the pad. The force along the rail is
  `(carriage_kg + plates_kg + m_upper)·g·(sin α + μ cos α)`, where `m_upper`
  is the trunk, head and arms that ride the pad. The tree and the IK are the
  leg press's.
- Parameters: as the leg press, without `seat_back_deg` (the pad is parallel
  to the rail), with `carriage_kg` in place of `sled_kg`.

**Oracles for both:**
- **Zero-lever oracle**: a posture where the force line passes through the
  knee gives `τ_knee` = the shank and foot weight terms only.
- A posture with the force line a stated distance d from the hip:
  `τ_hip = F·d` plus the segment terms.
- **Direction oracle**: changing `rail_deg` with everything else fixed
  changes F by `sin α` and nothing else.
- Moving `cop` from heel to toes moves load from the hip to the knee (R-0066
  AC14), shown on one posture with both values as literals.

### 2.7 Slicing

| Slice | Content | Depends on | Size (est.) |
|---|---|---|---|
| **66-a** | garust as a git dependency at the ADR rev; `kernel.rs` (tree, `Push`, `moments` with the cross-check, `resisting`); the squat on the kernel; parity test; `chain.rs` removed | — | ~500 |
| **66-b** | `core::exercise`: the trait, `Description`, `Input`, generic validation, `library()` with `back_squat`; the three endpoints; `model_for` and `physics` from the library | 66-a | ~600 |
| **66-c** | `dumbbell_curl`, `triceps_pushdown` and their oracles | 66-b | ~400 |
| **66-d** | `lat_pulldown`, two-link IK, reach refusal, oracles | 66-c | ~400 |
| **66-e** | `leg_press_45`, `hack_squat`, the platform assumption, friction, oracles | 66-b | ~500 |

Each slice is a PR with its tests first (red), the architect's review and
the owner's merge. 66-c to 66-e add no endpoint code: a new model is one file
and one line in `library()`. R-0067's screen (SPEC-0067) can start against
66-b's description with only the squat in it.

## 3. Code outline

```
backend/crates/core/src/biomech/
  kernel.rs        PlanarTree, LoadSpec, Push (impl garust Load), moments(),
                   resisting() — the ONLY module that imports garust     [66-a]
  squat.rs         + tree(body, posture) -> PlanarTree                    [66-a]
  analysis.rs      moments from kernel::moments                           [66-a]
  chain.rs         removed                                                 [66-a]
backend/crates/core/src/exercise/
  mod.rs           ExerciseModel, ExerciseId, Equipment, library()        [66-b]
  description.rs   Description, Parameter, Group, body_parameters()       [66-b]
  input.rs         Input, validate(description, values)                   [66-b]
  range.rs         ExerciseRange, Frame, named points                     [66-b]
  back_squat.rs    adapter over biomech::squat_range                      [66-b]
  curl.rs, pushdown.rs                                                    [66-c]
  pulldown.rs, ik.rs (two-link over garust::twolink)                      [66-d]
  guided.rs        the platform assumption, shared by both machines;
  leg_press.rs, hack_squat.rs                                             [66-e]
  tests/           per model: oracles, AC5 cross-check, ranges, goldens
backend/crates/api/src/exercise/
  mod.rs, handlers.rs, request.rs, response.rs                            [66-b]
```

## 4. Non-goals

- The interactive screen: SPEC-0067.
- Lever and cam machines (R-0066 OQ-3), frontal-plane and 3D exercises,
  acceleration terms.
- Saving anything. The deadlift and bench (R-0062, R-0063) as models: they
  join the library through their own specs.

## 5. Open questions

- **OQ-A. Machine defaults.** The defaults above (pulley heights, rail and
  seat angles, sled masses) are illustrative, from the reels' geometry and
  common machines, and the result says so (R-0066 AC3). Measure two real
  machines before acceptance, or accept as illustrative? Proposal: accept as
  illustrative now; R-0066 OQ-2 stays open for measured values.
- **OQ-B. The squat's old endpoint.** Keep `POST /biomech/squat/range`
  through SPEC-0067, then remove it there. Proposal: yes.

## 6. Acceptance criteria (this spec)

- 66-a: parity ≤ 1e-9 N·m on every golden posture; `chain.rs` gone;
  `cargo tree -p fitai-core` shows garust and no I/O crate; the squat
  endpoint's e2e unchanged.
- 66-b: the three endpoints with `400`/`401`/`404` and a refusal; the squat
  served by the new endpoint equals the old one within 1e-9 N·m; the plan's
  `physics` from the library.
- 66-c to 66-e: per exercise, the oracles of §2.5–2.6, the AC5 cross-check on
  every test posture, each parameter at both ends of its range, one out of
  range rejected, and the default posture's golden values.
- All: `cargo fmt`, `clippy -D warnings`, `cargo test --workspace` green.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-11-21 | The squat moves onto garust first (66-a), with parity as the gate and `chain.rs` removed in the same PR. | ADR 0001; one engine; six exercises on a hand-written chain would be six chains. |
| 2026-11-21 | Guided machines: the platform force acts along the rail through the foot's centre of pressure, and `cop` is a setting. | The machine is statically indeterminate otherwise. Stating the assumption, and letting the lifter move it, keeps the result checkable. |
| 2026-11-21 | The five first exercises: dumbbell curl, triceps pushdown, lat pulldown, 45° leg press, hack squat. | Owner asked for five. They cover free weight, cable and guided rail, one joint to three, and each has a reel to compare against. |

## Changelog

- 2026-11-21 — created (Draft).
