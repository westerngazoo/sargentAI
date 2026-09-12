# Muscle model (Hill) — design brief for the interactive simulator

Date: 2026-09-05
Status: **brief only — no requirement, no spec, no code.** Nothing here has been
implemented in this repository. It exists so the owner can approve three small
decisions (§6.5) and then a requirement.

**Headline:** the architecture already supports this. `fitai-core` is pure and
portable today; what needs changing is two paragraphs of prose that describe a
stricter rule than the code actually follows.

---

## 1. What was asked

> "Can the muscle be simplified to a spring model? I want this for the app, with
> a graphical interface driving the simulator."

Intended use: **teaching**. The user drags sliders and watches the curves deform
in real time. Not prediction, not prescription.

## 2. Answer: a motor and two springs, not one spring

A spring's force depends on how far it is stretched. A muscle's active force does
not — it depends on its **length** and its **velocity**, which is a different
thing entirely. The standard teaching model (A. V. Hill, 1938) has three
elements, and only two of them are springs:

| Element | What it is | Spring? |
|---|---|---|
| **CE** — contractile | the only active part | **No.** Force is a function of length and velocity |
| **PE** — parallel elastic | connective tissue | Yes. Contributes only past resting length |
| **SE** — series elastic | tendon | Yes |

Everything below is **normalised**, which is what makes it slider-friendly and
unit-free:

```
l = length / optimal length        (1.0 = where it produces its maximum)
v = velocity / max velocity        (+ shortening, − lengthening)
F = force / max isometric force
```

## 3. The model

Three pure functions and one composition. No state.

### 3.1 Active force–length

```
fl(l) = exp( −((l − 1) / ω)² )                     ω ≈ 0.45
```

A bell centred on optimal length. This is *active insufficiency*: a muscle that
crosses two joints can end up so short it can no longer contribute.

### 3.2 Force–velocity (Hill)

```
v ≥ 0 (shortening):   fv = (1 − v) / (1 + v/a)              a ≈ 0.25
v < 0 (lengthening):  fv = (F_ecc·|v| + b) / (|v| + b)      F_ecc ≈ 1.5, b ≈ 0.35
```

Both branches equal 1.0 at v = 0 but arrive with different slopes. **That kink is
real, not a fitting artefact** — it is why an eccentric rep holds roughly 1.3×
what can be lifted concentrically.

### 3.3 Passive force–length

```
l ≤ 1:   fp = 0
l > 1:   fp = (exp(k·(l−1)/e0) − 1) / (exp(k) − 1)          k ≈ 4.0, e0 ≈ 0.60
```

### 3.4 Total

```
F(l, v, act) = act · fl(l) · fv(v) + fp(l)                  act ∈ [0, 1]
```

**Seven sliders, every one with a name a person can say out loud:** bell width,
Hill curvature, eccentric ceiling, eccentric rate, passive stiffness, passive
threshold, activation. The defaults above are textbook values and are meant to be
tuned, not treated as truth.

## 4. Validation performed

A reference implementation was written and exercised outside this repository (see
§8). It reproduces the properties the model is required to have:

| Check | Expected | Got |
|---|---|---|
| Isometric at optimal length | 1.000 | 1.000 |
| Force at maximum shortening velocity | 0.000 | 0.000 |
| Fast eccentric | → 1.50 ceiling | 1.426 at v = −2 |
| Passive force below resting length | 0 | 0 |
| Force–length symmetry about l = 1 | symmetric | 0.291 at both l = 0.5 and l = 1.5 |

## 5. Worked example — why this is worth building

The value is not the curves. It is the **crossing** of two curves: what the load
*demands* against what the muscle *can supply*.

Taking a dumbbell curl (load 196 N at a 0.32 m forearm, elbow flexors modelled at
2200 N inserting 4 cm from the joint, fibre length sweeping 1.15 → 0.75 across
the range):

| Load | Result |
|---|---|
| 20 kg | passes through the whole range |
| 25 kg | passes |
| **30 kg** | **stalls between 69° and 131° of elbow flexion** |
| 35 kg | stalls between 54° and 143° |

