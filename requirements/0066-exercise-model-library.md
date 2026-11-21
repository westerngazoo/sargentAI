# R-0066 — Exercise model library: every exercise on the kernel, machines included

- **Status:** Draft
- **Milestone:** M6 (the lift model)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-11-21
- **Depends on:** R-0045 as amended 2026-10-23 (the garust kernel, AC17's
  checks, the Winter scaling) and SPEC-0045 slice E (the squat moved onto the
  kernel). R-0062 and R-0063 become two entries of this library rather than
  one-off models (see OQ-4).
- **Realized by:** SPEC-0066 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The server holds a library of exercise models. Each model describes one
exercise as a kinematic tree on the garust kernel, the loads acting on it, the
equipment that produces those loads, and a motion convention that turns a few
parameters into a range of postures. For any modelled exercise, a lifter's
body, a load and the equipment's settings, the server returns the range of
postures with each joint's moment, its uncertainty and its stated
assumptions, in the shape the squat check already uses.

The library covers four kinds of resistance: free weight under gravity, a
cable running to a pulley, a carriage guided along a rail, and a lever or cam
machine with a resistance profile. Every exercise the reels already simulate
(`rotorf-sico/fbf`) is a candidate entry. Each one enters the library only
after it is rebuilt on the kernel and passes the same checks as the squat. A
reel's own numbers are never copied in.

## 2. Rationale

- **One engine, or the promise breaks.** The reels compute each exercise in
  their own script, and on the squat the reel and the model already disagreed
  by 35% (R-0045 decision log, 2026-10-17). The app can show an exercise's
  physics only if the app's engine computes it.
- **Machines are most of a gym.** A cable, a leg press or a pec deck changes
  where the force points and how it grows through the rep. A model that knows
  only gravity cannot show why a pulley's height matters (owner, 2026-11-21).
- **Today only the squat is modelled.** Every other exercise in the plan
  reads *Pronto* (R-0061 AC9). The library is what turns *Pronto* into
  *Física*, one exercise at a time.

## 3. Acceptance criteria

### The model of an exercise

- **AC1. One description per exercise.** An exercise model states its tree
  (segments and joints), its loads, its equipment, its motion convention and
  its parameters with their defaults and allowed ranges. No exercise carries
  its own moment rule: moments come from the kernel (R-0045 AC1).
