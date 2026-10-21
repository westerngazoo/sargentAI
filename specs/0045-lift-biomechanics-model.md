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
`Assumption::SinglePosture { at: Position::Parallel }`, and AC4's "through the
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
Σ mᵢ·kᵢ / Σ mᵢ   over the §2.4.2 split                  = HAT's own COM, 0.626
```

The fourth is the one that constrains **placement** rather than mass, and it
was missing. The first three catch a mistyped mass; none of them catches a
segment put in the wrong place, and the split and the lump are the table's two
descriptions of the same 0.678 M on the same line. It is what fixes L5/S1
(§2.5) — and its absence had left every hip moment 16% low.

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
  at the midpoint of shoulder → load point. When there is no load, arms hang
  straight down: COM directly below the shoulder at the **whole arm's** centre
  of mass — the mass-weighted mean of upper arm, forearm and hand at their own
  Table 4.1 centres, `0.171 H` (0.300 m at 1.75 m). Convention; error carried.
  *(The draft used `0.436 × upper-arm length`, the upper arm's centre alone —
  half the depth. Corrected 2026-09-27. No slice-A moment moved: a vertically
  hanging arm has the shoulder's `x` at any depth.)*

Winter's lumped HAT (COM 0.626) assumes **arms at the sides**. Hands on the
bar move the arm mass to the shoulder → bar midpoint, which on the trunk line
is 0.975 — *below* the shoulder's 1.000, because the bar is at 0.95. So the
split's hip term is slightly **smaller** than the lump's, not larger. (The
draft asserted the reverse and used it to justify the split. It was wrong on
both counts: the reason to use the split is that the lumbar row needs the
trunk divided at L5/S1, and the lump cannot be divided.)

**The split and the lump must agree, and that is a testable claim** — same
mass, same line, the table's own two descriptions of one thing. Solving it for
the one placement the table does not give is what fixes L5/S1 (§2.5), and the
check is the fourth sum claim in §2.3.

#### 2.4.3 What this does to the reel's numbers

MECÁNICA 01 publishes **274 N·m** at the hip for a 100 kg bar and a 0.40 m
femur. That is the **bar-only** component. With an 80 kg lifter and the reel's
own segment lengths as overrides (finding 16 — table proportions at 1.75 m
give a 0.429 m femur, so height must scale only the *unmeasured* segments) and
the bar at 0.95 of the trunk line, the trunk solves to **33.64°** and the
honest hip moment is **370.6 N·m**. The published figure understates it by
**35%**.

Two cross-checks, both in the suite. The bar-only term comes back as
**273.62 N·m**, which is the reel's own number recovered as a component — so
the correction is *derived*, not asserted. And the same posture through the
lumped HAT row gives **371.4 N·m**: the split and the lump agree to 0.2%, which
is the form the §2.3 identity takes on a real posture.

*(Both numbers moved during implementation. The draft said 31.75°, which was
the bar-at-1.00 geometry, and ≈366 from the lump; the split then landed at 357
and the ±5% band in B3 hid the disagreement. The cause was `L5S1_UP_THE_TRUNK`
— see §2.5. B3's band is now ±1 N·m.)*

The reel is a bar-only model and says so nowhere. Whether to publish the
correction is the owner's (§5.4).

### 2.5 The lumbar row (OQ-3) — and what it cannot see

**Placement.** A single joint at **L5/S1**, on the trunk line above the hip
joint centre, at **0.2674 of hip → shoulder** — *derived from the table, not
chosen*.

The draft attributed a `0.10 × trunk` offset to "Winter's convention"; it is
not in Table 4.1 (finding 7), and taken as a bare convention it was simply
wrong. Table 4.1 describes this mass twice — once split into pelvis, thorax &
abdomen, head and arms, and once lumped as **HAT**, same 0.678 M on the same
trochanter → glenohumeral line, with its own centre of mass at 0.626. The
split must reproduce the lump. Every placement in the split comes from the
table except L5/S1, so the identity determines it:

```text
Σ mᵢ·kᵢ / Σ mᵢ  =  0.626        ⟹   L5/S1 at 0.2674 of the trunk line
```

At `0.10` the split's centre of mass sat at **0.539**, so every hip moment the
model produced was 16% low, with nothing in the suite to say so — and L5/S1
landed 5 cm above the greater trochanter, which is anatomically wrong on its
face. At the derived value it lands **13.5 cm** above it for a 1.75 m lifter,
which is where the lumbosacral junction belongs.

**What the derivation assumes, stated.** The identity is one equation and the
split has three conventions the table does not supply — L5/S1, the head COM
beyond the shoulder, and the arms' placement. Attributing the whole residual
to L5/S1 is a choice: it is the only one of the three with no Winter value of
any kind, the only one the draft had already flagged as pending citation, and
the only one whose draft value was independently implausible. The candidate
sources — de Leva (1996), *Adjustments to Zatsiorsky-Seluyanov's segment
inertia parameters*, and Chaffin, Andersson & Martin, *Occupational
Biomechanics*, 2D static model — are still worth opening: a cited L5/S1 that
disagrees with 0.2674 would mean the residual belongs partly elsewhere, and the
sum claim would fail and say so. That is the difference between a derived
constant and a convention.

The error term stands: the true joint varies by several centimetres between
people. It is carried as the ±3 cm `l5s1_placement` spread on **every** row,
not the lumbar row alone (§2.8).

**Load side.** Everything above L5/S1: thorax & abdomen (0.355 M), head (0.081
M), arms (0.100 M), and the load — **0.536 M**, not the 0.678 HAT lump
(finding 7). The L4-L5 / L5-S1 boundary mismatch is absorbed into the stated
error.

**The lumbar row is where the L5/S1 spread costs most, and it is the bar that
makes it so.** Moving the joint centre moves the row's own lever against its
whole load side — including the bar, the largest mass there. For the fixture
lifter under 100 kg the lumbar half-width is 20.8 N·m against 5.8 at the other
three joints. A placement this model derives rather than cites is exactly the
one a heavy bar amplifies, which is the argument for opening de Leva.

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
| shank angle from vertical | **the lifter's weight-bearing ankle dorsiflexion** (§2.6.1.1) | measured — a **required input** to the slice-A solve; no default is committed |
| bar attachment on the trunk line | `0.95 × trunk` (high-bar) | convention; the low-bar value is a knob (§2.6.2) |
| hands | on the bar | convention |

The reel's `22°` shank and `(−0.04, 0.09)` ankle were literals from
`muneco.py` with no source; they are replaced, and where a literal survives it
is labelled a convention rather than a fact.

#### 2.6.1.1 The shank angle is the ankle (owner decision, 2026-09-15)

With the foot flat on the floor, **the shank's angle from vertical is the
ankle's dorsiflexion angle** — the same angle from the same neutral. So the
baseline shank angle is not a convention to be picked: it is a property of the
lifter, and one that is routinely measured (the weight-bearing lunge /
knee-to-wall test).

This replaces the draft's unstated "the unknown", which left the baseline
undefined — and the omission was not cosmetic. Solved against the §2.6.3
system-COM constraint for a 1.75 m / 80 kg lifter at table proportions, at
parallel, **bodyweight only with the arms hanging** (the §2.4.2 convention —
the conditions matter, see the bar row below):

| dorsiflexion | trunk lean | hip flexion | |
|---|---|---|---|
| ≤ 28.7° | — | — | parallel is not reachable: `DepthNotReachable` |
| 30° | 74.5° | 164.5° | fold — a good morning out of the hole |
| 35° | 55.7° | 145.7° | deep lean |
| 40° | 44.1° | 134.1° | normal squat |
| 45° | 35.1° | 125.1° | normal squat |
| 50° | 27.7° | 117.7° | upright |
| 55° | 21.5° | 111.5° | upright |

At parallel the femur is horizontal, so **hip flexion is exactly `90° + lean`**
— the column is derived, not separately claimed.

**The model reproduces a known coaching fact**: restricted ankles force forward
lean, and that is *why* a stiff-ankled lifter squats like a good morning. That
is an output worth showing, not an artefact.

**And a second one, which the table above would hide.** Put a 100 kg bar on the
same lifter and the threshold moves from 28.7° of dorsiflexion to **13.4°**,
with 40° of ankle giving a 31.6° lean rather than 44.1°. The bar sits at 0.95
of the trunk line — a longer lever than the body's own load side, whose mass is
centred at 0.626 of it — so a smaller lean carries the system over the
mid-foot. The load is a counterweight: **a loaded squat reaches depth at less
ankle, and more upright, than a bodyweight one.** Any table of this relation
must therefore state its load, which this one does.

*(This table has been wrong twice. The first version stated no load. The second
stated one but was computed with `L5S1_UP_THE_TRUNK = 0.10`, which put the
body's load side at 0.539 of the trunk line instead of 0.626 and moved the
whole relation ~5° — see §2.5. These are the values B5c checks, and the §2.3
identity is now what stops a third.)*

**Source (pending, AC2).** A default for an **unmeasured** lifter would need a
cited normative value for weight-bearing dorsiflexion — the candidate is the
weight-bearing lunge test literature (Bennell et al. 1998 for the protocol,
plus a normative-range paper). Until cited and recorded, no default is
committed, exactly as §2.3's table is not — and unlike the table there is no
sum claim that could self-check an invented one.

**So in slice A dorsiflexion is a required input to the solve, not an
`Option` with a fallback.** `core::biomech` never supplies a number nobody
sourced; a caller that has no measurement must decide what to tell the user,
and it cannot do that by accident. When the profile path lands (slice C) it
joins the per-person measurements with a cited fallback, and the result carries
`Assumption::TableDerivedLengths` when the fallback was used. It becomes a knob
in slice B — heel elevation is precisely a dorsiflexion knob, which is why
lifting shoes exist.

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

**It is linear in `sin δ` too**, which is what makes the rejection rule below
closed-form rather than a search. Writing `u = sin δ` for the dorsiflexion and
`Σ m·x` for the first moment about the mid-foot line:

```text
Σ m·x = A(u) + B·sin θ ,     A(u) = P + Q·u ,   Q > 0
```

because every load-side `x` is `hip_x + k·trunk·sin θ` with a fixed `k`, and
`hip_x = ankle_x + shank·u − femur` is itself affine in `u`. `B` collects the
`k`-weighted masses and does not depend on `u`. Balance is `A(u) + B·sin θ = 0`,
so

```text
sin θ = −(P + Q·u) / B
```

and requiring `sin θ ∈ [0, 1]` — the trunk between vertical and horizontal,
the stated domain — gives an admissible window in the lifter's own ankle:

```text
u_min = (−B − P)/Q      the least dorsiflexion that reaches the depth
u_max = −P/Q            beyond which the trunk would have to lean backwards
```

Both ends are algebra over the lifter's own lengths and masses. **No constant
that anyone has to cite appears in either.**

**Which unknown** (finding 12, resolved 2026-09-15): the **shank angle is an
input** (§2.6.1.1) — a required one, supplied by the caller — so the baseline's
unknown is always the **trunk angle θ**. The draft's §2.6.1 and §2.6.3
contradicted each other on this; the ankle decision settles it.

In slice B a torso-angle knob overrides θ, and the **shank angle** becomes the
unknown — but only within the lifter's dorsiflexion: a solve needing more ankle
than they have is `DepthNotReachable`, not a posture. Bar-position and stance
knobs leave θ as the unknown. This rule determines which rejection fires and is
what makes the solver's behaviour fully specified rather than
implementation-defined.

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
        /// A solved joint angle is outside the range observed in lifting.
    /// **Unreachable in slice A**: no limit constant is committed until one is
    /// cited (§2.6.4). Present so the closed set does not change shape later.
    AnatomicallyImplausible { joint: Joint, angle_deg: f64 },
    /// The requested depth needs more range than the lifter has — at the
    /// baseline, more ankle dorsiflexion than they possess. Names the limiting
    /// joint, because "your ankle is what stops you" is the useful sentence,
    /// not a number for a posture they cannot hold.
    DepthNotReachable { limiting: Joint, needed_deg: f64, available_deg: f64 },
}
```

