# R-0039 — Program Authoring Model (trainer/self-authored programs)

- **Status:** Draft — authoring model confirmed by owner 2026-07-22
- **Milestone:** M-Platform (trainer marketplace) — layer A (authoring)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-07-22
- **Depends on:** R-0038 (periodization engine — load math + set types this
                  reuses), R-0015 (per-lift e1RM — the load anchor)
- **Realized by:** SPEC-0039 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

## 1. Statement

A domain model + materializer that lets a **trainer** (or a self-user) **author
their own program**: define exercises with **low / medium / high** intensity
classes, group them into **core** lifts and **accessories**, and **periodize**
them by placing each `(lift, class)` on **relative day indices** in a cycle
(e.g. squat *low* on day 1, *medium* on day 3, *high* on day 6). A pure
`materialize` step turns the authored program + the user's e1RM into concrete
prescribed sessions (reusing R-0038's load math), so the app can display and log
it. Calendars are **client-relative** — the program uses day *indices*, not
weekdays.

## 2. Rationale

This is the keystone of the trainer-platform pivot (see the trainer-platform
vision): it delivers "as a trainer, write my program" **and** "as a self-user,
build my own" — the piece every later layer (assignment, community,
subscriptions) sits on. It is buildable now, pure and testable, and needs no
roles/payments yet.

## 3. The authoring model (owner to confirm this is right)

- **AC1. Exercise with three intensity classes.** An authored exercise has a
  name and a prescription for each of **Low / Medium / High**. Each class =
  `{ warmup_sets, work: Vec<WorkSetLine> }`, and a `WorkSetLine` =
  `{ sets, reps, load_pct }` — **one or more lines** so a class can be a top set
  plus back-offs (e.g. High = `1×3 @90%` then `3×5 @80%`). `load_pct` is a
  fraction of the lift's e1RM.
- **AC2. Core vs accessories.** A program has a set of **core** exercises
  (the owner cited ~9 — not hard-capped in v1, but flagged if far outside a sane
  range) and a set of **accessory** exercises. Both are authored the same way.
- **AC3. Relative-day periodization schedule.** A `Schedule` has a `cycle_days`
  length and, for each day index `1..=cycle_days`, a list of
  `(exercise, intensity_class)` entries — e.g. day 1 = `[(Squat, Low), (Bench,
  Medium)]`. A given exercise can appear on several days at different classes
  (squat Low/Med/High on days 1/3/6). Rest days simply have no entries.
- **AC4. Client-relative calendar.** The schedule is day-*index* based; mapping
  day 1 → a real weekday/date is the client's choice and is **out of scope here**
  (a later assignment layer). The authored program carries no weekday.
- **AC5. Materialization.** `materialize(program, e1rm, plate_kg)` produces a
  concrete cycle: for each scheduled `(exercise, class)`, emit the warmup-set
  count and the work sets, each set `{ reps, intensity_pct, target_load_kg }`
  where `target_load_kg = round_to_plate(e1rm × load_pct)` (R-0038 load math);
  `None` load when the lift has no e1RM. Reuses R-0038's `PrescribedSet`.
- **AC6. Validation (typed, never panic).** Every schedule entry references an
  exercise that exists in the program (core or accessory); `cycle_days ≥ 1`; day
  indices within `1..=cycle_days`; every `load_pct ∈ (0,1]`; every `reps ≥ 1`;
  `plate_kg` finite and > 0; no blank exercise names; a program has ≥ 1 exercise
  and ≥ 1 scheduled entry. Invalid input returns a typed error.
- **AC7. Pure + deterministic + serializable.** The model + materializer are pure
  `fitai-core` (no I/O, no clock); same inputs ⇒ same output; every type
  round-trips through JSON.
- **AC8. Tests.** Author a small program (e.g. squat/bench/deadlift core + 2
  accessories, squat L/M/H on days 1/3/6); assert the materialized cycle's shape,
  loads (%×e1RM plate-rounded), no-e1RM `None` path, warmup counts, and each
  validation error; determinism + serde round-trip.

## 4. Constraints & non-goals

- Pure model + materializer only. **Explicitly out of scope (later layers):**
  - Persistence / CRUD endpoints for authored programs.
  - The mobile **program-builder UI**.
  - **Roles** (trainer/client/self), assignment to clients, trainer visibility.
  - **Community/roster**, **subscriptions/billing** (M7).
  - Program-tied meal plans.
  - Weekday/date calendars (client-relative mapping).
- Loads anchor on e1RM (%1RM), consistent with R-0038. No RPE model, no ML.

## 5. Open questions (deferred to SPEC-0039)

- **OQ-1 (resolved):** a class carries **multiple** `WorkSetLine`s (top set +
  back-offs). Materialization expands each line's `sets` count into that many
  `PrescribedSet`s (v1 emits a flat work-set list).
- **OQ-2:** Do warmups carry their own loads (ramp), or just a count in v1?
  (Lean: count only; ramp is a later nicety.)
- **OQ-3:** How does a materialized cycle relate to R-0038's `PeriodizedProgram`
  (a 4th "authored" scheme, or a distinct `MaterializedCycle` type)? (Lean:
  distinct type reusing `PrescribedSet`.)
- **OQ-4 (resolved):** **one repeating cycle** for v1 — no week-to-week
  progression; that is a later requirement.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-22 | Authoring model is the first platform layer (before roles/payments) | Owner choice — keystone everything else needs; buildable now. |
| 2026-07-22 | Relative day indices, calendar client-side | A trainer's program is portable across clients who start on different weekdays. |
| 2026-07-22 | Reuse R-0038 load math + `PrescribedSet` | Server/client agree on numbers; one load path. |
| 2026-07-22 | Multiple work-set lines per class; one repeating cycle (v1) | Owner choices — top-set/back-off expressiveness; defer weekly progression. |

## Changelog

- _2026-07-22 — created (Draft); pending owner confirmation of §3 before spec._
