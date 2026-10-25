# SPEC-0061 — Annex A: the six day cycles (OQ-A, for owner approval)

- **Status:** Draft v2. It awaits owner approval (SPEC-0061 §2.1, OQ-A) and
  blocks slice 2b only.
- **Realizes:** R-0061 AC6 (the plan is an ordered sequence of days)
- **Created:** 2026-10-23. **Revised:** 2026-10-24 after the second
  architect review (N7).

## 0. How to read this annex

**Every cycle is reconstructed.** No archetype record in
`core/src/archetype/seed.rs` lists exercises, and four of the six give no
day order either. The column *Order from* says, cycle by cycle, what comes
from the record and what is ours:

| Record | Order from | What the record's split prose gives |
|--------|------------|-------------------------------------|
| `heavy-duty-mass` (Yates) | **the record** | "4-day split (delts/triceps, back, chest/biceps, legs); each muscle once every ~6 days" |
| `high-intensity-minimalist` (Mentzer) | **the record** | "4-workout rotation (chest/back, legs, delts/arms, legs); each muscle every ~4-7 days" |
| `mass-monster-volume` (Cutler) | ours, within the record's structure | "5-day bodypart split; each muscle once weekly; back and legs split into two sessions" (no order) |
| `classic-aesthetic-taper` (Arnold) | ours | "high-volume double split, ~6 days/week; each major muscle ~2x/week" |
| `powerbuilder-leverage` (Columbu) | ours | "heavy compounds (squat/bench/deadlift) plus bodybuilding accessories; each muscle ~2x per ~14-day cycle" |
| `modern-precision-hypertrophy` (Heath) | ours | "5-day bodypart split; each muscle once weekly; ~3-5 exercises per bodypart" |

**Exercises are listed per day.** v1 joined a generic list per muscle
group, which put skull crushers on a chest-and-biceps day (second review,
N7). Every day below names its own exercises.
- Names stay in English, as curated data (SPEC-0061 §4).
- Sets, reps and loads are **not** part of the cycle. The record's own
  intensity, rest and progression guidance still say how to train.

**`focus`** uses the closed `MuscleGroup` vocabulary: `chest`, `back`,
`shoulders`, `arms`, `legs`, `core`. The client builds the day's name from
it, for example `[shoulders, arms]` → "Hombro y brazo".

## 1. The cycles

### `heavy-duty-mass` (Yates; order from the record)

A 6-day cycle with 4 training days, so each muscle comes round every 6
days, as the record says. That is about 4.7 training days a week.

| # | Day | `focus` | Exercises |
|---|-----|---------|-----------|
| 0 | Hombro y brazo | shoulders, arms | Seated Dumbbell Press, Lateral Raise, Reverse Pec Deck, Lying Triceps Extension, Triceps Pushdown |
| 1 | Espalda | back | Pulldown, Barbell Row, One-Arm Dumbbell Row, Deadlift, Back Extension |
| 2 | Descanso | — | — |
| 3 | Pecho y brazo | chest, arms | Incline Barbell Press, Flat Dumbbell Press, Pec Deck, Barbell Curl, Hammer Curl |
| 4 | Pierna | legs | Leg Extension, Leg Press, Hack Squat, Lying Leg Curl, Standing Calf Raise |
| 5 | Descanso | — | — |

Day 0 trains triceps and day 3 trains biceps. That matches the record's
"delts/triceps" and "chest/biceps".

### `high-intensity-minimalist` (Mentzer; order from the record)

An 8-day cycle with 4 training days, one session every other day: about
3.5 training days a week.

| # | Day | `focus` | Exercises |
|---|-----|---------|-----------|
| 0 | Pecho y espalda | chest, back | Pec Deck, Incline Press, Dips, Dumbbell Pullover, Pulldown, Barbell Row |
| 1 | Descanso | — | — |
| 2 | Pierna | legs | Leg Extension, Leg Press, Leg Curl, Standing Calf Raise |
| 3 | Descanso | — | — |
| 4 | Hombro y brazo | shoulders, arms | Lateral Raise, Overhead Press, Barbell Curl, Triceps Pushdown |
| 5 | Descanso | — | — |
| 6 | Pierna | legs | Back Squat, Leg Curl, Seated Calf Raise |
| 7 | Descanso | — | — |