The draft's `StanceExceedsFemur` is folded into `OutOfRange` — with the range
at 2.2× and the solve domain at 3.6×, it could never fire (finding 6).
**Joint limits come from observed lifting postures, not goniometric norms**
(owner decision, 2026-09-15). The draft's placeholder — hip flexion > 130° —
**rejected the model's own baseline**: a realistic parallel squat reaches
125–146° of hip flexion (§2.6.1.1), so the check fired on the primary output.
A passive-ROM norm is the wrong instrument, because a loaded deep squat exceeds
passive norms by design.

The limits must be cited from measured squat kinematics. Candidates to verify
before any constant is committed: Escamilla et al. (2001), *A three-dimensional
biomechanical analysis of the squat during varying stance widths*, and
Escamilla (2001), *Knee biomechanics of the dynamic squat exercise*, both Med
Sci Sports Exerc. **Until one is recorded, no limit constant is committed and
`AnatomicallyImplausible` is unreachable** — slice A ships with
`DoesNotBalance` and `DepthNotReachable` only. A threshold nobody has sourced
is not a threshold.

**A restricted lifter's baseline may legitimately fail, and that is a finding.**
The correct output for them is not a number for an impossible posture but
`DepthNotReachable { limiting: Joint::Ankle }` — *your ankle is what stops
you*, which is the most useful sentence the model can produce for that person.
Depth becomes a knob in slice B so they can be analysed at a depth they
actually reach.

**Which of the two fires is decided by §2.6.3's window, not by a threshold.**
Given the lifter's `u = sin δ`:

| condition | outcome |
|---|---|
| `u_min ≤ u ≤ u_max` | a posture: `sin θ = −(P + Q·u)/B` |
| `u < u_min` and `u_min ≤ 1` | `DepthNotReachable { limiting: Ankle, needed_deg: asin(u_min), available_deg: δ }` |
| `u < u_min` and `u_min > 1` | `DoesNotBalance { unknown: TrunkAngle }` — no ankle in `[0°, 90°]` would do |
| `u > u_max` | `DoesNotBalance { unknown: TrunkAngle }` — the trunk would have to lean backwards |

