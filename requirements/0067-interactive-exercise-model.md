# R-0067 — The interactive exercise model: change the body, the angles and the machine

- **Status:** Accepted (owner, 2026-11-21)
- **Milestone:** M6 (the lift model, shown to the lifter)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-11-21
- **Depends on:** R-0066 (the exercise model library, AC8 and AC9), R-0059
  (the squat check: figure, playback, readouts, caveats, Met). R-0061 AC9
  (the day's *Física* button) is where it opens.
- **Realized by:** SPEC-0067 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

Each modelled exercise opens as a model the lifter can play with. It starts
from a standard body, the exercise's default motion and the machine's default
settings. The lifter can change the body's measurements (trunk, upper arm,
forearm, thigh, shank), the motion's parameters (for example trunk lean, how
far the knee travels, where the bar sits), and the machine's settings (for
example the pulley's height, the seat, the rail's angle). After each change
the server recomputes and the figure, the moment bars and the readouts follow.

The lifter can also load their own profile's measurements, and return to the
standard body at any time. Every number on the screen comes from the server.

## 2. Rationale

- **The check shows one squat; the question is "and if…?"** R-0059 draws the
  squat for the lifter's measurements. Lifters and coaches ask what changes
  with a longer femur, more lean, a lower pulley. Moving one thing and seeing
  every joint answer is the product's promise made tangible (owner,
  2026-11-21).
- **A standard body makes it useful before onboarding.** Someone who has not
  entered measurements still sees a complete model, then makes it theirs.
- **Fixed videos cannot answer "and if…?"** The owner chose the interactive
  model over shipping the reels' mp4s (2026-11-21, R-0066 decision log).

## 3. Acceptance criteria

### The default

- **AC1. The standard body.** Every model opens on a stated standard body (a
  height and mass with segment lengths from the Winter table; see OQ-1), the
  exercise's default parameters and the equipment's default settings, all
  served by R-0066 AC9. The screen says it is the standard body.
- **AC2. My body.** The lifter can switch to their profile's measurements
  (height, mass, any measured segment lengths) and back to the standard body.
  Missing segment lengths come from the table, and the screen says which.

### The controls

- **AC3. Controls from the server's description.** The app builds each
  exercise's controls from R-0066 AC9: name, unit, default, range and step.
  No exercise's controls are compiled into the app.
- **AC4. Three groups.** The controls are grouped as *Cuerpo* (segment
  lengths, height, mass), *Movimiento* (the motion's parameters) and
  *Máquina* (the equipment's settings, only when the exercise has equipment
  with settings), plus the load.
- **AC5. Reset.** Each control and each group can return to its default.

### Recompute

- **AC6. The server recomputes.** A change sends one request with every
  current value. The figure, the moment bars and the readouts update from the
  answer. While a request is in flight the last answer stays on screen, and
  only the newest answer is drawn (stale answers are dropped).
- **AC7. Responsive enough to play with.** While a control is dragged, the
  app sends at most one request per stated interval (see OQ-3). The final
  value is always sent when the drag ends.
- **AC8. Refusals.** A combination the model cannot satisfy (for example no
  balanced posture reaches the depth asked) shows the server's typed reason
  in place of the figure, as R-0059's rejected state does, and says which
  control to move back.
- **AC9. Offline.** With no connection the screen says so and keeps the last
  answer. It computes nothing locally.

### What the lifter sees

- **AC10. R-0059's screen, extended.** Playback, scrubbing, the moment bars,
  the live graph, *La cuenta* and the caveats are R-0059's. The machine is
  drawn with the figure: the cable to its pulley, the rail, the lever.
- **AC11. What changed.** After a change, each joint's peak moment shows its
  difference from the previous answer in N·m and %, until the next change.
- **AC12. The caveats** (R-0059's, plus): the standard body is a table
  average, not the lifter; a machine is a generic geometry (R-0066 AC3).

### Boundaries

- **AC13. Thin client.** The app computes no posture and no moment. It may
  interpolate between returned frames for smooth playback, display only
  (as R-0065 AC13).
- **AC14. Nothing saved in v1.** Values changed on this screen do not change
  the profile. Saving a variant is a later requirement (see OQ-4).
- **AC15. No injury or safety claims** (R-0045 AC14, R-0059 AC6).
- **AC16. Tests.**
  - Flutter: controls built from a served description; each group; reset;
    the switch between standard body and profile; a drag sending at most one
    request per interval and the final value on release; a stale answer
    dropped; a refusal's message; offline.
  - The e2e test is owned by the `qa` agent: open an exercise, move a
    machine setting, see the moments change.

## 4. Constraints & non-goals

- **Only modelled exercises.** An exercise without an R-0066 entry reads
  *Pronto* and has no interactive model.
- **Side plane first,** as R-0066.
- **No comparison with the lifter's video.** That is R-0065. Its minimum
  posture may later appear here.
- **No local physics,** even for speed. If the round trip proves too slow on
  real phones, the answer is a spec change discussed with the architect, not
  a client-side model.

## 5. Open questions

- **OQ-1. The standard body.** Which height and mass? Proposal: 1.75 m and
  75 kg, with a switch to a second standard body (1.62 m, 60 kg) so the
  default is not one sex's average only. *Answered 2026-11-21:* 1.75 m / 75 kg, with a second standard body of 1.62 m / 60 kg.
- **OQ-2. Which angles move.** Should the lifter set any angle directly, even
  one the solver normally computes to keep balance? Proposal: only the
  convention's parameters in v1. A solved angle shown as a read-out, not a
  control, so every posture drawn is one the model can stand behind. *Answered 2026-11-21:* only the convention's parameters in v1; solved angles are read-outs.
- **OQ-3. The interval.** Proposal: at most one request every 150 ms while
  dragging. To be checked on a mid-range Android over mobile data in the
  spec. *Answered 2026-11-21:* at most one request every 150 ms while dragging, checked on a mid-range Android in the spec.
- **OQ-4. Saving.** Save a variant (for example "my squat with a lower bar")
  in a later requirement? *Answered 2026-11-21:* saving is a later requirement.
- **OQ-5. Entry.** Replace the *Física* check of R-0059 with this screen, or
  open this one from it (*Jugar con el modelo*)? *Answered 2026-11-21:* opened from the Física check as *Jugar con el modelo*; the check stays.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-11-21 | Interactive model per exercise instead of fixed videos; a standard body by default; body, angles and machine settings adjustable. | Owner, 2026-11-21. |
| 2026-11-21 | A new requirement rather than an amendment of R-0059. | R-0059 is Met; this adds a screen and depends on R-0066. |
| 2026-11-21 | Accepted, with the proposals as the answers to the open questions. | Owner: "acepta 0066 y 0067". |
| 2026-11-22 | The standard body's ankle dorsiflexion is 38.8° (Konor et al. 2012, weight-bearing lunge, 20 healthy adults), stated on screen as the standard body's; dorsiflexion is a *Cuerpo* measure, not a *Movimiento* control. | Owner, option (c) with (a) (SPEC-0066 §2.3.1). SPEC-0045 forbids an uncited default; this one is cited. |

## Changelog

- 2026-11-21 — created (Draft).
- 2026-11-21 — Accepted by the owner; open questions answered with the proposals.
- 2026-11-22 — the standard body's dorsiflexion recorded (SPEC-0066 §2.3.1).