- **The pairs are pre-exhaustion,** as the record says ("pre-exhaust
  supersets"): an isolation move straight into a compound one, such as Pec
  Deck then Incline Press, or Leg Extension then Leg Press.
- **No integer spacing fits the record exactly.** The rotation trains legs
  twice and everything else once. With a session every other day:
  - legs come round every 4 days, inside the record's "4–7";
  - chest, back, shoulders and arms come round every 8 days, one day over.

  A tighter spacing would bring legs inside 4 days. **For the owner:** keep
  8 days, or accept legs at 3?

### `classic-aesthetic-taper` (Arnold; order ours)

A 7-day cycle with 6 training days: the familiar chest-and-back,
shoulders-and-arms, legs order, twice through. The era's morning and
evening double sessions become one session per day, because the app logs
one session per plan day.

| # | Day | `focus` | Exercises |
|---|-----|---------|-----------|
| 0 | Pecho y espalda | chest, back | Bench Press, Incline Press, Dumbbell Flye, Wide-Grip Pull-Up, Barbell Row, Dumbbell Pullover |
| 1 | Hombro y brazo | shoulders, arms | Overhead Press, Lateral Raise, Barbell Curl, Incline Dumbbell Curl, Close-Grip Bench Press, Triceps Pushdown |
| 2 | Pierna | legs | Back Squat, Leg Press, Leg Extension, Leg Curl, Stiff-Leg Deadlift, Standing Calf Raise |
| 3 | Pecho y espalda | chest, back | Incline Dumbbell Press, Dips, Cable Crossover, T-Bar Row, Close-Grip Pulldown |
| 4 | Hombro y brazo | shoulders, arms | Seated Dumbbell Press, Upright Row, Preacher Curl, Concentration Curl, Overhead Triceps Extension |
| 5 | Pierna | legs | Front Squat, Hack Squat, Lying Leg Curl, Seated Calf Raise |
| 6 | Descanso | — | — |

The second pass varies the exercises, as high-volume routines of the era
did.

### `powerbuilder-leverage` (Columbu; order ours)

A 7-day cycle with 4 training days, each built on one of the record's
heavy compounds.

| # | Day | `focus` | Exercises |
|---|-----|---------|-----------|
| 0 | Pierna | legs | Back Squat, Front Squat, Leg Press, Leg Curl, Standing Calf Raise |
| 1 | Pecho y brazo | chest, arms | Bench Press, Incline Dumbbell Press, Dips, Close-Grip Bench Press, Barbell Curl |
| 2 | Descanso | — | — |
| 3 | Espalda y pierna | back, legs | Deadlift, Barbell Row, Pull-Up, Good Morning |
| 4 | Hombro y brazo | shoulders, arms | Overhead Press, Lateral Raise, Face Pull, Skull Crusher, Hammer Curl |
| 5 | Descanso | — | — |
| 6 | Descanso | — | — |

- **Day names come from `focus`,** like every other cycle (owner,
  2026-10-25). Programs can be built in many styles (5×5, high intensity,
  volume), so a day is named for what it trains, not for one lift. The
  physics check hangs on the **exercise** (Back Squat, Deadlift, Bench
  Press), not on the day's name.
- **The record contradicts itself.** Its `weekly_frequency_per_muscle` is
  2, but its own prose says "~2x per ~14-day cycle", which is once a week.
  This cycle trains legs twice a week and everything else once, which
  keeps both readings within the seed test's ±1. Correcting the record's
  field is a separate data fix.

### `mass-monster-volume` (Cutler; order ours, structure from the record)

A 7-day cycle with 5 training days. Back and legs each take two sessions,
as the record says.

| # | Day | `focus` | Exercises |
|---|-----|---------|-----------|
| 0 | Pierna | legs | Leg Extension, Back Squat, Leg Press, Hack Squat, Standing Calf Raise |
| 1 | Pecho | chest | Incline Dumbbell Press, Bench Press, Hammer Strength Press, Cable Crossover |
| 2 | Espalda | back | Pull-Up, Wide-Grip Pulldown, Close-Grip Pulldown, Straight-Arm Pulldown |
| 3 | Hombro y brazo | shoulders, arms | Seated Dumbbell Press, Lateral Raise, Rear Delt Fly, Barbell Curl, Triceps Pushdown |
| 4 | Pierna y espalda | legs, back | Lying Leg Curl, Romanian Deadlift, Barbell Row, T-Bar Row |
| 5 | Descanso | — | — |
| 6 | Descanso | — | — |

Day 0 is quads and day 4 is hamstrings plus back thickness, as the
exercises show. The names stay at the `focus` level, by the same rule as
Columbu's.

### `modern-precision-hypertrophy` (Heath; order ours)

A 7-day cycle with 5 training days: each muscle once a week, 3 to 5
exercises per bodypart, as the record says. v1 trained arms three times a
week, against the record (N7).

| # | Day | `focus` | Exercises |
|---|-----|---------|-----------|
| 0 | Pierna | legs | Leg Extension, Back Squat, Leg Press, Lying Leg Curl, Standing Calf Raise |
| 1 | Pecho | chest | Incline Dumbbell Press, Bench Press, Cable Crossover, Dips |
| 2 | Espalda | back | Pull-Up, Barbell Row, Seated Cable Row, Straight-Arm Pulldown |
| 3 | Hombro | shoulders | Seated Dumbbell Press, Lateral Raise, Rear Delt Fly, Face Pull |
| 4 | Brazo | arms | Barbell Curl, Incline Dumbbell Curl, Close-Grip Bench Press, Triceps Pushdown, Overhead Triceps Extension |
| 5 | Descanso | — | — |
| 6 | Descanso | — | — |

## 2. The frequency check (SPEC-0061 §2.1)

For every `MuscleGroup` in a cycle, the seed test computes how often it is
trained per 7 days (`count × 7 / len`). The result must lie within ±1 of
the record's `weekly_frequency_per_muscle`.

**`arms` is excluded.** It merges biceps and triceps, which split routines
train on different days, so Yates' two arm days are two different
muscles. The test's comment says so.

| Record | Field | chest | back | shoulders | legs | ✔ |
|--------|-------|-------|------|-----------|------|---|
| `heavy-duty-mass` | 1 | 1.17 | 1.17 | 1.17 | 1.17 | ✔ |
| `high-intensity-minimalist` | 1 | 0.88 | 0.88 | 0.88 | 1.75 | ✔ |
| `classic-aesthetic-taper` | 2 | 2 | 2 | 2 | 2 | ✔ |
| `powerbuilder-leverage` | 2 | 1 | 1 | 1 | 2 | ✔ (see the note above) |
| `mass-monster-volume` | 1 | 1 | 2 | 1 | 2 | ✔ |
| `modern-precision-hypertrophy` | 1 | 1 | 1 | 1 | 1 | ✔ |

## 3. Validation this annex must pass

- Every cycle has at least one training day. ✔ (4, 4, 6, 4, 5, 5)
- Every training day has a non-empty `focus` and at least one exercise. ✔
- No cycle exceeds 14 days. ✔ (the longest is 8)
- Every record's provenance notes gain the line "day cycle and exercises
  reconstructed for R-0061".

## 4. Questions for the owner

1. **Mentzer's spacing:** keep 8 days, or accept legs at 3?
2. **The exercises themselves:** change anything you would not program
   for that style.

## 5. Owner decisions recorded here

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-25 | Days are named by `focus` only; no lift or subtitle labels on `PlanDay::Training`. | Owner: programs come in many styles (5×5, high intensity, volume), so a day is named for what it trains. The physics check attaches to the exercise. |

## Changelog

- 2026-10-23 — v1: six cycles with a generic per-muscle exercise list.
- 2026-10-24 — v2, after the second architect review (N7):
  - exercises are listed per day;
  - each cycle's provenance is stated (order from the record for Yates and
    Mentzer only);
  - Yates is now a 6-day cycle, matching "once every ~6 days";
  - Mentzer is an 8-day cycle;
  - Heath trains each muscle once a week;
  - the frequency check is added;
  - three questions for the owner.
- 2026-10-25 — owner: days named by focus only; the lift and subtitle labels are dropped.