This is the whole rule, and it is worth seeing what it costs: **nothing that
needs a citation.** `needed_deg` is not compared against a normative range —
it is the lifter's own algebra, reported back to them. The distinction between
the two rejections is exactly *"would more ankle fix this?"*, which is both the
honest question and the one the lifter can act on. And `90°` as the search
bound is the geometric domain of a shank angle from vertical, not a claim about
any ankle.

Worked, for the §2.6.1.1 lifter: bodyweight-only, `u_min = 0.4806` → **28.7°**,
so 25° of dorsiflexion is `DepthNotReachable` with `needed_deg ≈ 28.7`. Under a
100 kg bar the same lifter's `u_min` falls to 0.2319 → **13.4°**, and 25° is a
posture. Same person, same depth, different load: the rejection is a property
of the *lift*, not a label on the lifter.

All four rows are reachable and all four are claimed: `B9` and the
`u_min > 1` branch; `B5d` and `B11`; `u > u_max` via a long shank and a short
femur; and the undetermined case — a body with no mass on the trunk line and no
bar, for which the equation is `0 = 0` and any lean would be invented.

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

#### 2.6.6 The range: from standing toward the bottom (slice B1; owner decision 2026-10-17)

Slice A analyses one posture. The owner asked for the squat over its whole
range, as the base for a reel-style technique video and, later, a
frame-by-frame comparison with the lifter's own video.

**The problem is one equation short.** Between standing and the bottom three
angles move (shank, femur, trunk) and balance supplies one equation. Slice A
closed it at the bottom by fixing the femur (parallel) and the shank (the
lifter's dorsiflexion). Everywhere else something has to say how the shank and
the femur move together, and nothing measured does yet; that is the measured
source (§2.1.1). So the range declares **one convention for the legs** and
solves everything physics can. It is the first squat descent axis in either
repository: RFC-002's progress scalar comes from single-joint reels, and the
reels' own squat explicitly declined one (`physics-lab/mecanica/src/sentadilla.rs`:
"no published model validates a descent kinematic").

**The legs, by convention.** Upright (`s = 0`) and the bottom (`s = 1`) are
both solved (below). Between them the shank angle `α` (from vertical, knee
ahead positive) and the femur angle `φ` (from vertical, hip behind the knee
positive) are interpolated **in angle, never in sine**, in exactly this form:

```text
α(s) = (1 − s)·λ + s·δ          λ: the solved upright lean     δ: the lifter's dorsiflexion
φ(s) = −(1 − s)·λ + s·90°       −λ: the straight leg at upright   90°: the femur horizontal
knee flexion(s) = s·(δ + 90°)   an identity, so s has a geometric meaning independent of λ and the load
```

The form is `(1 − s)·a + s·b`, not `a + s·(b − a)`, which in f64 misses `b` at
`s = 1`. The frame at `s = 1` is never interpolated: it **is** the output of
`squat::baseline`.

**The convention decides the shape of every curve read across the range.**
Interpolating the sines instead of the angles is just as "linear", and it
moves the fixture's hip maximum from `s = 0.8` (373.6 N·m) to the bottom
(333.9 N·m, rising the whole way). So **where a moment peaks, whether the
trunk lean is monotone, and how far a lifter reaches are statements about this
convention, not about the lifter.** The lifter's own curve needs their own
kinematics. That is the measured source, and it is what the frame-by-frame
video comparison is for.

**The trunk is solved, never interpolated.** At every `s` the trunk angle
comes from the §2.6.3 balance equation, still linear in `sin θ` and
closed-form, in the domain `sin θ ∈ [0, 1]`. Every frame is a posture that
balances over the mid-foot. The reels' squat solves its torso at one posture;
no reel solves the trunk across a range.

**Upright is solved too.** The straight body, knee and hip extended, leans
forward from the ankle until the system centre of mass is over the mid-foot.
This is the ankle strategy of quiet standing (Winter 1995, candidate
citation). `Σ m·x` is exactly affine in the sine of that lean, with the arms
hanging or on the bar, so the lean `λ` is **measured off the model by two
probes**, the §3 rule for `P`/`Q`/`B`, and never re-derived from the `k`
fractions. It is **4.04°** for the fixture bodyweight-only and **3.35°** under
100 kg. "Everything vertical" is not a starting posture: with the ankle behind
the mid-foot it does not balance.

Worked for the fixture (1.75 m, 80 kg, 40° of dorsiflexion, 100 kg high-bar)
at six frames, moments in N·m resisting extension, **under this convention**:

| `s` | knee flex | hip flex | trunk | hip | knee | ankle | L5/S1 |
|---|---|---|---|---|---|---|---|
| 0.0 | 0° | 0.0° | 3.4° | 37.2 | −77.4 | 120.5 | 25.4 |
| 0.2 | 26° | 30.4° | 15.1° | 165.4 | 16.0 | 120.5 | 112.8 |
| 0.4 | 52° | 59.8° | 25.8° | 276.7 | 107.3 | 120.5 | 188.7 |
| 0.6 | 78° | 86.2° | 33.5° | 351.3 | 194.8 | 120.5 | 239.6 |
| 0.8 | 104° | 107.3° | 35.9° | 373.6 | 277.2 | 120.5 | 254.8 |
| 1.0 | 130° | 121.6° | 31.6° | 333.9 | 353.0 | 120.5 | 227.7 |

- **The last frame is slice A's baseline, exactly**, by construction.
- **Under this convention** the hip and L5/S1 rows peak near `s = 0.8`, and
  the trunk lean is not monotone (35.9° at `s = 0.8`, then 31.6°). Under sine
  interpolation neither is true. Neither is reported as a fact about the
  lifter.
- **The ankle moment is the same in every frame, and that tells nothing about
  the posture.** Under the balance constraint the moment about the ankle is
  the weight above it times the fixed ankle-to-mid-foot distance:
  177.7 kg × g × 0.0692 m = 120.5 N·m. The distance is the §2.6.1 mid-foot
  convention (0.26 × 0.152 H), so this row is set by that convention at any
  `δ`, in slice A as here. It is a theorem and a test (exact to within
  1e-9 N·m in f64). A reel must not present it as a finding about the plantar
  flexors.

**Where the range ends.** The frames are the **largest balanced prefix** of
the convention. Its end `s*` is the first `s` where `sin θ(s)` leaves `[0, 1]`,
and it does not depend on how many frames the caller asks for. The returned
frames are the sampled ones with `s ≤ s*`.

**How `s*` is found, and why a scan alone is not enough.** A stretch where
the trunk would pass horizontal sits around a local maximum of `sin θ(s)` (or,
for leaning back, a local minimum). Near the window's upper edge that stretch
shrinks to nothing: at 31.258786790912° bodyweight-only, `sin θ` peaks at
1.000000001 and the hole is about 5e-5 of the range wide, so any fixed scan
can step over it. The first implementation did exactly that, reported
`Bottom`, and returned frames cut short. So a fixed grid of 4096 steps samples
`sin θ(s)`, and **every sampled local extremum is refined by golden-section
search**. A hole is found through its peak, which the grid does see, rather
than through a sample landing inside it. The first sample outside the domain,
or the first refined extremum outside it, is then bisected to 1e-9, and its
balanced side is returned. The stated assumption is that `sin θ(s)` is smooth
with no two extrema closer than one step; the legs are a few trigonometric
terms in `s`, which satisfies it by orders of magnitude. If that assumption
ever failed, `RangeEnd::Bottom` still cannot come with frames missing: the
end is downgraded to `Stopped` at the last frame that balanced.

