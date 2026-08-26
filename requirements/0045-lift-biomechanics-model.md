# R-0045 — Lift Biomechanics Model (joint torque & variant comparison)

- **Status:** Draft
- **Milestone:** M6 (pose pipeline) / M-Platform (the coach-facing half)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-08-25
- **Depends on:** R-0044 (the keypoint series this consumes), R-0013 (pose
  estimation), R-0004 (the logged set supplying the external load), R-0002
  (profile — height and weight scale the segment model)
- **Realized by:** SPEC-0045 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

## 1. Statement

Turn an analyzed lift into **mechanics**: the torque at each major joint through
the movement, and how that torque redistributes when the lifter changes stance
width, bar position, torso angle, or foot rotation.

A coach starts from a client's real analyzed set, adjusts a variable, and sees
the consequence stated as numbers — *"hip torque −18%, knee torque +22%"* — then
exports that variant, with its cue, to the client.

## 2. Rationale

This is the payoff of R-0044's keypoint series. Once a lift is a vector
structure, the mechanics are **deterministic physics on data the app already
holds**: segment geometry from the keypoints, segment mass from the profile,
external load from the logged set.

It is also the first feature that gives a *trainer* something they cannot get
from a spreadsheet or a video call — a per-client, quantified answer to "what
happens if we change this", grounded in that client's actual limb lengths
rather than a textbook diagram. That makes it a natural fit for the
trainer-platform direction (R-0040/R-0041).

## 3. Acceptance criteria

### The model

- **AC1. Inverse dynamics, not a physics engine.** The lift is modelled as a
  linked rigid-segment chain in the sagittal plane; joint moments are solved
  from static equilibrium at each sampled frame. This is linear algebra over
  the R-0044 series — no forward simulation, no collision solver, no game
  physics dependency.
- **AC2. Anthropometric scaling from published tables.** Segment masses,
  lengths and centre-of-mass positions come from a standard anthropometric
  table (Dempster/Winter or equivalent), scaled by the user's height and
  weight. The table and its source are cited in the spec — no invented
  constants.
- **AC3. Real external load.** The bar load comes from the logged set the clip
  is attached to. Where no load is known, the model reports bodyweight-only
  mechanics and says so — it never assumes a weight.
- **AC4. Joint torque is the output.** Hip, knee, ankle, and lumbar moments
  through the movement, with the peak and the position at which it occurs, in
  N·m, per rep.
- **AC5. Muscle groups, never individual muscles.** Torque is attributed to the
  muscle **group** that produces it (hip extensors, knee extensors, spinal
  erectors). Per-muscle force is **explicitly out of scope**: multiple muscles
  cross every joint, so the distribution is mathematically indeterminate
  without a musculoskeletal model and an optimization assumption. Reporting a
  per-muscle number would be a fabrication a coach would repeat to a client as
  fact. (Owner decision.)

### Variant comparison

- **AC6. Grounded in the client's own lift.** A variant starts from a real
  analyzed set — that person's limb lengths, posture, and load — and adjusts one
  or more variables. The comparison is always *against what they actually did*,
  never against a generic figure. (Owner decision.)
- **AC7. Adjustable variables.** At minimum: stance width, foot rotation, bar
  position (high/low bar; grip width for bench), and torso angle. Each has a
  stated plausible range; values outside it are rejected rather than
  extrapolated.
- **AC8. Deltas, not absolutes, are the headline.** The output is the *change*
  from the lifter's actual mechanics (`hip −18%`, `knee +22%`), because the
  absolute torque carries the model's error while the delta largely cancels it.
- **AC9. Physically valid postures only.** A requested variant that is
  anatomically implausible or does not balance is rejected with a typed reason.
  The model must not silently return numbers for a posture a human cannot hold.

### Coach export

- **AC10. Export a variant.** A variant can be saved with a coaching cue and
  attached to a client's program, reusing the authored-program storage from
  R-0041 rather than a parallel mechanism.
- **AC11. Ownership.** Variants are per-user; another user's is **404**, never
  403. Coach→client sharing is gated on the roles requirement (R-0040) and is
  **not** in this requirement's scope — until then a variant belongs to its
  author only.

### Honesty

- **AC12. State the model's assumptions on every result.** Sagittal-plane only,
  quasi-static, rigid segments, table-derived masses. These are real
  limitations and a coach relaying numbers to a client needs them visible, not
  buried in a help page.
- **AC13. Inherit R-0044's refusal.** If the underlying clip was refused, or the
  camera view cannot support sagittal analysis, the model refuses too. Mechanics
  computed from bad keypoints are confident nonsense.
- **AC14. No injury or safety claims.** Higher torque is not "dangerous" and
  lower is not "safe" — the output describes load distribution, not risk.
  (Consistent with R-0044 AC14.)
- **AC15. Pure core.** All mechanics live in `fitai-core` as deterministic
  functions over the keypoint series plus profile and load. Unit-testable
  against hand-computed textbook cases with no video, no database, and no
  model.

- **AC16. Tests.** Hand-verifiable static cases (a known posture and load with
  a torque computed by hand); a symmetric posture yielding symmetric torques;
  the documented high-bar/low-bar hip-knee shift reproducing the expected
  direction; each rejection reason; delta stability under small keypoint noise;
  and the bodyweight-only path.

## 4. Constraints & non-goals

- **Out of scope:** per-muscle force or activation (AC5); EMG; 3D or
  frontal-plane mechanics; muscle-fibre or tendon models; injury-risk scoring;
  optimizing *for* the user (the coach chooses the variant — the app computes
  its consequence); real-time feedback during a set; coach→client sharing
  (needs R-0040).
- No new heavyweight dependency: `ndarray` is already present and sufficient.

## 5. Open questions (deferred to SPEC-0045)

- **OQ-1:** Which anthropometric table, and how are segment parameters scaled
  when the profile lacks body-fat percentage?
- **OQ-2:** Quasi-static (ignore acceleration) or full inverse dynamics with
  segment acceleration from the frame series? Quasi-static is far simpler and
  defensible near the sticking point; it understates torque during rapid
  movement. Which, and where is the error stated?
- **OQ-3:** How is the lumbar moment estimated without a spine model — a single
  L5/S1 joint approximation, and what is its stated error?
- **OQ-4:** How does a variant map to a *changed posture*? Adjusting "stance
  width" must produce a new plausible joint configuration; is that a kinematic
  solve, or interpolation between measured templates?
- **OQ-5:** Plausible ranges for AC7's variables, and their source.
- **OQ-6:** Does a saved variant belong to the client, the coach, or the
  program? Interacts with R-0040 roles.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-25 | Inverse dynamics in `core`, not a physics engine | The task is solving forces from a known posture, not simulating bodies forward. Rapier-class engines solve the opposite problem and would add a large dependency for none of the need. |
| 2026-08-25 | Joint torque + muscle **groups**; no per-muscle force | Owner decision. Muscle redundancy makes per-muscle force indeterminate without a musculoskeletal model; a number a coach would repeat as fact must not be invented. |
| 2026-08-25 | Variants compare against the client's own analyzed lift | Owner decision. Grounding in real limb lengths and real load is the whole advantage over a textbook diagram, and deltas cancel much of the model's absolute error. |
| 2026-08-25 | Deltas are the headline, not absolutes | The absolute torque carries every modelling assumption; the difference between two postures of the same body carries far fewer. |

## Changelog

- _2026-08-25 — created (Draft)._
