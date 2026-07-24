# SPEC-0039 — Program authoring model (`core::authoring`)

- **Status:** Draft
- **Realizes:** R-0039
- **Author:** Claude (main session)
- **Created:** 2026-07-22
- **Depends on:** SPEC-0038 (`PrescribedSet`, the load/plate math it reuses),
  SPEC-0015 (`current_e1rm`, lift keying).
- **Module(s):** `backend/crates/core/src/authoring.rs` (new — pure);
  a small `pub(crate)` extraction in `periodize.rs` for the shared load helpers.

## 1. Motivation

Realizes [R-0039](../requirements/0039-program-authoring.md): the trainer/self
authoring model — exercises with Low/Med/High classes (each a top-set +
back-off list), core + accessories, a relative-day schedule — and a pure
`materialize` that turns it + the user's e1RM into concrete sessions.

## 2. Design

### 2.1 Authored model

```rust
#[serde(rename_all = "snake_case")]
pub enum IntensityClass { Low, Medium, High }

/// One work-set line: `sets` × `reps` at `load_pct` of e1RM.
pub struct WorkSetLine { pub sets: u32, pub reps: u32, pub load_pct: f64 }

/// A class prescription: a warm-up count plus one or more work lines
/// (top set + back-offs).
pub struct ClassPrescription { pub warmup_sets: u32, pub work: Vec<WorkSetLine> }

/// An authored exercise with a prescription per class.
pub struct AuthoredExercise {
    pub name: String,
    pub low: ClassPrescription,
    pub medium: ClassPrescription,
    pub high: ClassPrescription,
}

/// A `(exercise, class)` placed on a day.
pub struct ScheduleEntry { pub exercise: String, pub class: IntensityClass }

/// A relative-day schedule. `days[i]` is day index `i+1`; an empty inner vec is
/// a rest day. Cycle length = `days.len()`.
pub struct Schedule { pub days: Vec<Vec<ScheduleEntry>> }

pub struct AuthoredProgram {
    pub name: String,
    pub core: Vec<AuthoredExercise>,
    pub accessories: Vec<AuthoredExercise>,
    pub schedule: Schedule,
}
```

All derive `Clone, Debug, PartialEq, Serialize, Deserialize` (AC7).

### 2.2 Materialized output

Reuses R-0038's `PrescribedSet { reps, intensity_pct, target_load_kg }`.

```rust
pub struct MaterializedEntry {
    pub lift: String,
    pub class: IntensityClass,
    pub warmup_sets: u32,
    pub work_sets: Vec<PrescribedSet>,  // work lines expanded: a line{sets:N} → N sets
}
pub struct MaterializedDay { pub day_index: u32, pub entries: Vec<MaterializedEntry> }
pub struct MaterializedCycle { pub cycle_days: u32, pub days: Vec<MaterializedDay> }
```

Rest days appear as a `MaterializedDay` with empty `entries` (so day indices stay
aligned).

### 2.3 Entry point

```rust
pub fn materialize(program: &AuthoredProgram, e1rm: &E1rmMap, plate_kg: f64)
    -> Result<MaterializedCycle, AuthorError>;
```

For each `ScheduleEntry`, resolve the exercise (case-insensitive over core ∪
accessories), pick its `class` prescription, and expand every `WorkSetLine` into
`sets` copies of `PrescribedSet { reps, intensity_pct: load_pct,
target_load_kg: load(e1rm, load_pct, plate_kg) }`. `e1rm` and lookups key by
`lift_key(name)` = `name.trim().to_lowercase()` (shared with R-0015/R-0038 via a
`pub(crate)` helper). Missing e1RM ⇒ `None` load (AC5).

### 2.4 Errors (AC6)

```rust
#[derive(…, thiserror::Error)]
pub enum AuthorError {
    NoExercises, BlankExercise, DuplicateExercise(String),
    NoScheduleDays, NoScheduledEntries, UnknownExercise(String),
    EmptyWorkLines, BadIntensity, BadReps, BadSets, BadPlate,
}
```

Validation before materializing (fail fast, never panic):
- `plate_kg` finite and `> 0` → else `BadPlate`.
- `core ∪ accessories` non-empty → else `NoExercises`; no blank exercise name →
  else `BlankExercise`; a **case-insensitive duplicate name is `DuplicateExercise`**
  (rejected, not silently shadowed — architect review).
- `schedule.days` non-empty → else `NoScheduleDays`; at least one entry across
  all days → else `NoScheduledEntries`.
- Every `ScheduleEntry.exercise` resolves in `core ∪ accessories` → else
  `UnknownExercise(name)`.
- For every exercise, **all three** class prescriptions: `!work.is_empty()` →
  `EmptyWorkLines`; every line `load_pct ∈ (0,1]` → `BadIntensity`, `reps ≥ 1` →
  `BadReps`, `sets ≥ 1` → `BadSets`. (Validate all three so an authoring mistake
  surfaces even on a class not yet scheduled.)