The result says how the range ended:

```rust
pub enum RangeEnd {
    /// Reached the bottom; the last frame is slice A's baseline.
    Bottom,
    /// Stopped at `reached` = s*. `bottom` is slice A's outcome at s = 1:
    /// `Some(rejection)` when the bottom itself does not balance, `None`
    /// when it does but this convention passes horizontal on the way.
    Stopped { reached: f64, bottom: Option<Rejection> },
}
```

Both stops happen:

- **The ankle stops it.** Fixture at 25°, bodyweight-only: `s* = 0.565` (knee
  65°, trunk at horizontal), and `bottom` carries slice A's
  `DepthNotReachable` with `needed_deg` 28.7. "How deep does this lifter get
  with their ankle" is the depth knob (slice B), which needs no convention;
  B1's `reached` is the convention's answer.
- **The convention stops it, though the bottom balances.** Fixture at 30°,
  bodyweight-only: slice A solves the bottom (trunk 74.5°), but under the
  convention the trunk passes horizontal for `s` between about 0.69 and 0.94.
  The range stops at `s* ≈ 0.69` with `bottom: None`, and the result
  attributes the stop to the convention, not the ankle. The window is narrow
  (dorsiflexion between 28.7° and 31.26° bodyweight-only, 13.4° to 13.78°
  under 100 kg; the upper edge is where the largest `sin θ(s)` reaches 1), and
  it is why the trunk's non-monotone path is a property of
  the convention worth replacing with measured kinematics first.

**Rejected outright**, with no frames:

- **A dorsiflexion below the upright lean** (`δ < λ`, so 0–4° for the
  fixture) would drive the shank backwards along the range:
  `DepthNotReachable { limiting: Ankle, needed_deg: λ, available_deg: δ }`.
- **A body that cannot stand balanced** (no mass on the trunk line and no
  bar, so `λ` is undetermined): `DoesNotBalance { unknown: ShankAngle }`,
  because the upright's unknown is its lean.

**What each frame admits to.**

| frame | assumptions |
|---|---|
| bottom (`s = 1`) | exactly `analyse_baseline`'s list, unchanged |
| upright (`s = 0`) | `SinglePosture { at: Upright }`, `SolvedBaseline`, the new `StraightBodyUpright` |
| between | `SinglePosture { at: Partial }`, `SolvedBaseline`, the new `JointsAdvanceTogether` (its doc names both ends) |

`SolvedBaseline`'s doc is generalised to "solved from this lifter's lengths
under a stated convention: the §2.6.1 baseline at the bottom, the §2.6.6
range elsewhere". `JointsAdvanceTogether` is not on the upright, where no
joint advances.

**Sampling.** The caller passes a `FrameCount` (`FrameCount::new(u16) ->
Option<Self>`, 2 to 241, a resource bound). Frame `i` sits at
`s = i as f64 / (n − 1) as f64`, computed directly and never accumulated, so a
frame at a given `s` is bit-identical whatever `n` is. The progress is a
coordinate of the convention and lives on the frame, not in `Position`, which
stays a unit enum so `Assumption` keeps `Eq` and `Hash`.

```rust
#[derive(Serialize)]
pub struct RangeFrame {
    pub progress: f64,           // s
    pub shank_deg: f64,          // α(s), the shank in use, not the lifter's δ
    pub knee_flexion_deg: f64,   // s·(δ + 90°): index video comparison by this, not by s
    pub hip_flexion_deg: f64,
    pub trunk_lean_deg: f64,
    pub posture: Posture,        // the points a reel or a video overlay draws
    pub mechanics: Mechanics,
}

#[derive(Serialize)]
pub struct SquatRange {
    pub frames: Vec<RangeFrame>, // upright to s*, in order
    pub end: RangeEnd,
}

pub fn squat_range(
    body: &Body,
    bar_kg: Option<LoadKg>,
    ankle: Dorsiflexion,
    frames: FrameCount,
) -> Result<SquatRange, Rejection>;
```

`Point` and `Posture` gain `Serialize`, so the points reach a reel or a
client without anyone recomputing where the knee is.

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

**What produces a `Mechanics`** — the draft named the type and no function that
builds one, so AC3, AC4, AC5 and AC12 had nothing to be tested against:

```rust
/// A posture the v1 solve produced, and the evidence that it did. Only
/// `squat::baseline` can build one.
pub struct Baseline(Posture);

/// The v1 model entry point: the four joint moments and everything the result
/// must admit to. Total — a posture that exists has moments (§2.4).
pub fn analyse_baseline(body: &Body, solved: &Baseline) -> Mechanics;

/// Source + model in one call: solve this lifter's baseline and analyse it.
pub fn baseline_mechanics(
    body: &Body,
    bar_kg: Option<LoadKg>,
    ankle: Dorsiflexion,
) -> Result<Mechanics, Rejection>;
```

**Why the entry point is named for its source.** `SolvedBaseline` and
`SinglePosture` are facts about *how the posture was made*, and §2.1's whole
point is that the model cannot see that. A single `analyse(body, p)` would
have to assert both — which is true today and a lie the day the R-0044 source
lands. So the model's core is private and each source names its own door:
`analyse_measured` arrives with the measured slice, sharing that core. The
alternative — a `Provenance` argument — was rejected twice over: it would let a
caller mislabel a posture, and `Provenance` is already taken at the crate root
by `archetype::Provenance` (the finding-10 collision, again).

**And the door is locked, not merely labelled.** `analyse_baseline` takes a
`Baseline`, not a `Posture`. Naming a function for its source only documents
the requirement; taking a type only that source can build *enforces* it, so a
measured or hand-built posture cannot be handed to it and come back carrying
`SolvedBaseline`. The wrapper costs one field and is the difference between a
comment and a guarantee.

**The load is `LoadKg`, the crate's validated load** — finite and within
`(0, 1000]` (`workout.rs`). With a bare `f64` a negative bar solved to a
plausible-looking posture, and `NaN` fell through to `DoesNotBalance`: a
physics rejection standing in by accident for what is a type error. The cap has
one consequence worth recording — `B5b`'s limit claim cannot be probed at 10⁴
kg, so it is stated as the convergence it is, over `100 → 250 → 500 → 1000`.

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
`uncertainty` is the half-width from propagating the **placement conventions**
through the same `moment_about` — the L5/S1 placement along the trunk line
(±3 cm, convention) and the head & neck and arm COM placements (±2 cm each,
convention) — by evaluating the moment at the corner cases and taking the
largest deviation. All three reach all four joints: the L5/S1 point is the
lumbar row's own joint centre, but it is *also* the proximal end of the pelvis
segment, whose centre of mass sits on every load side — so it is not the
lumbar-only spread the draft called it. No
distribution is assumed; the number is a bound from stated inputs, and each
input is a labelled convention until cited. The top-level `Uncertainty` names
those spreads, so the ± can be audited instead of trusted:

