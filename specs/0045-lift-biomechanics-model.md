# SPEC-0045 — Lift biomechanics model (`mecanica` + `core::biomech`)

- **Status:** Draft — awaiting architect review
- **Realizes:** R-0045 (Accepted 2026-09-11)
- **Author:** Claude (main session)
- **Created:** 2026-09-12
- **Depends on:** SPEC-0003 (`HeightCm`, `WeightKg`, `BodyFatPercentage`),
  SPEC-0004 (`LoadKg`), SPEC-0044 (`Calibration`, the keypoint series —
  consumed **later**, not in v1; see §2.1), physics-lab RFC-002 (the `mecanica`
  crate this vendors).
- **Module(s):** `backend/crates/mecanica/` (new workspace crate, vendored from
  physics-lab); `backend/crates/core/src/biomech/` (new — pure);
  `backend/crates/api/src/biomech/` (slice C, deferred).

## 1. Motivation

R-0045 turns a lift into mechanics: the moment at each joint, and how it moves
when the lifter changes something. It is the physics half of the direction the
owner set on 2026-09-06 — *physics is the funnel, the app is the destination* —
and it is what the @fisicobuenfisico audience already responds to.

Three facts established since the requirement was written shape this spec:

1. **The posture does not need a camera.** R-0045 says "over the R-0044
   series", but physics-lab RFC-002 §9 showed the dependency is optional:
   every model in the reels builds its posture analytically from limb lengths
   and one angle. `mecanica` already does this with 29 passing claims. v1
   takes posture from **parameters** — the user's profile and a chosen lift —
   and the keypoint path becomes a second *source* later (§2.1).
2. **`mecanica` computes the bar and nothing else.** Every `tau` in it is
   `load_kg × G × d`. AC2 requires segment masses, and they are not a
   refinement: head-arms-trunk is ~68% of body mass and adds roughly a third
   to the hip moment of a 100 kg squat (§2.4). This is the spec's largest
   addition — and a correction to the reels, which omit it too.
3. **R-0044's calibration carries ratios, not lengths.** `Calibration`
   (`technique/geometry.rs:165`) has `thigh_to_shank` and
   `shoulder_span_to_torso`. `τ = F·d` is dimensional; the keypoint path
   cannot feed this model without a scale bridge. §2.1 defines it and defers
   it.

## 2. Design

### 2.1 Parameters first, keypoints later — `Model × Source`

The model consumes a `Posture`: joint positions in **metres**, sagittal plane.
Two things can produce one:

| Source | Status | Where lengths come from |
|---|---|---|
| **Parametric** — profile + lift + knobs | **v1** | `HeightCm` × the §2.3 table, overridden by any measured segment |
| **Measured** — the R-0044 keypoint series | deferred | keypoint ratios × a scale reference (§2.1.1) |

The model never knows which produced its input. That is the `Model × Source`
separation from the guion RFC, and it is what lets v1 ship with no video, no
ML, no upload and no health-data retention.

```rust
/// A sagittal posture in metres. `+x` toward the toes, `+y` up, origin at the
/// mid-foot line. Produced by a solver (v1) or a measurement (later); the
/// model does not care which.
pub struct Posture {
    pub ankle: (f64, f64),
    pub knee: (f64, f64),
    pub hip: (f64, f64),
    pub shoulder: (f64, f64),
    /// Where the external load acts. `None` for a bodyweight-only analysis.
    pub load_at: Option<(f64, f64)>,
}
```

#### 2.1.1 The scale bridge (deferred, specified now)

A keypoint series yields angles and *ratios*. Absolute metres need one known
length. Candidates, in order of preference:

1. **A measured segment** from R-0034 body measurements or the R-0044
   calibration capture, when present.