Materialization resolves each entry with a **fallible** `by_key.get(...).ok_or(
UnknownExercise)` and `collect::<Result<_,_>>()` — never an index panic, so AC6's
"never panic" holds *by construction*, independent of `validate` (architect
review). Day indices are valid by construction (index = position+1).

### 2.5 Shared helpers

Extract from `periodize.rs` as `pub(crate)`:
`round_to_plate`, `load`, and `lift_key(&str) -> String`. `authoring` reuses them
so the two modules cannot drift on load math or keying. Note: `lift_key` trims +
lowercases; R-0015's `aggregate.rs` inlines only `to_lowercase` — keys agree
because e1RM inputs are already trimmed. A future `core::loadmath` module could
hold these + `PrescribedSet`/`E1rmMap` so `periodize` isn't the de-facto
foundation (deferred; the anti-drift goal is met either way).

## 3. Code outline

```rust
pub fn materialize(prog, e1rm, plate_kg) -> Result<MaterializedCycle, AuthorError> {
    validate(prog, plate_kg)?;
    let by_key: BTreeMap<String, &AuthoredExercise> = prog.core.iter()
        .chain(&prog.accessories)
        .map(|e| (lift_key(&e.name), e))
        .collect(); // first-wins on dup via entry-or-insert
    let days = prog.schedule.days.iter().enumerate().map(|(i, entries)| {
        let materialized = entries.iter().map(|se| {
            let ex = by_key[&lift_key(&se.exercise)]; // validated to exist
            let p = ex.prescription(se.class);
            let e = e1rm.get(&lift_key(&ex.name)).copied();
            let work_sets = p.work.iter().flat_map(|l|
                std::iter::repeat(PrescribedSet {
                    reps: l.reps, intensity_pct: l.load_pct,
                    target_load_kg: load(e, l.load_pct, plate_kg),
                }).take(l.sets as usize)).collect();
            MaterializedEntry { lift: ex.name.clone(), class: se.class,
                                warmup_sets: p.warmup_sets, work_sets }
        }).collect();
        MaterializedDay { day_index: (i as u32)+1, entries: materialized }
    }).collect();
    Ok(MaterializedCycle { cycle_days: prog.schedule.days.len() as u32, days })
}
```

**Deliberate v1 choices (architect review):** (a) R-0039 AC2's "flag if far
outside ~9 core lifts" is **deferred** — a pure materializer has no warning
channel and it is explicitly not an error; a builder-UI hint is the right home.
(b) All three class prescriptions are validated even when a class is never
scheduled (faithful to "a prescription for each of Low/Med/High"); relaxing this
to `Option<ClassPrescription>` is a future ergonomic improvement.

## 4. Non-goals

Per R-0039 §4: no persistence, endpoint, builder UI, roles, assignment,
community, subscriptions, meal plans, weekday calendars, or ML.

## 5. Open questions

- **OQ-2:** warmups are a count in v1 (no per-warmup load ramp).
- **OQ-3:** `MaterializedCycle` is a distinct type reusing `PrescribedSet` (not a
  4th `PeriodizationScheme`) — an authored cycle is not a scheme-generated
  mesocycle.

## 6. Acceptance criteria

Maps R-0039 AC1–AC8: model with L/M/H classes + work-set lines (AC1); core +
accessories (AC2); relative-day schedule (AC3); no weekday (AC4); materialize
with plate-rounded `%×e1RM` and `None` path (AC5); typed validation, never panic
(AC6); pure/deterministic/serde (AC7); the worked-example + per-error tests
(AC8).

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-22 | Cycle length = `schedule.days.len()`; rest = empty day | No separate `cycle_days` field to drift; day index by construction. |
| 2026-07-22 | Validate all three class prescriptions, not just scheduled ones | Surface authoring mistakes early, even on unscheduled classes. |
| 2026-07-22 | Extract shared `round_to_plate`/`load`/`lift_key` `pub(crate)` | One load path + one keying rule across periodize + authoring. |
| 2026-07-22 | `MaterializedCycle` distinct from `PeriodizedProgram` | Authored ≠ scheme-generated; reuse only `PrescribedSet`. |
| 2026-07-22 | Case-insensitive duplicate name = `DuplicateExercise` error | Stricter + cheaper to reason about than first-wins shadowing. |
| 2026-07-22 | Materialize via fallible `get().ok_or`, not `Index` | AC6 "never panic" by construction, not by cross-fn invariant. |

Public exports (via `lib.rs`): `materialize`, `AuthorError`, `AuthoredProgram`,
`AuthoredExercise`, `ClassPrescription`, `WorkSetLine`, `IntensityClass`,
`Schedule`, `ScheduleEntry`, `MaterializedCycle`, `MaterializedDay`,
`MaterializedEntry`.

## Changelog

- _2026-07-22 — created (Draft)._