That predicted stall band is the sticking point every lifter has felt just past
90°, and it falls out of the model rather than being put in by hand. A teaching
app that shows a learner *why* the bar stops there is doing something no rep
counter does.

## 6. Architecture — resolved

**The finding that decides this: `fitai-core` is already correct.**

`backend/crates/core` declares only `uuid`, `chrono`, `serde` and `thiserror`.
Its own manifest carries the comment *"the production purity boundary (no
sqlx/axum/http) is preserved."* Nearly six thousand lines of domain logic —
`archetype`, `matching`, `pose`, `program`, `periodize`, `adjust`, `nutrition` —
with no server dependency anywhere.

So the shared, portable core this feature needs **already exists and is already
disciplined**. It is simply parked in a folder named `backend/`, and nothing on
the app side consumes it yet. There is no re-architecture of the code to do.

### 6.1 What is actually wrong

`project-specifics.md` §"Domain notes" says:

> **The intelligence is server-side.** The mobile app is intentionally thin:
> log capture, photo capture, dashboard display. No on-device inference. This is
> a deliberate choice for the target market — Mexico and LATAM, where Android
> hardware quality varies widely — and it lets the ML model improve without
> shipping app updates.

**That reasoning is sound and must be kept.** But the rule it produces conflates
two different things under one word:

| | ML inference | Deterministic math |
|---|---|---|
| What it is | trained weights over user data | closed-form equations |
| Improves over time? | yes — that is the point | no, it is fixed |
| Must ship without a release? | **yes** | no, there is nothing to update |
| Cost per evaluation | model-dependent, can be large | 2 exponentials, ~15 flops |
| Can a low-end Android do it 60x/s? | maybe not | trivially |

Every reason given for keeping intelligence server-side applies to the left
column. **None of them applies to the right.** The Hill model has no weights, will
never be retrained, and costs roughly 3,800 floating-point operations to draw a
256-sample curve — about a hundredth of a percent of one frame's budget on the
weakest phone in the target market.

Routing that through HTTP does not protect the LATAM user. It makes the feature
unusable for them, because they are the ones on the worst connections.

### 6.2 The rule that replaces it

> **The client owns interaction. The server owns truth.**
>
> On device: anything that must answer within a frame to continuous user input,
> and whose result is fully determined by code we ship.
>
> On the server: anything that needs data the device does not have, models too
> large to ship, or a result that must be identical for every user and auditable
> after the fact.
>
> Both sides call the same `fitai-core`. Neither reimplements the other.

That last line is the real prize. Because the app and the API link the *same*
crate, they cannot disagree about the physics — a class of bug a reimplemented
client is guaranteed to have eventually.

It is also what §2 of the Engineering Constitution already asks for:
*"dependencies point inward toward the core."* The core exists; this gives it its
second consumer.

### 6.3 The concrete change

No files move. No paths change. No accepted spec is invalidated.

```
backend/                       <- the Rust workspace (name now misleading; see 6.4)
  crates/
    core/     fitai-core       <- unchanged, still pure
    muscle/   fitai-muscle     <- NEW. Hill model. Depends on nothing.
    api/      fitai-api        <- unchanged
    ffi/      fitai-ffi        <- NEW. flutter_rust_bridge surface.
                                  Depends on core + muscle. Nothing depends on it.
mobile/                        <- consumes fitai-ffi over FFI
```

`fitai-muscle` is its own crate rather than a module inside `fitai-core` because
it has strictly fewer dependencies — no `uuid`, no `chrono`, no `serde` — and
staying `no_std`-compatible keeps it honest and trivially testable.

`fitai-ffi` is a leaf. Dependencies point inward; `core` and `muscle` never learn
that a phone exists.

### 6.4 Costs, stated plainly

1. **CI.** `.github/workflows/ci.yml` pins `working-directory: backend` and
   `workspaces: backend`; both keep working unchanged. What is needed is one
   **new** job cross-compiling `fitai-ffi` for `aarch64-apple-ios` and the four
   Android ABIs. `build-apk.yml` grows a step to bundle `jniLibs`.