```rust
pub struct Uncertainty {
    /// L5/S1 placement along the trunk line — the lumbar row only.
    pub l5s1_placement: Metres,
    /// Head & neck and arm COM placement, along the line each is placed on.
    pub com_placement: Metres,
}
```

**The segment-length spread is not in here, and that is a correction.** The
draft listed ±5% on every table-derived length as a third input to this same
propagation. It cannot be: at a **fixed** posture the joint centres are given,
so a different femur length would change nothing — the effect of getting a
length wrong is that the lifter's *posture* is different, which only a re-solve
can show. That is §2.6.5's stability claim (`B12`), and it lives there because
it belongs to the source, not to the model. Two different quantities had been
given one name.

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
| **A** | `anthro` (table + sum claims + verification record); `Posture`, `Load`, `Point`/`Metres`; `above()` + `moment_about` with the sign rule; the lumbar row; the **baseline** squat solve with the system-COM constraint; **`analyse_baseline` → `Mechanics`** with its assumptions, `LoadBasis`, `ResistingGroup` and placement uncertainty; the `Rejection` variants reachable without knobs (`DoesNotBalance`, `DepthNotReachable`) | nothing |
| **B1** | the range (§2.6.6): `squat_range` from the solved upright toward the §2.6 bottom, legs interpolated in angle by the stated convention, the trunk solved for balance in every frame, the largest balanced prefix and its `RangeEnd`; `Position::{Upright, Partial}`, `Assumption::{StraightBodyUpright, JointsAdvanceTogether}`, `FrameCount`, `Serialize` on `Point`/`Posture` | A; §5.7(a) |
| **E** | one engine (§5.7(b)): the model moves to its own crate with no I/O dependencies, `fitai-core` re-exports it, and the reels' squat consumes it | B1; AC15 amendment |
| **B** | the three knobs, `OutOfRange`, the which-unknown rule, deltas, `Borderline`, the pull | A; §5.1 amendment |
| **C** | `api::biomech`, persistence, coach export | B, R-0041 read, R-0040 |

Slice A's standalone value is the corrected, tested, cited model and the reel
correction — real for the funnel, nil for the product until C. It changes the
published numbers and should ship with the reel correction.

## 3. Code outline

```
backend/crates/core/src/biomech/
  mod.rs        Posture, Point, Metres, NewtonMetres, Load, Joint,
                Dorsiflexion, Unknown, Rejection — the seam types
  anthro.rs     WINTER table + verification record, Segment, Body, scale(),
                MeasuredLengths, the three sum claims
  chain.rs      above(joint), moment_about() — the ONLY place g is multiplied;
                Joint::extension_sign() — the ONLY place a sign is applied
  analysis.rs   Mechanics, JointMoment, ResistingGroup, LoadBasis, Assumption,
                Position, Uncertainty; analyse_baseline()               [A]
  range.rs      the convention only: λ by two probes, the angle lerp,
                sampling, s*; RangePosture { posture, at: RangeAt } with
                private fields; squat_range(), FrameCount               [B1]
  squat.rs      pose() — the ONE home for the baseline geometry;
                baseline(body, bar, ankle) -> Result<Baseline, Rejection>,
                baseline_mechanics()                                    [A]
  knobs.rs      Knobs, apply(), which_unknown()                         [B]
  delta.rs      Delta, Borderline, perturb()                            [B]

backend/crates/core/examples/
  squat.rs      a developer driver: solve a lifter, sweep the
                dorsiflexion relation, check the reel               [A]
```

`examples/squat.rs` is **tooling, not product code** — it ships with
`cargo run --example` and never with the API image. It exists because slice C
is what gives the model an API (§2.9), and without it slice A can only be
exercised through its own test suite, which is a poor way to find out whether
the numbers are believable. It adds no model: every figure it prints comes
from `biomech`, and its `--reel` mode goes through `moment_about` rather than
`analyse_baseline`, because the reel's posture is built rather than solved and
stamping `SolvedBaseline` on it would be a lie (§2.1).

`lumbar.rs` is folded away: the L5/S1 point is one
`along(hip, shoulder, L5S1_UP_THE_TRUNK)` on `Posture::at`, and its load side is
one of `above()`'s four lists. A module for two lines would have been structure
for its own sake — the §2.5 convention still has exactly one home, which was the
point of naming it.

**Provenance across sources (B1).** `squat.rs` stays the only home of the
geometry and the balance: `pose()` takes `Legs { shank: (sin, cos), femur:
(sin, cos) }`, `Legs::parallel(u)` is exactly today's call, and
`pub(super) fn balanced(body, bar, legs) -> Option<Posture>` solves the trunk
with the same two probes and returns `None` outside `sin θ ∈ [0, 1]`.
`range.rs` holds only the convention, plus a `RangePosture` it alone can
build (`at: RangeAt::{Upright, Partial}`; the bottom is never one). A second
door, `pub(super) fn analyse_range(body, &RangePosture) -> Mechanics`, holds
the per-position assumption lists, and `mechanics()` stays private. The bottom
frame comes from `squat::baseline` → `analyse_baseline`, so range code can
neither build a `Baseline` nor stamp `Parallel`. Rejected: `pub(super)
mechanics` would let any sibling, a future `measured.rs` included, stamp
`SolvedBaseline`; a generic `Solved<P>` is premature with two sources. The
`range` ↔ `analysis` module cycle is the same shape as today's `squat` ↔
`analysis`, inherent to a sibling's private constructor.

**The same rule applies to the balance coefficients.** `P`, `Q` and `B` could
be written out from the §2.4.2 `k`-fractions, and the draft's implementation
did — giving those conventions a second home free to drift from
`ChainSegment::com`. They are instead **measured off the model**: `Σ m·x` is
affine in `(sin δ, sin θ)` with no cross term, so three probe postures through
the real `com` determine all three exactly. Same conventions, one definition,
four extra evaluations per solve.

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
- Any **measured or predicted** φ axis in v1: a descent kinematic,
  velocities, per rep. The §2.6.6 range is a declared one-parameter family of
  solved postures, and anything read across it (a peak, a trend, a reach) is
  the convention's, not the lifter's.
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

### 5.5 Taken 2026-09-15

- **Baseline shank angle** → the lifter's weight-bearing ankle dorsiflexion
  (§2.6.1.1). Resolves the §2.6.1/§2.6.3 contradiction, which had left the
  baseline posture undefined.
- **Joint limits** → cited from observed lifting postures, not passive
  goniometric norms (§2.6.4). No limit constant is committed until a source is
  recorded, so slice A ships without `AnatomicallyImplausible`.

### 5.6 Raised 2026-09-17 — two things the owner should see

**(a) — Decided 2026-09-27: keep the derived value, and fix the arm (A + C).**
`L5S1_UP_THE_TRUNK = 0.2674` ships as derived from the HAT identity, guarded by
the fourth sum claim. A direct citation (de Leva, Chaffin) is a later upgrade,
not a blocker — de Leva divides the trunk at the omphalion and xiphion, not at
L5/S1, so citing him is a mapping exercise rather than a lookup. The hanging
arm's centre of mass is corrected to the whole arm's (§2.4.2).

