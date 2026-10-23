# SPEC-0061 — Annex A: the six day cycles (OQ-A, for owner approval)

- **Status:** Draft — awaiting owner approval (SPEC-0061 §2.1, OQ-A)
- **Realizes:** R-0061 AC6 (the plan is an ordered sequence of days)
- **Created:** 2026-10-23

Every cycle below is derived from its record's own `split` prose in
`core/src/archetype/seed.rs`. The bodypart order is the record's; the
exercises are **not** in any record, so they come from the generic
`default_exercises` list in §2. Per SPEC-0061 §2.1, every cycle is
therefore labelled **reconstructed**, including the two records whose
splits are documented.

`focus` uses the closed `MuscleGroup` vocabulary (`core/src/workout.rs`):
`chest`, `back`, `shoulders`, `arms`, `legs`, `core`. It is coarse on
purpose: "delts/triceps" becomes `shoulders, arms`, and the client names
the day "Hombro y brazo".

## 1. The cycles

Day names are what the Spanish client will show, built from `focus`.

### `heavy-duty-mass` (HIT; split documented, cycle reconstructed)

Split: "4-day split (delts/triceps, back, chest/biceps, legs); each muscle
once every ~6 days". 7-day cycle, 4 training days.

| # | Day | `focus` |
|---|-----|---------|
| 0 | Hombro y brazo | shoulders, arms |
| 1 | Espalda | back |
| 2 | Descanso | — |
| 3 | Pecho y brazo | chest, arms |
| 4 | Descanso | — |
| 5 | Pierna | legs |
| 6 | Descanso | — |

### `high-intensity-minimalist` (HIT; split documented, cycle reconstructed)

Split: "4-workout rotation (chest/back, legs, delts/arms, legs); each muscle
every ~4-7 days". 12-day cycle, 4 training days, one session every third
day. The record's recovery window is 4–7 days; three days is the floor
that keeps a 14-day cycle limit while honouring the rotation.

| # | Day | `focus` |
|---|-----|---------|
| 0 | Pecho y espalda | chest, back |
| 1–2 | Descanso | — |
| 3 | Pierna | legs |
| 4–5 | Descanso | — |
| 6 | Hombro y brazo | shoulders, arms |
| 7–8 | Descanso | — |
| 9 | Pierna | legs |
| 10–11 | Descanso | — |

### `classic-aesthetic-taper` (high-volume; reconstructed)

Split: "high-volume double split, ~6 days/week; each major muscle
~2x/week". 7-day cycle, 6 training days. The era's morning/evening double
sessions collapse into one session per day: the app logs one session per
plan day.

| # | Day | `focus` |
|---|-----|---------|
| 0 | Pecho y espalda | chest, back |
| 1 | Hombro y brazo | shoulders, arms |
| 2 | Pierna | legs |
| 3 | Pecho y espalda | chest, back |
| 4 | Hombro y brazo | shoulders, arms |
| 5 | Pierna | legs |
| 6 | Descanso | — |

### `powerbuilder-leverage` (powerbuilding; reconstructed)

Split: "heavy compounds (squat/bench/deadlift) plus bodybuilding
accessories; each muscle ~2x per ~14-day cycle". Expressed as a 7-day
cycle: two passes make the record's 14 days, with each muscle twice.

| # | Day | `focus` |
|---|-----|---------|
| 0 | Pierna | legs |
| 1 | Pecho y brazo | chest, arms |
| 2 | Descanso | — |
| 3 | Espalda | back |
| 4 | Hombro y brazo | shoulders, arms |
| 5 | Descanso | — |
| 6 | Descanso | — |

### `mass-monster-volume` (high-volume; reconstructed)

Split: "5-day bodypart split; each muscle once weekly; back and legs split
into two sessions". 7-day cycle, 5 training days.

| # | Day | `focus` |
|---|-----|---------|
| 0 | Pecho | chest |
| 1 | Espalda | back |
| 2 | Hombro y brazo | shoulders, arms |
| 3 | Pierna | legs |
| 4 | Pierna, espalda y brazo | legs, back, arms |
| 5 | Descanso | — |
| 6 | Descanso | — |

### `modern-precision-hypertrophy` (modern hypertrophy; reconstructed)

Split: "5-day bodypart split; each muscle once weekly; ~3-5 exercises per
bodypart". 7-day cycle, 5 training days.

| # | Day | `focus` |
|---|-----|---------|
| 0 | Pierna | legs |
| 1 | Pecho y brazo | chest, arms |
| 2 | Espalda y brazo | back, arms |
| 3 | Hombro y core | shoulders, core |
| 4 | Brazo | arms |
| 5 | Descanso | — |
| 6 | Descanso | — |

## 2. `default_exercises(MuscleGroup)`

A training day's `exercises` is the concatenation of this list for each
group in its `focus`, in `focus` order. Names stay in English (SPEC-0061
§4). Loads, sets and reps are **not** part of the cycle: the record's own
intensity, rest and progression guidance still describe how to train.

| Group | Exercises |
|-------|-----------|
| chest | Barbell Bench Press, Incline Dumbbell Press, Cable Fly |
| back | Deadlift, Barbell Row, Pull-up, Lat Pulldown |
| shoulders | Overhead Press, Lateral Raise, Rear Delt Fly |
| arms | Barbell Curl, Triceps Pushdown, Incline Dumbbell Curl, Skull Crusher |
| legs | Barbell Squat, Romanian Deadlift, Leg Press, Leg Curl, Standing Calf Raise |
| core | Hanging Leg Raise, Cable Crunch |

## 3. Validation this annex must pass (SPEC-0061 §2.1)

- Every cycle has at least one training day. ✔ (4, 4, 6, 4, 5, 5)
- Every training day has a non-empty `focus` and at least one exercise. ✔
- No cycle exceeds 14 days. ✔ (longest: 12)
- Every record's provenance notes gain the line "day cycle and exercises
  reconstructed for R-0061".

## 4. Owner decisions recorded here

| Date | Decision | Rationale |
|------|----------|-----------|
| | *(awaiting owner)* | |