- **AC2. Four kinds of equipment.** Each load is one of:
  - **free weight**: a mass under gravity at a grip or a contact point
    (barbell, dumbbell, kettlebell, the body's own segments);
  - **cable**: a force of the stack's weight along the line from the grip to
    a pulley point, with the pulley's position a setting;
  - **guided**: a carriage on a rail at a stated angle, so the load acts
    along the rail (leg press, hack squat, Smith machine), with friction
    stated as an assumption;
  - **lever or cam**: a resistance that varies with the machine's angle
    through a stated profile.
  Each kind's settings have a default and a range, and the result restates
  the settings used.
- **AC3. Generic machines, stated as such.** A machine model is a generic
  geometry, not a brand's. The result says so, and says which settings would
  differ on a real machine (for example a cam's profile).
- **AC4. The motion convention.** Each model names the parameters that move
  it (for example trunk lean and knee travel in a squat, the elbow's path in a
  curl) and how the remaining angles are solved. Where the body must stay in
  balance (standing, unsupported), the solver keeps it in balance. Where the
  body is supported (a seat, a bench, a pad), the support is part of the
  model, and its reaction is reported.

### The checks (R-0045 AC17, per exercise)

- **AC5. Two derivations agree** on every posture the tests produce:
  `joint_torque` and `generalized_torque` within the stated tolerance.
- **AC6. Oracles.** At least one hand-computed or cited case per joint per
  exercise, with the expected value as a literal beside its arithmetic. For a
  cable or a guided load, at least one oracle tests the load's direction.
- **AC7. No entry without its checks.** An exercise is served as modelled
  only when AC5 and AC6 pass for it. Until then it reads *Pronto*, as today.

### Serving

- **AC8. One endpoint for every exercise.** The server computes the range for
  any modelled exercise from its id, the lifter's body (height, mass,
  optional segment lengths), the load, the equipment's settings and the
  motion parameters. Invalid fields get `400` (as in SPEC-0059), no session
  gets `401`, an unknown or unmodelled exercise gets `404`.
- **AC9. The model's description is served too.** The app can ask for an
  exercise's parameters, settings, defaults and ranges, so it can build its
  controls without a list compiled into the app (R-0067).
- **AC10. The squat moves without changing.** The squat is served through the
  same endpoint, and its answers match today's `POST /biomech/squat/range`
  within R-0045 AC17(c)'s bound (1e-9 N·m). The old endpoint stays until the
  app no longer calls it.
- **AC11. The plan knows what is modelled.** `physics` on each planned
  exercise (SPEC-0059) comes from the library, not a hard-coded list.

### Boundaries

- **AC12. Pure core.** Exercise models live in `fitai-core` over the kernel,
  testable without I/O (R-0045 AC15).
- **AC13. No injury or safety claims** (R-0045 AC14). A larger moment is a
  number, never "dangerous".
- **AC14. Tests.** Per exercise: AC5, AC6, a parameter at each end of its
  range, a value outside the range rejected, and the default posture's
  golden values. Per kind of equipment: a setting that changes the moment in
  the direction the hand-computed case predicts (a lower pulley, a steeper
  rail).

## 4. Constraints & non-goals

- **Sagittal first.** Exercises whose physics is in the side plane come
  first. A frontal or 3D exercise (for example the bench with grip width,
  R-0063, or a lateral raise) needs a 3D tree, and its entry says so.
- **Quasi-static.** Inverse dynamics without acceleration terms, as R-0045.
  Speed, momentum and the treadmill's running are out of scope.
- **No muscle forces.** Demand per joint only. Capacity and the sticking
  point are R-0064.
- **The reels are prior art, not a source of numbers** (ADR 0001 §5). Their
  geometry and motion may guide a model. Their results may not be pasted in
  as expected values.
- **No new dependency** beyond garust.

## 5. Open questions

- **OQ-1. Which exercises first.** The order in which the reel exercises enter
  the library. The reel inventory (2026-11-21) found roughly 25 free-weight,
  13 cable and 2 guided-rail exercises in `rotorf-sico/fbf/motor`. It found
  one chest-supported machine, drawn as a constant horizontal pull, and no
  lever or cam machine. All are sagittal and quasi-static except two
  frontal-plane reels (barbell vs dumbbell bench, lateral raise). Their motion
  is closed-form in code, so it can guide a model. Proposal: one per kind of
  equipment, simplest joint count first:
  1. **Curl** (free weight, elbow only): proves a second tree on the kernel
     with one joint.
  2. **Triceps pushdown** (cable, high pulley, elbow only): proves the cable
     load and the pulley setting with one joint.
  3. **Hack squat** (guided rail, hip and knee): proves the rail and reuses
     the squat's legs.
  4. **Lat pulldown** (cable, shoulder and elbow, trunk angle a parameter):
     the first two-joint upper-body chain.
  Then the remaining free-weight and cable exercises, and the deadlift and
  bench through R-0062 and R-0063.
- **OQ-2. Machine geometry.** Generic defaults for each machine (pulley
  heights, rail angles, cam profiles) need a source: measured on real
  machines, taken from manufacturers' published dimensions, or chosen and
  stated as illustrative?
- **OQ-3. Cam profiles.** A cam machine's resistance curve is rarely
  published, and no reel models one. Proposal: the lever-or-cam kind is
  specified now but its first entry waits for a machine whose geometry can be
  measured (a plate-loaded lever). The chest-supported row's "constant
  horizontal pull" from the reels is not reused as is: it is an assumption
  about a machine, not a model of one.
- **OQ-4. R-0062 and R-0063.** Do the deadlift and the bench stay their own
  requirements that add entries here, or fold into this one?
- **OQ-5. Exercise identity.** The plan names exercises by free text, and
  `model_for` matches a few names. The library needs stable exercise ids and a
  mapping from the names in programs and logs. Here, or its own requirement?

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-11-21 | Every exercise's physics comes from the app's engine; the reels' mp4s are not shipped in the app. | Owner chose the interactive model over fixed videos. The reels compute in their own scripts and disagreed with the model by 35% on the squat. |
| 2026-11-21 | Machines are in scope: cable, guided rail, lever or cam. | Owner, 2026-11-21: "también máquinas". |

## Changelog

- 2026-11-21 — created (Draft).