*As raised:* **L5/S1 was wrong, and the published correction moves with it.** §2.5's
`0.10` was a convention nobody had checked. The table's own HAT row says it is
`0.2674`, and at `0.10` every hip moment the model produced was 16% low. The
reel correction is therefore **274 → 370.6 N·m**, not the ≈366 this spec
carried and not the 357 the first implementation produced. Nothing about §5.4
changes — whether to publish is still yours — but the number to publish is
370.6, and it now has two independent cross-checks (the bar-only component
comes back as the reel's own 273.6, and the lumped HAT row agrees to 0.2%).

The derivation attributes the whole split-versus-lump residual to L5/S1. That
is the defensible attribution and §2.5 says why, but it is an attribution: a
cited L5/S1 that disagrees would mean part of the residual belongs to the head
or arm placement instead. The §2.3 identity is now a test, so that disagreement
would surface rather than hide.

**(b) The Winter verification is the one thing blocking a clean merge.** AC2
says the record is filled in before any constant is committed. The constants
are committed and `WINTER_VERIFICATION` is still `Pending`; the test that says
so is `#[ignore]`d, which keeps CI honest about what it can check but means
nothing will raise it again on its own. It needs somebody with the printed 4th
edition and twenty minutes. Until then AC2 stays unticked, and every published
number rests on values that are canonical but unverified.

### 5.7 Taken 2026-10-17

**(a) The squat over its whole range — slice B1, §2.6.6.** The owner asked
for the squat from standing to the bottom as the base for a reel-style
technique video and, later, a frame-by-frame comparison with the lifter's own
video. Taken with the leg convention stated as one, the trunk solved in every
frame, and AC4 amended to say that the family, and anything read across it,
holds under that convention only.

**(b) One engine — §5.2 revisited, as §5.2 said to do "if slice B introduces
a φ axis worth sharing".** `guion-video-creator`'s README already says the
reels and Goose Physics share one engine, which this spec had decided against.
The two did not agree: the reel published 274 N·m, and this model gives
370.6. §2.2 had kept them apart on purpose, as two different models; having
the reels inherit the corrected one is the reason to change that. Taken: the model moves to a small crate of its own in this repository,
with no I/O dependencies. `fitai-core` re-exports it, and the reels' squat
(`physics-lab/mecanica`, `guion-fisica`) depends on it by path, the way they
already depend on `garust`. One implementation means the reels inherit the
corrected numbers instead of being corrected by hand. This is its own slice,
E, after B1. **This is a decision, not yet a design.** Before E is built it
needs its own section covering:

- (a) `LoadKg` lives in `fitai_core::workout`, so the new crate cannot use it
  without a cycle. Move it down (touches SPEC-0004) or give the crate its own
  validated type.
- (b) `wasm32-unknown-unknown`, which physics-lab's CI builds with
  `panic = "abort"`; this repository must build that target for the crate.
- (c) **Licence: owner decision.** `backend/Cargo.toml` says `UNLICENSED`,
  `mecanica` is `MIT OR Apache-2.0`, and all four repositories are public.
- (d) §2.2's "private repository" objection no longer holds, since sargentAI
  is public. Prefer a git dependency pinned by rev over a sibling path, since
  the local checkout is named `fitAI`.
- (e) **The reels' golden values: owner decision, tied to §5.4.**
  `guion-video-creator/fixtures/dorados.json` gates 135 published samples at
  2%, and the squat ones move 35% under the corrected model. They either pin
  the bar-only component (273.62, B3) or move with the published correction.
- (f) Scope here: extract the crate, `pub use <crate> as biomech;` (which
  keeps `fitai_core::biomech::…`), and add CI. Migrating the consumers, and
  correcting the "UN SOLO MOTOR" README, are follow-ups in those repositories.
- (g) Mark §2.2 and §5.2 superseded.
- (h) AC15, proposed: "**Pure core, one engine.** All mechanics live in a
  single pure crate in this repository (no I/O dependencies; builds for
  `wasm32-unknown-unknown`) as deterministic functions over a `Posture` plus
  profile and load. `fitai-core` re-exports it unchanged as
  `fitai_core::biomech`; any other consumer depends on that crate and
  implements none of the model." §6's "`Cargo.toml` unchanged" becomes: one
  crate, `cargo tree` with no I/O crates, a wasm32 build in CI.

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
      result carries `SinglePosture`; the range (B1) by R1–R14.
- [ ] **AC5** — `ResistingGroup` has four variants; no `Muscle` type exists.
- [ ] **AC6** (amended) — every v1 result carries `SolvedBaseline`; deltas are
      computed from the lifter's own baseline.
- [ ] **AC7** (amended) — three knobs with ranges; `OutOfRange` on every bound.
- [ ] **AC8** — `Delta`'s headline field is the percentage change.
- [ ] **AC9** — every `Rejection` variant a slice **can** reach is reached by a
      test through the public solve. Slice A: `DoesNotBalance` (B9) and
      `DepthNotReachable` (B5d, B11). `AnatomicallyImplausible` is unreachable
      by construction until a limit is cited (§2.6.4) and `OutOfRange` arrives
      with the knobs — neither may be tested against an invented number.
- [ ] **AC12** — `assumptions` is non-empty; `TableDerivedMasses` is
      unconditional; `RigidTrunkLumbar` iff the lumbar row; `KneesTrackFeet`
      iff stance ≠ baseline.
- [ ] **AC13** (amended) — parametric rejections are typed; measured deferred.
- [ ] **AC14** — no field or string in the output namespace matches
      `/risk|danger|safe|injur/i` (B16).
- [ ] **AC15** — `core::biomech` is the single home; `Cargo.toml` unchanged.
- [ ] **AC16** — §7 in full.

## 7. Test plan

**The table (AC2):**
- `B0` the **four** claims of §2.3 hold; every `WINTER` constant has a
  non-empty citation string. The fourth — the split reproduces `WINTER.hat`'s
  own centre of mass — is what fixes L5/S1 (§2.5), and is the only one of the
  four that constrains placement rather than mass.

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
- `B5` `BodyweightOnly` solves **at a dorsiflexion that reaches parallel**
  (≥ 40°, §2.6.1.1): a posture exists, its system COM is over the mid-foot
  within 1 mm, and `load` says bodyweight-only.
- `B5c` the dorsiflexion→lean relation of §2.6.1.1 is strictly monotone and
  reproduces that table within 1°: more ankle, less forward lean. This makes
  the restricted-ankle finding testable rather than anecdotal.
- `B5d` at 25° of dorsiflexion, bodyweight-only, parallel is
  `DepthNotReachable` with `limiting: Joint::Ankle` and `needed_deg ≈ 28.7`
  (§2.6.4) — **not** a posture, and **not** `AnatomicallyImplausible`. And
  `needed_deg` is the *least* ankle that works: a tenth of a degree more
  solves.
- `B5e` the same lifter at the same 25°, under a 100 kg bar, **solves**, and at
  a **matched** ankle the loaded squat is the more upright of the two: the
  counterweight claim of §2.6.1.1, and the proof that the rejection is a
  property of the lift rather than a label on the lifter.
- `B5b` as the bar grows the solved θ converges to the bar-over-mid-foot
  solution — the limit claim of §2.6.3. Stated over `100 → 250 → 500 → 1000`
  kg, strictly shrinking and inside 6 mm at the top: `LoadKg` caps at 1000, and
  a limit tested outside the type's domain is a claim about a lift nobody can
  log.

**Symmetry (AC16):**
- `B6` a mirrored posture gives mirrored moments (sign rule applied
  consistently).

**Documented shifts (AC16, slice B):**
- `B7` low-bar vs high-bar: hip moment ↑, knee moment ↓ — as *changes in the
  extension-resisting moment*, not raw signs.
- `B8` wider stance: femur projection shortens → hip moment ↓; result carries
  `KneesTrackFeet`.

**Rejection (AC9):**
- `B9` `DoesNotBalance` through the public solve — and **no** dorsiflexion in
  the domain rescues that body, or it should have been `DepthNotReachable`.
- `B11` `DepthNotReachable` through the public solve (see `B5d`).
- The other two rows of §2.6.4's table: `u > u_max` (a long shank and a short
  femur — the trunk would have to lean backwards) and the undetermined lean (a
  body with no mass on the trunk line and no bar).
