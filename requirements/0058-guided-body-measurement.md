# R-0058 — Guided body measurement for the physics

- **Status:** Draft
- **Milestone:** M6 (the lift model's inputs)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-10-20
- **Depends on:** R-0003 (profile: height), R-0045 (the model that consumes
  these measurements)
- **Realized by:** SPEC-0058 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

The app guides a lifter through measuring what the lift model needs. That is
the height from the floor of four body landmarks (ankle, knee, hip, shoulder)
and their weight-bearing ankle dorsiflexion. The app checks that the numbers
are physically consistent before keeping them, stores them on the profile,
and uses them in the lift model in place of table proportions. Each
exercise's guide says which of these measurements it uses and links to the
measuring guide.

## 2. Rationale

- **The model's advantage is the lifter's own body.** R-0045 runs on table
  proportions until it has measurements, and table proportions are a
  population average. For a long-femured lifter the difference changes the
  answer: the ankle needed to reach parallel, and the trunk lean that
  balances there.
- **Measuring segments along the skin fails, and it fails quietly.** The
  first real measurements taken for this project came out 15–20% long for
  the lifter's height. Stacked up, they put the shoulder about 20 cm higher
  than a body that size allows. A tape that follows the body's curves, or a
  landmark taken from the wrong bone, produces numbers that look plausible
  one at a time. The model then returns precise answers for a body that is
  not the lifter's. **Heights from the floor**, read off a vertical tape, do
  not follow curves. The segments are their differences, and they cannot add
  up to more than the lifter's height.
- **Dorsiflexion is a required input with no default.** SPEC-0045 §2.6.1.1
  commits no population value, so without a guided measurement the model
  cannot run for a lifter who has not been measured.
- **The guide belongs in the app, at each exercise** (owner, 2026-10-20), not
  in a chat.

## 3. Acceptance criteria

- **AC1. The guide.** A step-by-step guide with an illustration for each
  landmark:
  - ankle: the centre of the outer ankle bone;
  - knee: the outer joint line;
  - hip: the greater trochanter;
  - shoulder: the centre of the shoulder joint, just below the bony tip.

  Each is measured standing barefoot against a vertical tape. Dorsiflexion is
  measured by the knee-to-wall test: the heel stays down, and the phone's
  level is laid along the shin to read degrees from vertical. Both legs are
  measured and the lower value is kept.
- **AC2. Segments from heights.** The app derives shank = knee − ankle, femur
  = hip − knee, trunk = shoulder − hip, and shows them before saving.
- **AC3. Consistency before storage.**
  - **Hard bounds block saving:** the heights must rise in order (ankle <
    knee < hip < shoulder < standing height).
  - **Soft bounds warn:** each segment must fall inside a plausibility band
    for the lifter's height. A value outside its band triggers a warning
    that names the measurement that looks wrong and says how to re-measure
    it.
  - **Nothing out of bounds is saved silently.**
- **AC4. Dorsiflexion input.** Degrees in `0..=90`. A negative reading from
  the phone is taken as a direction, with a one-line explanation. A reading
  above 90 is refused with a message.
- **AC5. Stored on the profile.** Per user, editable, with the date measured.
  Measurement history is not required in v1.
- **AC6. Used by the model.** The R-0045 model uses the stored lengths
  (`MeasuredLengths`) and the stored dorsiflexion whenever they exist. Its
  result still says which lengths came from the table: the foot, head and
  arms always do.
- **AC7. Per-exercise guide.** Each exercise's guide lists the measurements
  it uses (the squat uses all four heights and dorsiflexion) and links to the
  measuring guide. An exercise the model does not cover yet says so.
- **AC8. Honest without measurements.** With no measurements stored, the
  squat analysis says it is using table proportions. It asks for a
  dorsiflexion measurement rather than inventing one.
- **AC9. Privacy.** These are body measurements, so R-0024's data inventory
  gains rows for them before launch.

## 4. Constraints & non-goals

- **Not from photos or video.** Measuring from images is R-0044's measured
  source.
- **No on-device inference** (`project-specifics.md`).
- **Plausibility bands are labelled conventions until cited**, the R-0045 AC2
  precedent.
- **In v1 only the squat consumes these.** Other exercises list "not modelled
  yet".

## 5. Open questions

- **OQ-1.** The plausibility bands' source and width. Candidates are Winter
  Figure 4.1's fractions with a stated spread, or an anthropometric survey
  such as ANSUR II. The bands are the most consequential choice here: too
  tight rejects real bodies, too loose lets the error that motivated this
  requirement through.
- **OQ-2.** May a lifter keep a value that only fails a soft band, after an
  explicit confirmation? Recommended yes. Hard bounds are never overridable.
- **OQ-3.** Should the app prompt a re-measure later, given that
  dorsiflexion changes with mobility work? Deferred to v2.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-20 | Measure landmark heights from the floor, not segment lengths along the skin | The first real segment measurements were mutually inconsistent with the lifter's height by about 20 cm at the shoulder. A vertical tape follows no curves, and differences of heights cannot sum past the standing height. |
| 2026-10-20 | The measuring guide lives in the app, linked from each exercise | Owner request. |