2. **Height** from the profile via the §2.3 proportions — population-average,
   so the uncertainty is widened (R-0044 AC7's degradation rule).
3. **A plate in frame** (450 mm competition disc) — RFC-002 §9.2. Not relied on
   until verified against plates common in the target market (RFC-002 OQ-6).

None of this is built in v1. The seam is the `Posture` type: a `From<...>` for
the keypoint path is a later slice with its own claims.

### 2.2 Where `mecanica` lives, and the `Lift` collision

**Vendored as `backend/crates/mecanica`**, a third workspace member. The
Dockerfile builds with `COPY . .` from `backend/`, so a path dependency into a
sibling checkout does not exist in the build context, and a git dependency would
make the product build depend on another repository's `main`. The crate is
~900 lines with zero dependencies; a copy costs nothing and the product repo
has the stricter governance (this constitution, architect review, QA).

physics-lab consumes it back as a git dependency pinned to a fitAI commit — it
already takes `garust` as an external dependency, so there is precedent.
Canonical home: **here**.

**Not folded into `fitai-core`**, despite AC15's letter, for two reasons the
architect should weigh:

- `pub trait Lift` in `mecanica` collides with `pub enum Lift` in
  `core::technique` (merged on main). Different crates make it survivable;
  one crate makes it a rename of a shipped type.
- `mecanica` is genuinely zero-dependency. `fitai-core` pulls `chrono` and
  `uuid`. Keeping the physics in a crate that cannot even *see* a clock is a
  purity guarantee worth its own `Cargo.toml`.

The interpretation offered for AC15: a workspace crate that `fitai-core`
depends on, with no I/O and no deps, is *in the core's domain* — the criterion's
intent — without being *in the core's crate*. Flagged in §5 for the owner.

**The trait is renamed `Movement`** on vendoring. It is not yet merged
anywhere; the rename costs nothing now and a shipped enum later.

### 2.3 Anthropometrics (AC2, OQ-1)

Source: **Winter, D. A.** *Biomechanics and Motor Control of Human Movement*,
4th ed., Wiley 2009 — Table 4.1 (segment masses and centres of mass, after
Dempster 1955) and Figure 4.1 (segment lengths as fractions of height, after
Drillis & Contini 1966). The single most-cited table in the field; the values
below are the canonical ones and **must be verified against the cited edition
before the constants are committed** — AC2 forbids invented numbers, and a
transcription error here scales every torque.

Segment mass as a fraction of body mass `M`, and centre of mass as a fraction
of segment length from the **proximal** end:

| segment | mass / M | COM (proximal) | length / H |
|---|---|---|---|
| foot | 0.0145 | 0.50 | 0.152 |
| shank | 0.0465 | 0.433 | 0.246 |
| thigh | 0.100 | 0.433 | 0.245 |
| trunk (hip→shoulder) | 0.497 | 0.50 | 0.288 |
| head & neck | 0.081 | — | — |
| upper arm | 0.028 | 0.436 | 0.186 |
| forearm | 0.016 | 0.430 | 0.146 |
| hand | 0.006 | 0.506 | 0.108 |
| **HAT** (head, arms, trunk) | **0.678** | **0.626** | — |

`HAT` is what loads the hip and lumbar joints in a squat or pull: everything
above the hip, as one rigid body with its COM at 0.626 of the hip→shoulder
distance. That is the segment the squat model uses; the finer rows exist for
the bench (slice B) and for the lumbar split (§2.5).

```rust
/// One row of the table. Never constructed outside `anthro::WINTER`.
pub struct Segment {
    pub mass_fraction: f64,
    pub com_from_proximal: f64,
    pub length_fraction_of_height: f64,
}

/// Scale the table to a person. `measured` overrides any length the user has
/// actually had measured — the whole point of R-0045 AC6 is *their* limbs.
pub fn scale(height_m: f64, mass_kg: f64, measured: &MeasuredLengths) -> Body;
```

**Body fat (OQ-1, resolved):** the table does not vary by adiposity and this
spec does **not** invent an adjustment. `BodyFatPercentage` is accepted and
ignored in v1, and the result's assumption list (§2.8) says so. Any future
correction needs its own cited source.

**Individual variation:** these are population means with real spread — the
reason a long-femured lifter's squat "doesn't look like mine". Every length is
overridable by a measurement, and the uncertainty carried on a result (§2.8)
widens when the table, rather than a measurement, supplied it.

### 2.4 The static chain (AC1, AC4, OQ-2)

**Quasi-static, by decision.** At each posture, every joint is in moment
equilibrium; no accelerations. This is exact at the sticking point — where the
lifter is slowest and the coaching question lives — and **understates** the
moment during fast phases. The result carries that statement (§2.8). Full
inverse dynamics needs a time series with second derivatives, which a keypoint
series at R-0044's sampling rate cannot supply cleanly; deferred with the
measured source.

The moment at a joint is the sum, over everything distal to it on the load
side, of *weight × horizontal offset from the joint*:

```
τ_J = Σ_segments  m_s · g · (x_COM,s − x_J)   +   m_load · g · (x_load − x_J)
```

Signed: positive means the load tends to flex the joint and the extensors must
resist. For the squat at the bottom:

```
τ_hip  = HAT weight × (x_HAT − x_hip)  +  bar weight × (x_bar − x_hip)
τ_knee = (HAT + thigh) weight × offsets   +  bar weight × (x_bar − x_knee)
τ_ankle = everything above the ankle
```

**What this does to the reel's numbers**, and why it must be stated: MECÁNICA 01
publishes 274 N·m at the hip for a 100 kg bar and a 0.40 m femur. With an 80 kg
lifter at 1.75 m, `HAT = 0.678 × 80 = 54.2 kg`, its COM sits `0.626 × 0.53 ×
sin 32° = 0.176 m` ahead of the hip, adding `54.2 × 9.81 × 0.176 ≈ 94 N·m`.
**The honest figure is ~368 N·m, not 274.** The reel is a bar-only model and
says so nowhere. This is a third correction the crate has surfaced (after the
kickback peak and the missing segment weight in the reels generally), and it
belongs in a published correction, not only here.

The `Movement` trait and `work`/`peak`/`zero_crossing` are unchanged from
RFC-002; the moment function gains a `&Body` and a `Posture` instead of bare
lengths. `Sentadilla`'s bottom-position solve (`resolver_sentadilla` lineage)
becomes the v1 posture solver.

### 2.5 The lumbar moment (OQ-3) — and what it cannot see

**Approximation:** a single joint at **L5/S1**, placed on the trunk line at
`0.10 × trunk length` above the hip joint centre (Winter's convention for the
lumbosacral joint relative to the greater trochanter — verify with the table).
The trunk above it is one rigid body. The lumbar moment is the moment of
everything above L5/S1 — trunk-above-L5, head, arms, bar — about that point.

This is the quantity textbooks compute and it is legitimate. It is also
exactly the moment the **spinal erectors must produce to hold the trunk rigid
at that angle** — which is the correct muscle-group attribution (AC5).

**What it is not.** R-0044 AC10b established that COCO-17 has no spine landmark
and back rounding is not detectable. A rigid-trunk lumbar moment **cannot
detect rounding either** — a neutral and a flexed spine at the same shoulder→hip
line yield the same number. The result must say "assuming a rigid trunk" and
must never be rendered as a spine-health figure. A rounded spine changes the
*internal* moment arms; this model does not know that, and says so.

**Stated error:** the L5/S1 offset is a population constant; the true joint
varies by several centimetres between people, which shifts the lumbar moment
by the corresponding fraction of the HAT weight's lever. Carried as an
uncertainty term on the lumbar row only.

### 2.6 Variants (AC6–AC9, OQ-4, OQ-5)

A variant is **the lifter's own posture with one or more knobs changed**, then
re-solved. The comparison is always against the actual posture (AC6), and the
headline is the **delta** (AC8) — because the table-derived masses carry the
absolute error and it largely cancels between two postures of the same body.

#### 2.6.1 The knobs, and how each changes a sagittal posture

| knob (AC7) | sagittal effect | range | source of range |
|---|---|---|---|
| **bar position** | where the load attaches on the trunk line: high-bar ≈ 0.95, low-bar ≈ 0.80 of hip→shoulder; front ≈ 0.98 with the arm offset | 0.75–1.00 | anatomical: below 0.75 is off the scapula |
| **torso angle** | direct knob on the solve; bar-over-midfoot then fixes the hip | 0°–70° from vertical | beyond 70° no squat pattern balances |
| **stance width** | abducts the femur; its *sagittal projection* shortens to `femur × cos(abduction)`, where `abduction = asin((stance − hip_width)/2 / femur)`. This is the sumo-vs-conventional effect | 0.8–2.2 × hip width | past 2.2 the abduction solve leaves its domain |
| **foot rotation** | **none representable** — see below | — | — |

**Foot rotation is frontal/transverse-plane.** A sagittal model cannot express
it honestly, and AC7 lists it "at minimum". This spec **cannot satisfy AC7 as
written** and asks for the same treatment SPEC-0044 gave three impossible
measurements: an amendment deferring foot rotation to a 3D model, recorded in
R-0045's changelog. Rendering a number for it would be the fabrication AC9
forbids.

#### 2.6.2 The solve (OQ-4, resolved: kinematic, not interpolated)

Interpolating between measured templates would mean the model contains
postures the lifter never held. The solve is **kinematic**: given segment
lengths and knobs, place the ankle, solve the knee from shank angle, the hip
from the femur's sagittal projection, and the trunk angle from the constraint
that the bar's vertical passes through the mid-foot — `resolver_sentadilla`'s
construction, generalised. One equation, one unknown, closed form.

#### 2.6.3 Rejection (AC9) — a closed set

```rust
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum Rejection {
    /// A knob is outside its §2.6.1 range.
    OutOfRange { knob: Knob, value: f64, min: f64, max: f64 },
    /// The bar-over-midfoot constraint has no solution: asin domain exceeded.
    /// The lifter would have to lean past horizontal.
    DoesNotBalance,
    /// A solved joint angle is outside physiological range
    /// (knee flexion > 160°, hip flexion > 130°, ankle dorsiflexion > 45°).
    AnatomicallyImplausible { joint: Joint, angle_deg: f64 },
    /// The stance abduction solve leaves its domain.
    StanceExceedsFemur,
}
```

Exhaustive `match` in the token mapper, no `_` arm — the R-0042 precedent. The
joint limits are from Winter Table A.1 / standard goniometric norms and need
the same cited-source verification as §2.3.

#### 2.6.4 Delta stability (AC16)

Small keypoint noise must not flip the sign of a delta. The test perturbs every
joint by ±1 cm (MoveNet's error scale from SPEC-0044) and asserts the delta's
sign and its magnitude within the carried uncertainty. If a delta *is* inside
noise, the result says `Borderline` rather than reporting a direction — the
R-0044 AC10c rule.

### 2.7 Muscle groups, never muscles (AC5)

Each joint moment is attributed to the **group** that resists it, and nothing
finer:

| joint | resisting group |
|---|---|
| hip | hip extensors |
| knee | knee extensors |
| ankle | plantar flexors |
| L5/S1 | spinal erectors |

There is no per-muscle split anywhere in the model, and the type system makes
it hard to add one by accident: `MuscleGroup` is a closed enum with four
variants and no `Muscle` type exists. RFC-002 §7 explains why — one equation,
*n* unknowns — and §7.6 what it would take to change that honestly (EMG-driven,
with the cost function displayed). Neither is in scope.

### 2.8 Every result states its assumptions (AC12) and refuses honestly (AC13)

```rust
pub struct Mechanics {
    pub joints: [JointMoment; 4],       // hip, knee, ankle, lumbar — fixed order
    pub load: LoadBasis,                 // Bar(kg) | BodyweightOnly
    pub assumptions: &'static [Assumption],
    pub uncertainty: Uncertainty,
}

/// Rendered on every result, not in a help page.
pub enum Assumption {
    SagittalPlane,
    QuasiStatic,
    RigidSegments,
    RigidTrunkLumbar,        // "cannot detect spinal rounding"
    TableDerivedMasses,      // present unless every length was measured
    BodyFatIgnored,          // §2.3
}
```

`LoadBasis::BodyweightOnly` is what AC3 requires when no logged set supplies a
load: the analysis runs, reports bodyweight mechanics, and says that is what it
did. **It never assumes a bar.**

AC13 for the parametric source has nothing upstream to inherit; its refusals
*are* §2.6.3. For the measured source (later), any R-0044 `Refusal` propagates
unchanged — mechanics from refused keypoints are not computed.

**AC14** is enforced at the type level: there is no `risk`, `danger`, `safe`
or `injury` field anywhere in `Mechanics`, and the prose fields are cues
(`"your hip does 18% less here"`), never verdicts.

### 2.9 Persistence and export (AC10, AC11, OQ-6) — slice C, deferred

A saved variant is per-user, author-only, `404` never `403`, via the single
`load_owned` pattern from SPEC-0041. Storage reuses `authored_programs`
(AC10) — **but** the shape a variant attaches to (a `MaterializedEntry`? a
lift on a day?) needs R-0041's model read against this one, and OQ-6's answer
depends on R-0040 roles, which do not exist. Slice C is specified when both are
clearer. Nothing in slices A/B writes to a database.

### 2.10 Slicing

| slice | contents | depends on |
|---|---|---|
| **A** | vendor `mecanica` → `Movement`; `anthro` table; `Posture`; the squat with segment weights and the lumbar row; `Rejection` | nothing |
| **B** | variants: the knobs, the kinematic solve, deltas, `Borderline`; the bench and the pull | A |
| **C** | `api::biomech` endpoints, persistence, coach export | B, R-0041 read, R-0040 |

Slice A is the one that changes the published numbers and needs the reel
correction to go out with it.

## 3. Code outline

```
backend/crates/mecanica/               vendored; trait Lift -> Movement
backend/crates/core/src/biomech/
  mod.rs        Posture, Mechanics, JointMoment, LoadBasis, Assumption
  anthro.rs     WINTER table, Segment, Body, scale(), MeasuredLengths
  chain.rs      moment_about(joint, &Body, &Posture, load) — the Σ m·g·Δx
  lumbar.rs     l5s1_point(), the rigid-trunk lumbar moment
  squat.rs      solve(knobs, &Body) -> Result<Posture, Rejection>
  variant.rs    (slice B) Knobs, apply(), Delta, Borderline
```

Representative — the whole model is this shape, and `chain::moment_about` is
the only place gravity is multiplied:

```rust
/// Σ over everything on the load side of `joint`: weight × horizontal lever.
/// Signed: positive flexes the joint, so the extensors resist it.
pub fn moment_about(joint: Joint, body: &Body, p: &Posture, load: LoadBasis) -> f64 {
    let (xj, _) = p.at(joint);
    let segments = body.distal_of(joint);
    let mut tau = 0.0;
    for s in segments {
        tau += s.mass_kg * G * (s.com_x(p) - xj);
    }
    if let (LoadBasis::Bar(kg), Some((xl, _))) = (load, p.load_at) {
        tau += kg * G * (xl - xj);
    }
    tau
}
```

`body.distal_of(joint)` is a fixed list per joint — hip: `[HAT]`; knee:
`[HAT, Thigh]`; ankle: `[HAT, Thigh, Shank]`; lumbar: `[TrunkAboveL5, Head,
Arms]` — not a graph walk. Four joints, four lists, no abstraction.

## 4. Non-goals

- Per-muscle force or activation (AC5, RFC-002 §7.3). No `Muscle` type exists.
- Foot rotation, knee valgus, grip width as an abduction — anything frontal or
  transverse. Sagittal only, and the result says so.
- Full inverse dynamics with acceleration terms (OQ-2 resolved: quasi-static).
- The measured (keypoint) source, and the scale bridge (§2.1.1). Seam only.
- Coach→client sharing (needs R-0040). Slice C at most stores the author's own.
- Any injury, safety or risk output (AC14). Not a field, not a word.
- Body-fat-dependent segment parameters (§2.3): no source, so no adjustment.

## 5. Open questions

### Resolved here (owner may veto)

- **OQ-1** → Winter Table 4.1 / Fig 4.1, verified against the cited edition
  before commit; body fat ignored with the assumption stated. §2.3.
- **OQ-2** → quasi-static; error stated on every result. §2.4.
- **OQ-3** → single L5/S1 joint, rigid trunk, cannot see rounding, says so. §2.5.
- **OQ-4** → kinematic solve, never interpolation. §2.6.2.
- **OQ-5** → ranges in §2.6.1 with sources; two are anatomical bounds, one is
  a solve-domain bound.

### Still open — need the owner

- **OQ-6** — variant ownership. Deferred with slice C; depends on R-0040.
- **AC7 amendment.** Foot rotation is not representable in a sagittal model.
  Request: amend AC7 to "stance width, bar position, torso angle" and defer
  foot rotation to a 3D model, as SPEC-0044 did for its impossible criteria.
- **AC15 interpretation.** `mecanica` as a workspace crate `fitai-core`
  depends on, rather than a module inside it. §2.2 gives the reasons.
- **Canonical home of `mecanica`** — here, with physics-lab consuming by git
  dependency. Reverses RFC-002's implicit assumption; §2.2 says why.
- **The published correction.** §2.4's ~368 vs 274 N·m is a real error in a
  published reel. Whether and how to correct it publicly is the owner's.

## 6. Acceptance criteria

Each maps to R-0045; the qa agent owns the tests.

- [ ] **AC1** — `chain::moment_about` is the only gravity term; no engine, no
      stepping. Verified by the crate's dependency list: none.
- [ ] **AC2** — every constant in `anthro::WINTER` carries a table/row citation
      in its doc comment and matches the cited edition (verified by hand, once).
- [ ] **AC3** — `LoadBasis::BodyweightOnly` path produces a result whose
      `load` says so and whose moments contain no bar term.
- [ ] **AC4** — `Mechanics.joints` is exactly `[hip, knee, ankle, lumbar]`,
      each with `peak_nm` and `at_phi`.
- [ ] **AC5** — `MuscleGroup` has four variants; grep for `Muscle` as a type
      finds nothing.
- [ ] **AC6/AC8** — `Delta` is computed from the lifter's own `Posture`;
      the headline field is the percentage change, absolutes are secondary.
- [ ] **AC7** — three knobs with ranges; `OutOfRange` on every bound. Foot
      rotation: pending amendment.
- [ ] **AC9** — every `Rejection` variant is reachable by a test.
- [ ] **AC12** — `assumptions` is non-empty on every result; `RigidTrunkLumbar`
      is present whenever the lumbar row is.
- [ ] **AC13** — parametric: rejections are typed; measured: deferred.
- [ ] **AC14** — no field or string in the output namespace matches
      `/risk|danger|safe|injur/i`.
- [ ] **AC15** — `mecanica` and `core::biomech` compile with no I/O crates.
- [ ] **AC16** — the test plan below, in full.

## 7. Test plan

Claims, in the physics-lab sense — each is a `#[test]` that names what it
proves.

**Hand-computed statics (AC16 "hand-verifiable"):**
- `B1` a vertical trunk over the hip, no bar → hip moment ≈ 0 (HAT COM above
  the joint). Bench for the sign convention.
- `B2` trunk at 30° from vertical, known HAT, no bar → `τ_hip = HAT·g·0.626·L·sin30°`
  exactly, by hand.
- `B3` the MECÁNICA 01 setup with an 80 kg / 1.75 m lifter → bar term reproduces
  274 N·m **and** the total is ~368 N·m. The reel's number is recovered as the
  bar-only component, so the correction is derived, not asserted.
- `B4` knee moment is invariant to femur length at fixed shank geometry
  (RFC-002 C2, now with segment weight included — must still hold, because the
  bar and the HAT both move with the hip, not the knee).

**Symmetry (AC16):**
- `B5` a symmetric posture (bar directly over hip, trunk vertical) gives equal
  and opposite moments about mirrored points.

**Documented shifts (AC16):**
- `B6` low-bar vs high-bar: hip moment ↑, knee moment ↓ — the direction every
  textbook gives; asserted as a sign, not a magnitude.
- `B7` wider stance: femur's sagittal projection shortens → hip moment ↓.

**Rejection (AC9):**
- `B8`–`B11` one test per `Rejection` variant, each reaching it through the
  public solve.

**Stability (AC16):**
- `B12` ±1 cm perturbation of every joint: every delta keeps its sign or is
  reported `Borderline`.

**Bodyweight-only (AC3, AC16):**
- `B13` `LoadBasis::BodyweightOnly` → result has no bar term and the load
  field says so.

**Honesty:**
- `B14` every result's `assumptions` contains `SagittalPlane` and
  `QuasiStatic`; contains `TableDerivedMasses` iff any length came from the
  table.
- `B15` the lumbar row is present ⇒ `RigidTrunkLumbar` is present.

## 8. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-09-12 | Parametric source first; keypoints later | The posture needs no camera (RFC-002 §9); v1 ships with no video, ML, upload or retention. |
| 2026-09-12 | Vendor `mecanica` into the workspace; rename `Lift` → `Movement` | `COPY . .` build context; `Lift` collides with a merged enum; zero-dep purity kept in its own crate. |
| 2026-09-12 | Quasi-static | Exact at the sticking point; error stated on every result; acceleration needs a series the sampler cannot supply cleanly. |
| 2026-09-12 | Single L5/S1 joint, rigid trunk | The textbook quantity; **cannot see rounding**, and the result says so — consistent with R-0044 AC10b. |
| 2026-09-12 | Body fat ignored in v1 | The cited table does not vary by adiposity; inventing an adjustment would violate AC2. |
| 2026-09-12 | Kinematic solve for variants, never interpolation | Interpolated postures are ones the lifter never held. |
| 2026-09-12 | Foot rotation: request AC7 amendment rather than fabricate | Frontal-plane; a sagittal number for it would be invented. |

## Changelog

- _2026-09-12 — created (Draft)._