- `B10` (`AnatomicallyImplausible`) is **deferred until a limit is cited**
  (§2.6.4): with no committed constant it is unreachable by construction, and a
  test for it would assert an invented number.

**Uncertainty (§2.8):**
- `B17` each joint's half-width is the sum of its three placement
  sensitivities, derived from the table rather than read off the result; the
  three non-lumbar rows are identical (the nudged segments are on all three
  load sides and none of those joints moves); the lumbar row is ~3.5× larger,
  because the joint centre moves against a load side that includes the bar.

**The range (§2.6.6, slice B1):**
- `R1` the last frame is `squat::baseline`'s output: same posture, same four
  moments, same assumptions, `Position::Parallel`, `end: RangeEnd::Bottom`.
- `R2` the first frame is the solved upright: knee and hip flexion 0, trunk
  lean equal to shank lean, system centre of mass over the mid-foot within
  1 mm, and the lean worked by hand (4.04° bodyweight, 3.35° under 100 kg).
- `R3` every frame balances, within 1 mm.
- `R4` the ankle moment is the same in every frame to within 1e-9 N·m, and
  equals the weight above the ankle × g × 0.0692 m, by hand (120.55 N·m under
  100 kg).
- `R5` knee flexion is exactly `s·(δ + 90°)` and rises strictly with `s`.
  (Hip flexion is not monotone in general; for the fixture it is, and the
  claim is stated for the fixture only.)
- `R6` with `n = 6`, the fixture under 100 kg reproduces the §2.6.6 table to
  0.1, with its largest hip moment at `s = 0.8` (373.6 N·m), a claim about
  the convention, which the test's doc comment says.
- `R7` fixture at 25°, bodyweight-only: `end` is `Stopped { reached ≈ 0.565,
  bottom: Some(DepthNotReachable { needed_deg ≈ 28.7, .. }) }`, `reached` is
  the same for `n` = 2, 6, 101 and 241, and no returned frame has `s > s*`.
- `R8` frames strictly between carry `Position::Partial` and
  `JointsAdvanceTogether`; the upright carries `Upright` and
  `StraightBodyUpright`; the bottom carries neither.
- `R9` a frame at a given `s` is bit-identical whatever `n` is, and
  `FrameCount` refuses fewer than 2 and more than 241.
- `R10` fixture at 30°, bodyweight-only: slice A solves the bottom, and the
  range stops at `s* ≈ 0.69` with `bottom: None`. The convention is blamed,
  not the ankle.
- `R11` a dorsiflexion below the upright lean (2° for the fixture) is
  `DepthNotReachable { needed_deg: λ, .. }` for the whole range, with no
  frames.
- `R12` a body with no mass and no bar is `DoesNotBalance { unknown:
  ShankAngle }` for the whole range (AC9).
- `R13` a hole narrower than the scan step still stops the range: at
  31.258786790912°, bodyweight-only, n = 143, `end` is `Stopped { reached ≈
  0.80984, bottom: None }`, derived independently of the crate.
- `R14` `Bottom` always comes with all `n` frames, the last one slice A's
  baseline, and `Stopped` with exactly the sampled frames with `s ≤ s*`,
  swept across the window's edge and four frame counts.

**Stability (AC16, finding 13):**
- `B12` every table-derived length ±5%: every delta keeps its sign or is
  `Borderline`.

**Honesty:**
- `B13` every `analyse_baseline` result carries `SagittalPlane`,
  `QuasiStatic`, `SinglePosture`, `SolvedBaseline`, `TableDerivedMasses` and
  `TableDerivedLengths`. In v1 that last one holds whatever the user has had
  measured — the foot, head & neck and upper arm have **no** measurement path,
  so measuring the three that do cannot remove it, and §2.8's `iff` reduces to
  *always*. Tested with all three measured as well as none, so that it stops
  reducing to *always* the day a fourth length becomes measurable rather than
  quietly continuing to.