2. **Two paragraphs of prose.** `project-specifics.md` §"Domain notes" and
   `README.md` line 12 state the old rule and must be amended to §6.2. That is
   the only edit to existing files.
3. **The folder name `backend/` becomes inaccurate** once the app links a crate
   from it. Renaming touches twelve specs that reference the path, so the
   recommendation is to **leave it and fix the wording** — a rename buys nothing
   and invalidates accepted documents.
4. **Float determinism.** The same crate now runs on two architectures. For a
   teaching visual this is irrelevant. If a later feature persists or compares a
   core-computed value across device and server, that needs its own decision.

### 6.5 What still needs the owner's approval

Per §1.2 these are not Claude's to decide. What is proposed:

- **D-N.1** — amend the "no on-device inference" rule to the interaction/truth
  split in §6.2, keeping the LATAM reasoning intact for ML.
- **D-N.2** — add `fitai-muscle` and `fitai-ffi` to the existing workspace rather
  than creating a second repository.
- **D-N.3** — leave `backend/` named as it is.

## 7. Proposed API shape

The constraint that shapes this API is the 60 Hz redraw. The core must not
allocate per frame and must not return one point at a time.

```rust
/// Every slider in the UI, in one struct.
pub struct Params {
    pub omega: f32,        // bell width          (force–length)
    pub a_hill: f32,       // curvature           (force–velocity, shortening)
    pub f_ecc: f32,        // eccentric ceiling
    pub b_ecc: f32,        // eccentric rate
    pub k_pe: f32,         // passive stiffness
    pub e0_pe: f32,        // passive threshold
    pub activation: f32,   // 0..1
}

/// Point queries — cheap, for readouts.
pub fn force(p: &Params, l: f32, v: f32) -> f32;

/// Curve queries — fill a caller-owned buffer, sampled uniformly over [x0, x1].
/// The UI allocates once and reuses the buffer every frame.
pub fn force_length_curve(p: &Params, x0: f32, x1: f32, out: &mut [f32]);
pub fn force_velocity_curve(p: &Params, x0: f32, x1: f32, out: &mut [f32]);
pub fn passive_curve(p: &Params, x0: f32, x1: f32, out: &mut [f32]);
```

Deterministic, `no_std`-compatible, no I/O, no globals. That makes it trivial to
unit-test against the table in §4 before any UI exists.

## 8. Reference implementation

A validated prototype lives **outside this repository** at:

`/Users/goose/projects/fisicobuenfisico/tools/musculo.py`

Pure functions, no state, one function per equation above. It is the artefact the
numbers in §4 and §5 came from. It is a reference for porting, not a dependency.

## 9. Suggested acceptance criteria (for the requirement, once approved)

1. The five checks in §4 pass as unit tests, with the tolerances stated there.
2. Curve functions fill a caller-supplied buffer and perform no heap allocation
   (verifiable with a counting allocator in test).
3. All seven parameters are reachable from the UI and every change is reflected
   in the next frame.
4. A curve of 256 samples recomputes in under 1 ms on a mid-range device, so a
   frame budget of 16 ms is never at risk from the model.
5. Parameters out of physical range are rejected at the boundary rather than
   producing NaN downstream.

## 10. Explicitly out of scope

- The **series elastic element** (tendon). With a rigid tendon the teaching
  version loses nothing; adding it turns a closed-form evaluation into a root
  find, and that is a separate decision.
- Any claim about **hypertrophy, injury or prescription**. The model outputs the
  force a muscle can produce under stated assumptions. It does not know what will
  make anyone bigger or hurt them, and the UI copy must not imply otherwise.
- **Parameter personalisation** from user data. The defaults are textbook values
  for a generic muscle.

---

*Prepared per §1.3 of the Engineering Constitution — described before built. No
code has been added to this repository. The next step is the owner's answer on
D-N in §6, after which a requirement can be drafted.*
