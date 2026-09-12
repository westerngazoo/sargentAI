# SPEC-0045 — Lift biomechanics model (`core::biomech`)

- **Status:** **Accepted** (owner, 2026-09-12) — architect ACCEPT WITH
  CHANGES applied (§10); all three §5 decisions taken as recommended
- **Realizes:** R-0045 as amended 2026-09-12 (§5.1 applied to the
  requirement file)
- **Author:** Claude (main session)
- **Created:** 2026-09-12
- **Depends on:** SPEC-0003 (`HeightCm`, `WeightKg`, `BodyFatPercentage`),
  SPEC-0004 (`LoadKg`), SPEC-0044 (`Calibration`, the keypoint series — a
  **later** source, not v1; §2.1). physics-lab RFC-002 is prior art, not a
  dependency (§2.2).
- **Module(s):** `backend/crates/core/src/biomech/` (new — pure);
  `backend/crates/api/src/biomech/` (slice C, deferred).

## 1. Motivation

R-0045 turns a lift into mechanics: the moment at each joint, and how it moves
when the lifter changes something. It is the physics half of the direction the
owner set on 2026-09-06 — *physics is the funnel, the app is the destination*
(`ROADMAP.md`, Current state).

Three facts established since the requirement was written shape this spec,
and the first of them means **the requirement's wording no longer matches
the direction it was accepted under** (§5.1):

1. **The posture does not need a camera.** R-0045 is worded for a measured
   source — "over the R-0044 series", "the clip", "per rep". physics-lab
   RFC-002 §9 showed the dependency is optional: every reel model builds its
   posture analytically from limb lengths and one angle. v1 takes posture from
   **parameters** and the keypoint path becomes a second *source* later (§2.1).
   That is what lets it ship with no video, no ML, no upload and no
   health-data retention — and it is an owner decision already recorded in
   the roadmap, not one this spec makes.
2. **The reel models compute the bar and nothing else.** AC2 requires segment
   masses, and they are not a refinement: head-arms-trunk is ~68% of body mass
   and adds roughly a third to the hip moment of a 100 kg squat (§2.4). This is
   the spec's largest addition — and a correction to a published reel.
3. **R-0044's calibration carries ratios, not lengths.** `τ = F·d` is
   dimensional; the keypoint path cannot feed this model without a scale
   bridge. §2.1.1 defines it and defers it.

## 2. Design

### 2.1 Parameters first, keypoints later — `Model × Source`

The model consumes a `Posture`: joint positions in **metres**, sagittal plane,
plus where the load acts. Two things can produce one:

| source | status | where lengths come from |
|---|---|---|
| **parametric** — profile + lift + baseline conventions | **v1** | `HeightCm` × the §2.3 table |
| **measured** — the R-0044 keypoint series | deferred | keypoint ratios × a scale reference (§2.1.1) |

The model never knows which produced its input. That is the `Model × Source`
separation, and `moment_about(&Posture)` is source-agnostic by construction.

```rust
/// Metres, sagittal plane. `+x` toward the toes, `+y` up, origin on the
/// mid-foot line at floor level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point { pub x: Metres, pub y: Metres }

/// A posture the model can load. Produced by the v1 solver or, later, by a
/// measurement; the model does not care which.
pub struct Posture {
    pub ankle: Point,
    pub knee: Point,
    pub hip: Point,
    pub shoulder: Point,
    /// The external load, with the point it acts at. A bar with no position
    /// is unrepresentable (architect finding 9): `None` is bodyweight-only,
    /// `Some` carries both the mass and where it is.
    pub load: Option<Load>,
}

pub struct Load { pub kg: f64, pub at: Point }
```

**v1 is a single posture.** The parametric squat is solved at the bottom
position only — `mecanica::sentadilla` refuses to implement a φ-axis trait
precisely because no published model validates a descent kinematic. So there
is no "through the movement", no peak, no "per rep" in v1. Every result carries
`Assumption::SinglePosture { at: Position::Bottom }`, and AC4's "through the
movement / per rep" is routed to the measured source in §5.1. Claiming a peak on
a single posture would be a fabricated axis (architect finding 2).

#### 2.1.1 The scale bridge (deferred, specified now)

A keypoint series yields angles and *ratios*. Absolute metres need one known
length. Candidates, in order of preference:

1. **A measured segment.** **No source for one exists in v1.** Body
   measurements shipped in PR #52 without a requirement file — there is no
   R-0034 to cite, and R-0044's calibration capture is itself deferred. Every
   v1 result therefore carries `Assumption::TableDerivedLengths` (§2.8); the
   override seam (`MeasuredLengths`) exists so the day a source lands, nothing
   in the model changes.
2. **Height** from the profile via the §2.3 proportions — population-average,
   uncertainty widened accordingly.
3. **A plate in frame** (450 mm competition disc) — RFC-002 §9.2, not relied on
   until verified against plates common in the target market.

None of this is built in v1. The seam is `Posture`; a `From<LiftSeries>` for it
is a later slice with its own claims.

### 2.2 Where the physics lives — no vendoring

The draft proposed vendoring physics-lab's `mecanica` crate. The architect
rejected it (finding 5), correctly, on three grounds:

- Two of its four modules are cable-kickback and hip-thrust **reel models with
  reel constants**. No `R-NNNN` asks for them. CLAUDE.md §1.1 and §2 ("no dead
  code") forbid shipping them regardless of how cheap the copy is.
- The `Lift` collision argument does not survive contact with `lib.rs`:
  `technique::Lift` is module-scoped and not re-exported at the crate root.
  Modules already namespace.
- The actual R-0045 physics — anthropometrics, chain, lumbar, solve — does not
  exist in `mecanica` at all. There was nothing to vendor *for this
  requirement* except `G`.

**Decision: `fitai_core::biomech` is the single home**, satisfying AC15 as
written. `G = 9.81` is one constant; the reel-correction claim (B3) is one
test; both are trivially duplicated. physics-lab keeps its `mecanica` as the
lesson runtime's crate — a **teaching** model that is bar-only *by design*, for
reels whose whole point is one variable at a time. The product model includes
segment weights. They are supposed to differ, and the reel correction (§2.4) is
the bridge between them.

The alternative — a zero-dependency workspace crate holding *only* R-0045's
model, which physics-lab consumes by pinned git dependency — keeps one
implementation but needs an AC15 amendment, a cross-repo build dependency on
`westerngazoo/sargentAI` (private; physics-lab's CI would need credentials),
and a `Movement` trait with no implementor in slice A. It is the right shape
**if and when slice B introduces a φ axis**, and is recorded as owner decision
§5.2 rather than taken here.

### 2.3 Anthropometrics (AC2, OQ-1)

Source: **Winter, D. A.** *Biomechanics and Motor Control of Human Movement*,
4th ed., Wiley 2009 — Table 4.1 (segment masses and centres of mass, after
Dempster 1955) and Figure 4.1 (segment lengths and breadths as fractions of
height, after Drillis & Contini 1966).

**AC2 control (architect finding 17).** "Verify before commit" is an obligation
with no artifact. The control is two-fold: (i) the verification — edition,
page, date, who — is recorded in this section before any constant is
committed, and is blank until then; (ii) the table is **self-checking** by
claim tests that must hold whatever the values are:

```
HAT + 2·(thigh + shank + foot)                          = 1.000
trunk + head&neck + 2·(upper arm + forearm + hand)      = HAT
thorax&abdomen + pelvis                                 = trunk
```

> **Verification record:** *pending.* The values below are the canonical ones
> and pass the sums above, but a recollection is not the control.

Mass as a fraction of body mass `M`; centre of mass as a fraction of segment
length from the **proximal** end; length as a fraction of height `H`:

| segment | proximal → distal | mass / M | COM (proximal) | length / H |
|---|---|---|---|---|
| foot | ankle → toe | 0.0145 | 0.50 | 0.152 |
| shank | knee → ankle | 0.0465 | 0.433 | 0.246 |
| thigh | hip → knee | 0.100 | 0.433 | 0.245 |
| pelvis | L4-L5 → greater trochanter | 0.142 | 0.105 | — |
| thorax & abdomen | C7-T1 → L4-L5 | 0.355 | 0.63 | — |
| trunk (= pelvis + thorax&abdomen) | shoulder → hip | 0.497 | 0.50 | 0.288 |
| head & neck | C7-T1 → vertex | 0.081 | — | 0.130 (vertex→chin) |
| upper arm | shoulder → elbow | 0.028 | 0.436 | 0.186 |
| forearm | elbow → wrist | 0.016 | 0.430 | 0.146 |
| hand | wrist → knuckle | 0.006 | 0.506 | 0.108 |
| **HAT** | greater trochanter → glenohumeral | **0.678** | **0.626** | — |
| hip breadth | — | — | — | 0.191 |
| shoulder breadth | — | — | — | 0.259 |
| ankle height | floor → ankle joint | — | — | 0.039 |

`hip breadth` and `ankle height` are inputs to the solve (§2.6) and were
missing from the draft (findings 11, 15). The trunk split (pelvis / thorax &
abdomen) is what the lumbar row needs (§2.5) and was also missing (finding 7).

```rust
/// One row. Never constructed outside `anthro::WINTER`; each constant's doc
/// comment carries its table/figure reference.
pub struct Segment {
    pub mass_fraction: f64,
    pub com_from_proximal: f64,
    pub length_fraction_of_height: Option<f64>,
}

/// Scale the table to a person. `measured` overrides any LENGTH the user has
/// had measured. Masses have no measurement path and are always table-derived.
pub fn scale(height: Metres, mass_kg: f64, measured: &MeasuredLengths) -> Body;
```

**Body fat (OQ-1, resolved):** the table does not vary by adiposity and this
spec does **not** invent an adjustment. `BodyFatPercentage` is accepted and
ignored in v1; the result says so (`Assumption::BodyFatIgnored`).

**Individual variation:** these are population means with real spread — the
reason a long-femured lifter's squat "doesn't look like mine". Every *length*
is overridable by a measurement; every *mass* is not (finding 8), and the
result's assumptions say which is which.

### 2.4 The static chain (AC1, AC4, OQ-2)

**Quasi-static, by decision.** At each posture, every joint is in moment
equilibrium; no accelerations. Exact at the sticking point — where the lifter
is slowest and the coaching question lives — and **understates** the moment
during fast phases. Stated on every result.

#### 2.4.1 The moment, and its sign (architect finding 3)

The net moment about joint `J` from everything on the load side of it:

```
M_J = Σ_s  m_s · g · (x_COM,s − x_J)   +   m_load · g · (x_load − x_J)
```

That raw sum's sign is a *coordinate* fact, not a physiological one. On a
zig-zag chain at the squat bottom the load line is at `x = 0`, the hip at
`x ≈ −0.28` and the knee at `x ≈ +0.12`: the raw sum is positive at the hip
and **negative at the knee**, yet both are flexion moments the extensors must
resist. The draft asserted a uniform meaning for the raw sign; that was wrong,
and a sign-based test would have encoded it.

**The output is defined per joint as the moment resisting extension**, always
reported so that positive means "the extensors are loaded". The per-joint
orientation is applied in **exactly one place**:

```rust
impl Joint {
    /// +1 where a load ahead of the joint flexes it (hip, lumbar, ankle);
    /// −1 where a load ahead of the joint extends it (knee).
    fn extension_sign(self) -> f64 { ... }
}
/// N·m resisting extension. Positive: extensors loaded. Negative: the load
/// is assisting extension — which is a real state (a bar behind the hip).
pub fn moment_about(j: Joint, body: &Body, p: &Posture) -> NewtonMetres {
    j.extension_sign() * raw_sum(j, body, p)
}
```

Signed, never `.abs()`: the sign is what reveals a load that assists rather
than resists, and RFC-002 already established why that must survive.

#### 2.4.2 What the load side contains

Four joints, four fixed lists — a serial chain summed from the top, no graph
walk (the architect confirmed this is the right level for the squat and the
pull, and the wrong one for a bench — §5.3):

| joint | `above(joint)` |
|---|---|
| ankle | shank, thigh, pelvis, thorax&abdomen, head, arms, load |
| knee | thigh, pelvis, thorax&abdomen, head, arms, load |
| hip | pelvis-above-hip*, thorax&abdomen, head, arms, load |
| L5/S1 | thorax&abdomen, head, arms, load |

\* the pelvis segment spans L4-L5 → greater trochanter and its COM sits
0.105 from L4-L5, i.e. almost entirely above the hip; it is included in the
hip's load side in full. Stated as a convention with its error term.

**Head and arm placement** (finding 7): `Posture` has no head or hand points,
so the two are placed by convention from the four joints it does have —

- **head & neck COM**: on the trunk line extended beyond the shoulder by
  `0.5 × 0.130 H`. Convention; error carried.
- **arms**: hands are on the bar in a back squat, so each arm's COM is placed
  at the midpoint of shoulder → load point. When there is no load, arms hang:
  COM directly below the shoulder at `0.436 × upper-arm length`. Convention;
  error carried.

Winter's lumped HAT (COM 0.626) assumes **arms at the sides**. With hands on
the bar the arm mass sits higher and further forward, so the split
underestimates less than the lump does — the true hip correction is *larger*
than a HAT-only figure. This is why v1 uses the split, not the lump.

#### 2.4.3 What this does to the reel's numbers

MECÁNICA 01 publishes **274 N·m** at the hip for a 100 kg bar and a 0.40 m
femur. That is the **bar-only** component. With an 80 kg lifter and the reel's
own segment lengths as overrides (finding 16 — table proportions at 1.75 m
give a 0.429 m femur, so height must scale only the *unmeasured* segments),
the trunk solves to 31.75° and the HAT term is `0.678 M · g · 0.626 · d_hip`
exactly: **92.9 N·m** (finding 18). The honest hip moment is **≈ 366 N·m**,
and higher still once the arms are placed on the bar.

The reel is a bar-only model and says so nowhere. Test B3 recovers 274 as the
bar component and *derives* 366 from it, so the published correction is a
computed result, not an assertion. Whether to publish it is the owner's (§5.4).

### 2.5 The lumbar row (OQ-3) — and what it cannot see

**Placement.** A single joint at **L5/S1**, on the trunk line above the hip
joint centre. The draft attributed a `0.10 × trunk` offset to "Winter's
convention"; it is not in Table 4.1 (finding 7). It is declared here as a
**convention**, with candidate sources to verify against before the constant
is committed — de Leva (1996), *Adjustments to Zatsiorsky-Seluyanov's segment
inertia parameters*, and Chaffin, Andersson & Martin, *Occupational
Biomechanics*, 2D static model — and an error term: the true joint varies by
several centimetres between people, shifting the lumbar moment by that fraction
of the load-side lever. Carried on the lumbar row's uncertainty only.

**Load side.** Everything above L5/S1: thorax & abdomen (0.355 M), head (0.081
M), arms (0.100 M), and the load — **≈ 0.536 M**, not the 0.678 HAT lump
(finding 7). The L4-L5 / L5-S1 boundary mismatch is absorbed into the stated
error.

**What it is.** The **net extensor moment at L5/S1, attributed to the erector
group** (finding 19) — passive tissue and intra-abdominal pressure share it,
so it is not "exactly what the erectors produce".

**What it is not.** R-0044 AC10b established that COCO-17 has no spine landmark
and back rounding is not detectable. A rigid-trunk lumbar moment **cannot
detect rounding either** — a neutral and a flexed spine at the same
shoulder→hip line yield the same number. The result carries
`Assumption::RigidTrunkLumbar` and must never be rendered as a spine-health
figure.

### 2.6 The solve, the baseline, and the knobs (AC6–AC9, OQ-4, OQ-5)

#### 2.6.1 Baseline conventions (architect finding 11)

The v1 "actual" posture — what every delta is measured from — is **solved**,
not measured. AC6's "the client's own lift" is therefore *their own lengths
and load* under a **stated baseline**, and the result says so
(`Assumption::SolvedBaseline`). The baseline conventions and their provenance:

| quantity | value | provenance |
|---|---|---|
| thigh angle at the bottom | parallel (femur horizontal) | convention: the depth standard R-0044 AC8 also uses |
| ankle joint height | `0.039 H` | Winter Fig 4.1 |
| ankle x (behind the mid-foot line) | `−0.26 × foot length` = `−0.040 H` | derived: mid-foot at half foot length, ankle at the heel-side quarter — **convention**, replaces `muneco.py`'s `−0.04 m` literal |
| shank angle from vertical | **the unknown** (§2.6.3) — *not* the reel's fixed 22° | — |
| bar attachment on the trunk line | `0.95 × trunk` (high-bar) | convention; the low-bar value is a knob (§2.6.2) |
| hands | on the bar | convention |

The reel's `22°` shank and `(−0.04, 0.09)` ankle were literals from
`muneco.py` with no source; they are replaced, and where a literal survives it
is labelled a convention rather than a fact.

#### 2.6.2 The knobs (AC7) — slice B, none in slice A

| knob | sagittal effect | range | range provenance |
|---|---|---|---|
| **bar position** | attachment point along the trunk line: high-bar 0.95, low-bar 0.80 | 0.75–1.00 | anatomical: below ~0.75 of hip→shoulder the bar is off the scapular shelf — **stated convention** |
| **torso angle** | fixes θ; the shank angle becomes the unknown (§2.6.3) | 10°–65° from vertical | **stated convention** pending citation (Escamilla et al. 2001, squat trunk-angle data, is the candidate). The draft's "beyond 70° nothing balances" was false — a 70° trunk balances as a good-morning; and 0° is unreachable at parallel |
| **stance width** | abducts the femur; its sagittal projection shortens to `femur × cos α`, `α = asin((stance − hip_breadth) / 2 / femur)` | 1.0–2.2 × hip breadth | **stated convention** pending citation (Escamilla et al. 2000, sumo vs conventional, is the candidate). The draft claimed 2.2 was a solve-domain bound; the solve's true domain is `stance < hip_breadth + 2·femur ≈ 3.6×` — so the bound is a convention, not physics |
| **foot rotation** | **none representable** — §5.1 | — | — |

**Stance carries two stated limits** (finding 15): the projection assumes the
knee tracks over the foot in the frontal plane (`Assumption::KneesTrackFeet`),
and the abducted femur carries a **frontal-plane hip moment this model does
not compute** — "wider stance → hip moment ↓" is a sagittal-projection
statement and the result must say so, or a coach reads it as "sumo loads the
hips less".

**Front squat is not a bar-position knob** (finding 23): its load is a
perpendicular offset from the trunk line, not a fraction along it. A different
lift; dropped from v1.

**Foot rotation is frontal/transverse-plane.** Toe-out changes the knee's
tracking direction, which a sagittal model cannot observe — unlike stance
width, whose effect *projects* onto the sagittal plane under the stated
tracking assumption. Rendering a number for it would be the fabrication AC9
forbids. Amendment requested (§5.1).

#### 2.6.3 The solve (OQ-4) — kinematic, closed-form, and which unknown

Interpolating between measured templates would put postures the lifter never
held into the model. The solve is **kinematic**: place the ankle (§2.6.1),
solve the knee from the shank angle and the hip from the femur's sagittal
projection, then close the chain with **one balance equation in one unknown**.

**The balance constraint** (finding 4): the draft used "bar over the
mid-foot", which does not exist when there is no bar. The honest condition is
**the combined centre of mass of everything above the ankle — segments and
load — over the mid-foot line**. It is linear in `sin θ` (every load-side term
is proportional to it), so it stays closed-form, and bar-over-mid-foot is
recovered as the limit where bar mass dominates. Mid-foot itself is a coaching
convention within the base of support; stated.

**Which unknown** (finding 12): if a torso angle is specified, the **shank
angle** is solved (hence knee travel, hence the knee moment); otherwise the
**trunk angle** θ is. Bar-position and stance knobs solve θ. This rule
determines which rejection fires and is what makes the solver's behaviour
fully specified rather than implementation-defined.

#### 2.6.4 Rejection (AC9) — a closed set, every variant reachable

```rust
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum Rejection {
    /// A knob is outside its §2.6.2 range. Fires first, so the ranges
    /// bound what the solve ever sees.
    OutOfRange { knob: Knob, value: f64, min: f64, max: f64 },
    /// The balance equation has no solution in the unknown's domain: the
    /// lifter would have to lean past horizontal (θ), or the knee would
    /// have to travel past the foot's reach (shank angle).
    DoesNotBalance { unknown: Unknown },
    /// A solved joint angle is outside physiological range.
    AnatomicallyImplausible { joint: Joint, angle_deg: f64 },
}
```

The draft's `StanceExceedsFemur` is folded into `OutOfRange` — with the range
at 2.2× and the solve domain at 3.6×, it could never fire (finding 6).
Joint limits (knee flexion > 160°, hip flexion > 130°, ankle dorsiflexion >
45°) are **stated conventions** pending a goniometric source — the draft cited
"Winter Table A.1", which is gait data, not norms (finding 16); Norkin & White
or the AAOS tables are the candidates.

Exhaustive `match` in the token mapper, no `_` arm — the R-0042 precedent.

#### 2.6.5 Deltas and stability (AC8, AC16)

A variant is the lifter's own baseline with knobs changed, re-solved. The
headline is the **delta** — the table-derived masses carry the absolute error
and it largely cancels between two postures of the same body.

**Stability under what?** The draft perturbed keypoints, which v1 does not
have (finding 13). For the parametric source, stability is under
**segment-length perturbation**: every table-derived length ±5% (a stated
convention for population spread pending a cited value), and every delta must
keep its sign or be reported `Borderline`. The keypoint version (±2 cm, SPEC-
0044's σ) belongs to the measured slice.

### 2.7 Resisting groups, never muscles (AC5)

Each joint moment is attributed to the **group** that resists it, and nothing
finer. Named `ResistingGroup` — `fitai_core::MuscleGroup` already exists
(`workout.rs:22`, re-exported at the root) with an anatomical meaning
(`Chest | Back | …`), and this is a functional one (finding 10):

| joint | `ResistingGroup` |
|---|---|
| hip | hip extensors |
| knee | knee extensors |
| ankle | plantar flexors |
| L5/S1 | spinal erectors |

No per-muscle split exists anywhere, and the type system makes it hard to add
by accident: `ResistingGroup` is a closed enum with four variants and no
`Muscle` type exists. RFC-002 §7 explains why, and §7.6 what it would take to
change that honestly. Neither is in scope.

### 2.8 Every result states its assumptions (AC12) and refuses honestly (AC13)

```rust
pub struct Mechanics {
    /// hip, knee, ankle, lumbar — fixed order. The bench, which would have
    /// needed a per-lift shape, is out of R-0045 (§5.3).
    pub joints: [JointMoment; 4],
    pub load: LoadBasis,
    pub assumptions: Vec<Assumption>,       // varies per result (finding 21)
    pub uncertainty: Uncertainty,
}

pub struct JointMoment {
    pub joint: Joint,
    pub resisting: ResistingGroup,
    pub moment: NewtonMetres,                // resisting extension, signed
    pub uncertainty: NewtonMetres,
}

pub enum LoadBasis { Bar { kg: f64 }, BodyweightOnly }

pub enum Assumption {
    SagittalPlane,
    QuasiStatic,
    RigidSegments,
    SinglePosture { at: Position },          // v1 always
    SolvedBaseline,                          // v1 always: not a measured posture
    RigidTrunkLumbar,                        // present iff the lumbar row is
    TableDerivedMasses,                      // ALWAYS — masses have no measurement path
    TableDerivedLengths,                     // iff any length came from the table (v1: always)
    KneesTrackFeet,                          // iff stance width ≠ baseline
    BodyFatIgnored,
}
```

**`Uncertainty` is a design, not a name** (finding 13). Each joint's
uncertainty is the half-width from propagating three stated spreads through the
same `moment_about` — segment-length spread (±5%, convention), the L5/S1
placement spread (lumbar row only, ±3 cm, convention), and the head/arm
placement conventions (±2 cm) — by evaluating the moment at the corner cases
and taking the largest deviation. No distribution is assumed; the number is a
bound from stated inputs, and each input is a labelled convention until cited.
A delta whose magnitude is inside its uncertainty is `Borderline` (R-0044
AC10c's rule).

`LoadBasis::BodyweightOnly` is what AC3 requires when no load is supplied: the
analysis runs, reports bodyweight mechanics, and says that is what it did. It
never assumes a bar. In v1 the load is an **explicit input** (a `LoadKg`); the
"logged set the clip is attached to" wording belongs to the measured source
(§5.1).

AC13 for the parametric source has nothing upstream to inherit; its refusals
*are* §2.6.4. For the measured source (later), any R-0044 `Refusal` propagates
unchanged.

**AC14** is enforced at the type level: no `risk`, `danger`, `safe` or
`injury` field anywhere in `Mechanics`, and the prose fields are cues, never
verdicts. For slice C, a coach's own cue text is user speech outside the
model's output (finding 24) — stated now so C does not invent a word filter.

### 2.9 Persistence and export (AC10, AC11, OQ-6) — slice C, deferred

A saved variant is per-user, author-only, `404` never `403`, via the single
`load_owned` pattern from SPEC-0041. Storage reuses `authored_programs` (AC10)
— but the shape a variant attaches to needs R-0041's model read against this
one, and OQ-6's answer depends on R-0040 roles, which do not exist. Slice C is
specified when both are clearer. Nothing in slices A/B writes to a database.
**R-0045 stays open until C ships**, and the roadmap must say so.

### 2.10 Slicing

| slice | contents | depends on |
|---|---|---|
| **A** | `anthro` (table + sum claims + verification record); `Posture`, `Load`, `Point`/`Metres`; `above()` + `moment_about` with the sign rule; the lumbar row; the **baseline** squat solve with the system-COM constraint; `Rejection` variants reachable without knobs (`DoesNotBalance`, `AnatomicallyImplausible`) | nothing |
| **B** | the three knobs, `OutOfRange`, the which-unknown rule, deltas, `Borderline`, the pull | A; §5.1 amendment |

| **C** | `api::biomech`, persistence, coach export | B, R-0041 read, R-0040 |

Slice A's standalone value is the corrected, tested, cited model and the reel
correction — real for the funnel, nil for the product until C. It changes the
published numbers and should ship with the reel correction.

## 3. Code outline

```
backend/crates/core/src/biomech/
  mod.rs        Posture, Point, Metres, NewtonMetres, Load, Mechanics,
                JointMoment, LoadBasis, Assumption, Uncertainty
  anthro.rs     WINTER table + verification record, Segment, Body, scale(),
                MeasuredLengths, the three sum claims
  chain.rs      above(joint), moment_about() — the ONLY place g is multiplied;
                Joint::extension_sign() — the ONLY place a sign is applied
  lumbar.rs     l5s1_point() (convention + error), the lumbar load side
  squat.rs      baseline(body, load) -> Result<Posture, Rejection>   [A]
  knobs.rs      Knobs, apply(), which_unknown()                        [B]
  delta.rs      Delta, Borderline, perturb()                           [B]
```

Representative — `chain.rs` is the whole model's spine:

```rust
/// Σ over everything on the load side of `j`: weight × horizontal lever.
/// Coordinate-signed; `moment_about` orients it per joint.
fn raw_sum(j: Joint, body: &Body, p: &Posture) -> f64 {
    let xj = p.at(j).x.0;
    let mut m = 0.0;
    for seg in above(j) {
        m += body.mass_kg(seg) * G * (seg.com(body, p).x.0 - xj);
    }
    if let Some(load) = &p.load {
        m += load.kg * G * (load.at.x.0 - xj);
    }
    m
}

/// N·m resisting extension: positive means the extensors are loaded.
pub fn moment_about(j: Joint, body: &Body, p: &Posture) -> NewtonMetres {
    NewtonMetres(j.extension_sign() * raw_sum(j, body, p))
}
```

`above(j)` is four fixed lists (§2.4.2), not a graph walk. `Point`/`Metres`/
`NewtonMetres` are newtypes (finding 22) — this crate's own "radians, not
degrees" lesson, and finding 3, are both unit-and-sign errors a bare `f64`
invites.

## 4. Non-goals

- Per-muscle force or activation (AC5, RFC-002 §7.3). No `Muscle` type exists.
- Foot rotation, knee valgus, grip width as abduction, the frontal-plane hip
  moment of an abducted femur — anything outside the sagittal plane.
- Full inverse dynamics with acceleration terms (OQ-2: quasi-static).
- Any φ axis, peak, or "per rep" in v1 (single posture; §2.1).
- The measured (keypoint) source and the scale bridge (§2.1.1). Seam only.
- The front squat (a different lift, not a knob). The bench (struck, §5.3).
- Vendoring `mecanica` or any reel model (§2.2).
- Coach→client sharing (needs R-0040). Slice C stores the author's own only.
- Any injury, safety or risk output (AC14). Not a field, not a word.
- Body-fat-dependent segment parameters (§2.3): no source, so no adjustment.

## 5. Owner decisions — all taken 2026-09-12

### 5.1 R-0045 amendment request (architect finding 1)

The requirement was accepted on 2026-09-11 with wording for a **measured**
source. The owner's 2026-09-06 direction — recorded in `ROADMAP.md` — is
parametric first. RFC-002 §9 says the dependency change "is the fitAI owner's
call". The draft asked to amend AC7 only; the parametric source touches
**seven** lines. Proposed wording, mirroring R-0044's 2026-08-25 amendment:

| line | current | proposed |
|---|---|---|
| **Depends on** | R-0044 (the keypoint series this consumes) | R-0044 *for the measured source only*; v1's parametric source depends on R-0003 (profile) and R-0004 (load) alone |
| **AC1** | "linear algebra over the R-0044 series" | "…over a `Posture` in metres, produced by the v1 parametric solver or, later, by the R-0044 series" |
| **AC3** | "from the logged set the clip is attached to" | "from an explicit `LoadKg` in v1; from the attached logged set once the measured source exists" |
| **AC4** | "through the movement, with the peak and the position … per rep" | "at the solved posture in v1 (single position, stated on the result); through the movement and per rep once the measured source exists" |
| **AC6** | "starts from a real analyzed set — that person's limb lengths, posture, and load" | "…that person's limb lengths and load, at a **stated baseline posture** in v1; their measured posture once the measured source exists" |
| **AC7** | "stance width, foot rotation, bar position … torso angle" | "stance width, bar position, torso angle. Foot rotation is frontal/transverse-plane and is deferred to a 3D model — a sagittal number for it would be fabricated" |
| **AC13** | "If the underlying clip was refused…" | "Parametric source: a typed `Rejection` for any posture that does not balance or is anatomically implausible. Measured source: inherits R-0044's refusal unchanged" |
| **AC15** | "over the keypoint series plus profile and load" | "over a `Posture` plus profile and load" |

**Decided: accepted as proposed.** Applied to `requirements/0045-*.md` with
a changelog entry and decision-log rows.

### 5.2 Home of the physics (architect finding 5)

**Recommended and taken in §2.2:** `fitai_core::biomech`, AC15 as written, no
vendoring, physics-lab keeps its own teaching crate.

**Alternative:** a zero-dependency workspace crate with *only* R-0045's model,
consumed by physics-lab via pinned git dependency. One implementation, but:
needs an AC15 amendment, a cross-repo build dependency on a private repo, and
has no `Movement` implementor until slice B. **Decided: `fitai_core::biomech` only.** Revisit if slice B introduces a
φ axis worth sharing.

### 5.3 Is the bench in R-0045? (architect finding 14)

AC4 names hip/knee/ankle/lumbar; AC7 mentions "grip width for bench". A bench
has elbow and shoulder joints, a two-arm load split, and a **frontal-plane**
grip-width effect. Neither `Posture` nor a four-joint result can express it,
and grip width cannot be modelled sagittally any more than foot rotation can.
**Decided: struck.** AC7's grip-width clause removed; the bench gets its own
requirement when a frontal-plane model exists. `Mechanics.joints` is therefore
`[JointMoment; 4]` — hip, knee, ankle, lumbar, fixed order.

### 5.4 The published correction

§2.4.3's 274 → ≈366 N·m is a real error in a published reel — the third the
model has surfaced. Whether and how to correct it publicly is yours. The spec's
only stake is that slice A **derives** the number so a correction is computed
rather than asserted.

### Resolved here (owner may veto)

- **OQ-1** → Winter Table 4.1 / Fig 4.1, with a recorded verification and
  self-checking sum claims; body fat ignored, stated. §2.3.
- **OQ-2** → quasi-static; error stated on every result. §2.4.
- **OQ-3** → single L5/S1 joint as a stated convention with error term and
  candidate citations; rigid trunk, cannot see rounding, says so. §2.5.
- **OQ-4** → kinematic solve with the system-COM constraint and a stated
  which-unknown rule; never interpolation. §2.6.3.
- **OQ-5** → ranges in §2.6.2, each labelled a **stated convention pending
  citation**, with the candidate source named. The draft called two of them
  physics; they were not.
- **OQ-6** → deferred with slice C; depends on R-0040.

## 6. Acceptance criteria

Against R-0045 **as amended by §5.1**; the qa agent owns the tests.

- [ ] **AC1** — `chain::moment_about` is the only gravity term; no engine, no
      stepping; `biomech` compiles with no I/O crates.
- [ ] **AC2** — every constant in `anthro::WINTER` carries its table/figure
      reference; the verification record in §2.3 is filled in before the
      constants are committed; the three sum claims pass.
- [ ] **AC3** — `LoadBasis::BodyweightOnly` produces a result whose `load`
      says so and whose moments contain no bar term; a bar with no position is
      unrepresentable.
- [ ] **AC4** (amended) — `joints` has hip, knee, ankle, lumbar; every v1
      result carries `SinglePosture`.
- [ ] **AC5** — `ResistingGroup` has four variants; no `Muscle` type exists.
- [ ] **AC6** (amended) — every v1 result carries `SolvedBaseline`; deltas are
      computed from the lifter's own baseline.
- [ ] **AC7** (amended) — three knobs with ranges; `OutOfRange` on every bound.
- [ ] **AC8** — `Delta`'s headline field is the percentage change.
- [ ] **AC9** — every `Rejection` variant is reached by a test through the
      public solve.
- [ ] **AC12** — `assumptions` is non-empty; `TableDerivedMasses` is
      unconditional; `RigidTrunkLumbar` iff the lumbar row; `KneesTrackFeet`
      iff stance ≠ baseline.
- [ ] **AC13** (amended) — parametric rejections are typed; measured deferred.
- [ ] **AC14** — no field or string in the output namespace matches
      `/risk|danger|safe|injur/i`.
- [ ] **AC15** — `core::biomech` is the single home; `Cargo.toml` unchanged.
- [ ] **AC16** — §7 in full.

## 7. Test plan

**The table (AC2):**
- `B0` the three sum claims of §2.3 hold; every `WINTER` constant has a
  non-empty citation string.

**Hand-computed statics (AC16):**
- `B1` trunk vertical over the hip, no bar → hip moment ≈ 0. Sign bench.
- `B2` trunk at 30° from vertical, known load side, no bar →
  `M_hip = Σ m·g·lever` exactly, by hand, **positive** (extensors loaded).
- `B2k` the same posture → **knee** moment positive too, despite the raw sum
  being negative there. This is the test finding 3 exists for.
- `B3` the MECÁNICA 01 setup: reel lengths as `MeasuredLengths` overrides
  (femur 0.40, shank 0.43, trunk 0.53), 80 kg, 100 kg bar high-bar → the bar
  component reproduces **274 N·m** and the total is **≈ 366**. The reel's
  number is recovered as a component, so the correction is derived.
- `B4` knee moment invariant to femur length at fixed shank geometry (RFC-002
  C2, now with segment weight — must still hold: bar and load side move with
  the hip, not the knee).

**Balance (AC3, finding 4):**
- `B5` `BodyweightOnly` solves: a posture exists, its system COM is over the
  mid-foot within 1 mm, and `load` says bodyweight-only.
- `B5b` with a bar of mass → ∞ (numerically: 10⁴ kg) the solved θ converges to
  the bar-over-mid-foot solution — the limit claim of §2.6.3.

**Symmetry (AC16):**
- `B6` a mirrored posture gives mirrored moments (sign rule applied
  consistently).

**Documented shifts (AC16, slice B):**
- `B7` low-bar vs high-bar: hip moment ↑, knee moment ↓ — as *changes in the
  extension-resisting moment*, not raw signs.
- `B8` wider stance: femur projection shortens → hip moment ↓; result carries
  `KneesTrackFeet`.

**Rejection (AC9):**
- `B9`–`B11` one per variant, each reached through the public solve, with the
  which-unknown rule determining which fires.

**Stability (AC16, finding 13):**
- `B12` every table-derived length ±5%: every delta keeps its sign or is
  `Borderline`.

**Honesty:**
- `B13` every result carries `SagittalPlane`, `QuasiStatic`, `SinglePosture`,
  `SolvedBaseline`, `TableDerivedMasses`; carries `TableDerivedLengths` iff any
  length was unmeasured (v1: always).
- `B14` lumbar row present ⇒ `RigidTrunkLumbar` present.
- `B15` `LoadBasis::Bar` with the load side ⇒ the arm COMs are at the
  shoulder→bar midpoints; `BodyweightOnly` ⇒ arms hang.

## 8. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-09-12 | Parametric source first; keypoints later | The posture needs no camera (RFC-002 §9); owner direction 2026-09-06. **Requires the §5.1 amendment.** |
| 2026-09-12 | v1 is a single solved posture | No validated descent kinematic exists; a φ axis would be fabricated (finding 2). |
| 2026-09-12 | `core::biomech` is the single home; no vendoring | Reel models have no requirement; `Lift` does not collide; `G` is one constant (finding 5). |
| 2026-09-12 | Output is "moment resisting extension", sign applied in one place | Raw coordinate sign flips at the knee on a zig-zag chain (finding 3). |
| 2026-09-12 | Balance = system COM over mid-foot | Bar-over-mid-foot does not exist without a bar (finding 4); recovered as a limit. |
| 2026-09-12 | Torso-angle knob solves the shank; others solve θ | Makes rejection ordering implementation-independent (finding 12). |
| 2026-09-12 | Trunk split, not the HAT lump; arms on the bar | The lumbar row needs the split; the lump under-counts arms on a bar (finding 7, 18). |
| 2026-09-12 | Quasi-static | Exact at the sticking point; error stated. |
| 2026-09-12 | L5/S1 as a stated convention with candidate citations | The draft's "Winter's convention" was not in the table (finding 7). |
| 2026-09-12 | Body fat ignored in v1 | The cited table does not vary by it; inventing an adjustment violates AC2. |
| 2026-09-12 | Kinematic solve, never interpolation | Interpolated postures are ones the lifter never held. |
| 2026-09-12 | Foot rotation and grip width: amend, don't fabricate | Frontal-plane; the SPEC-0044 precedent. |
| 2026-09-12 | Every range is a labelled convention until cited | The draft called two of them physics; one was false and one unreachable (finding 6). |

## 9. Changelog

- _2026-09-12 — created (Draft)._
- _2026-09-12 — reworked in full after architect review (§10). Every required
  finding applied; three owner decisions isolated in §5._
- _2026-09-12 — **Accepted.** All three §5 decisions taken as recommended;
  requirement amended; bench struck; `joints` fixed at four._

## 10. Architect review — ACCEPT WITH CHANGES (2026-09-12), all required changes applied

Sixteen required findings, nine recommended. Every required finding was
verified against the code or the arithmetic and applied; the recommended ones
were applied where they cost nothing (17, 18, 19, 20, 21, 22, 23, 24) and
deferred where they need slice B (25 — lifting `technique::Lift` to a neutral
module is a slice-B concern, since slice A is squat-only).

| # | finding | applied in |
|---|---|---|
| 1 | amendment under-scoped: seven lines, not AC7 alone | §5.1 |
| 2 | AC4 quietly claimed on a single posture | §2.1, `SinglePosture` |
| 3 | sign convention wrong at the knee | §2.4.1, `extension_sign`, B2k |
| 4 | no balance constraint for bodyweight-only | §2.6.3, B5/B5b |
| 5 | vendoring ships requirement-less reel models; collision argument false | §2.2, §5.2 |
| 6 | `StanceExceedsFemur` unreachable; range sources false | §2.6.2, §2.6.4 |
| 7 | L5/S1 uncited; trunk split and head/arm placement missing | §2.3, §2.4.2, §2.5 |
| 8 | `TableDerivedMasses` wrong condition; R-0034 does not exist | §2.1.1, §2.8 |
| 9 | a bar with no position was representable | `Load { kg, at }` |
| 10 | `MuscleGroup` collides with `fitai_core::MuscleGroup` | `ResistingGroup` |
| 11 | baseline conventions unsourced; A/B line contradictory | §2.6.1, §2.10 |
| 12 | "one unknown" unnamed | §2.6.3 |
| 13 | `Uncertainty` undesigned; wrong noise model | §2.8, §2.6.5, B12 |
| 14 | the bench cannot fit the types | §5.3, `Vec<JointMoment>` |
| 15 | stance assumes knee-over-foot; frontal moment unstated; hip breadth missing | §2.6.2, §2.3 |
| 16 | two mis-citations (Winter App. A; B3's lengths) | §2.6.4, §2.4.3, B3 |

**Two things the reviewer said that are worth keeping in front of anyone
implementing this:** (a) B3's shape — recover the published number as a
*component* and derive the correction — is the pattern for every claim that
contradicts a reel; (b) with findings 3, 4, 6 and 12 applied, the solver's
rejection ordering is fully determined by the text, which is the test of a
finished design section. Before them, three different implementations would
all have "matched the spec".