- `B14` lumbar row present ⇒ `RigidTrunkLumbar` present.
- `B16` no field name or enum variant in the output namespace matches
  `/risk|danger|safe|injur/i` (AC14), and `ResistingGroup` has exactly the four
  variants of §2.7 (AC5).
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
| 2026-09-15 | Baseline shank angle = the lifter's ankle dorsiflexion | Owner decision. They are the same angle for a flat foot, it is measurable, and it makes the model reproduce why stiff ankles force forward lean. The draft left this undefined and the posture swung 175°→125° across plausible values. |
| 2026-09-15 | Limits cited from lifting kinematics; none committed until sourced | Owner decision. The placeholder 130° hip limit rejected the model's own baseline — a real parallel squat reaches 125–146°. A passive-ROM norm is the wrong instrument, and an uncited threshold is not a threshold. |
| 2026-09-15 | `DepthNotReachable` names the limiting joint | A restricted lifter failing to reach parallel is a true finding, not a model failure, and "your ankle stops you" beats a number for a posture they cannot hold. |
| 2026-09-12 | Every range is a labelled convention until cited | The draft called two of them physics; one was false and one unreachable (finding 6). |
| 2026-09-17 | `analyse_baseline`/`baseline_mechanics` produce the `Mechanics`; the model core is private and each source names its own door | The draft named the result type and no producer, so AC3/AC4/AC5/AC12 and B13/B14 had nothing to test. A source-agnostic `analyse` would have to assert `SolvedBaseline` — true today, false the day R-0044 feeds it; a `Provenance` argument would let a caller mislabel a posture, and collides with `archetype::Provenance`. |
| 2026-09-17 | Dorsiflexion is a **required** solve input in slice A; no population default | Follows the 2026-09-15 owner decision applied consistently: an uncited default is an invented constant, and unlike the Winter table no sum claim could catch it. The caller that lacks a measurement must decide what to say, and now cannot do so by accident. |
| 2026-09-17 | `DepthNotReachable` vs `DoesNotBalance` decided by §2.6.3's `[u_min, u_max]` window | Closed-form in the lifter's own lengths, so *"would more ankle fix this?"* is answered with no constant anyone must cite — which is what made the variant implementable while `AnatomicallyImplausible` is not. |
| 2026-09-17 | The ±5% segment-length spread leaves `Uncertainty` for §2.6.5 | At a fixed posture a different femur length changes nothing; its real effect needs a re-solve. The draft had given two quantities one name. |
| 2026-09-17 | `L5S1_UP_THE_TRUNK` is **derived** from the HAT identity (0.2674), not chosen | Table 4.1 describes the same 0.678 M twice, split and lumped; the split must reproduce the lump, and L5/S1 is the only placement in it the table does not supply. The draft's `0.10` left every hip moment 16% low and put the joint 5 cm above the trochanter. Raised to the owner in §5.6 — it moves the published reel correction. |
| 2026-09-17 | A fourth §2.3 sum claim, on placement rather than mass | The first three catch a mistyped mass and none catches a segment in the wrong place. Its absence is why the above went unnoticed through a spec, a red suite and a green implementation. |
| 2026-09-17 | `analyse_baseline` takes a `Baseline`, not a `Posture` | Naming a function for its source documents the requirement; taking a type only that source can build enforces it. One field, and `SolvedBaseline` becomes unforgeable. |
| 2026-09-17 | The load is `LoadKg`; `P`/`Q`/`B` are probed, not re-derived | A bare `f64` let a negative bar solve, with physics rejections standing in for a type error. Writing the `k`-fractions out a second time gave §2.4.2 two homes — the thing folding `lumbar.rs` away was meant to prevent. |
| 2026-09-27 | **Owner decision:** L5/S1 stays derived (0.2674); citation is an upgrade, not a blocker | The HAT identity is self-checking and the fourth sum claim fails if anything upstream moves. The head convention cannot absorb the residual (it would need the head's centre 26 cm above C7-T1), so attributing it to L5/S1 is close to forced, not merely chosen. |
| 2026-09-27 | **Owner decision:** the hanging arm hangs at the whole arm's centre of mass | The draft used the upper arm's centre alone — 14 cm below the shoulder against the arm's 30. Invisible in slice A's x-moments, wrong everywhere the arms move. Test-first: B15 went red on the new expectation before the fix. |
| 2026-09-17 | AC2 stays unticked; the verification test is `#[ignore]`d, not deleted | CI should assert what it can check and say plainly what it cannot. Tracked in §5.6(b) as the one item blocking a clean merge. |
| 2026-10-17 | **Owner decision:** the squat over its whole range, slice B1 (§2.6.6) | Base for the technique video and the video comparison. One progress scalar is the reels' own construction, stated as a convention. The trunk is solved by balance in every frame, so no frame is a posture that would fall over. |
| 2026-10-17 | Upright is the solved ankle-strategy lean, not "everything vertical" | With the ankle behind the mid-foot an all-vertical body does not balance. The straight body leaning 3–4° from the ankle is quiet standing (Winter 1995). |
| 2026-10-17 | **Owner decision:** one engine for the app and the reels (§5.7(b)) | The reel and the model disagreed by 35% on the same lift. Two implementations of one claim is how that happens; one crate both depend on is how it stops. |
| 2026-10-17 | The leg coupling is interpolated between two solved ends and declared; the trunk is never interpolated | Qualifies "Kinematic solve, never interpolation" (2026-09-12) and replaces "a φ axis would be fabricated" for the range: what is interpolated is named on every frame, and what physics can solve still is. |
| 2026-10-17 | Interpolate angles, not sines; the bottom frame is slice A's output | The two readings differ by 99 N·m at the fixture's hip and move its peak. Pinning one makes the frames reproducible; saying so on every cross-frame claim keeps them honest. |
| 2026-10-17 | The range is the largest balanced prefix, with an explicit `RangeEnd` | A middle frame can fail where the bottom balances (fixture at 30°). A `Result<(), Rejection>` could not say that, and a reach that changed with the frame count was not a result. |
| 2026-10-20 | `s*` refines every sampled local extremum of `sin θ(s)`, then bisects | The qa sign-off found a hole about 5e-5 wide near the window's edge that a fixed scan stepped over, reporting `Bottom` with frames cut short. Holes sit around extrema, which the grid does see. `Bottom` can no longer come with frames missing. |

## 9. Changelog

- _2026-09-12 — created (Draft)._
- _2026-09-12 — reworked in full after architect review (§10). Every required
  finding applied; three owner decisions isolated in §5._
- _2026-09-12 — **Accepted.** All three §5 decisions taken as recommended;
  requirement amended; bench struck; `joints` fixed at four._
- _2026-09-15 — amended after the qa red suite found two blocking defects the
  design had not resolved: the baseline shank angle was undefined (§2.6.1 and
  §2.6.3 contradicted each other) and the placeholder hip limit rejected the
  baseline itself. Both settled by owner decision; `DepthNotReachable` added._
- _2026-09-17 — amended to close the `Mechanics` gap (§2.8, §3, §2.10: the
  result type had no producer) and three defects the implementation surfaced:
  §2.6.1.1's table stated no load and reproduced neither case — corrected, with
  the counterweight finding it had been hiding; `DepthNotReachable` had been
  specified against a hip-flexion limit §2.6.4 forbids committing — replaced by
  the closed-form ankle window of §2.6.3; and `Uncertainty` had been given the
  segment-length spread, which a fixed posture cannot see._
- _2026-09-17 — amended again after architect review of the implementation,
  which found that the model contradicted its own table: the §2.4.2 split's
  centre of mass sat at 0.539 of the trunk line against `WINTER.hat`'s 0.626,
  unchecked, because every §2.3 sum claim was about mass and none about
  placement. `L5S1_UP_THE_TRUNK` is now derived from that identity (0.2674 for
  0.10), a fourth sum claim guards it, and every number in §2.4.3, §2.6.1.1 and
  §2.6.4 moved with it — the reel correction to **370.6 N·m**. Also: the
  §2.4.2 sentence justifying the split had the direction backwards; the load is
  `LoadKg`; `analyse_baseline` takes a `Baseline` so `SolvedBaseline` cannot be
  asserted about a posture nobody solved; and the balance coefficients are
  probed off `ChainSegment::com` rather than re-derived. Raised to the owner in
  §5.6._
- _2026-09-27 — owner decisions on §5.6(a): L5/S1 stays derived, and the
  hanging arm is placed at the whole arm's centre of mass rather than the upper
  arm's. §2.4.2 corrected; no slice-A moment changes._
- _2026-10-17 — owner decisions §5.7: the squat over its whole range (slice
  B1, §2.6.6), with one stated convention and the trunk solved per frame; and
  one engine for the app and the reels (slice E), revisiting §5.2. AC4
  amended._
- _2026-10-17 — architect review of B1 (ACCEPT WITH CHANGES) applied: the
  interpolation written as formulas (angles, not sines), cross-frame claims
  attributed to the convention, the range as its largest balanced prefix with
  `RangeEnd`, per-frame assumptions, the provenance seam, the rejections for
  `δ < λ` and an unbalanceable upright, §4 amended, and slice E recorded as a
  decision whose design is pending._

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
- _2026-10-20 — after the qa sign-off of B1's implementation: `s*` found by
  refining local extrema of `sin θ(s)` (a narrow hole near the window's edge
  had been stepped over), the `Bottom` invariant guarded, R13 and R14 added._
