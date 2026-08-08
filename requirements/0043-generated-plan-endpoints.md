# R-0043 — Generated Plan Endpoints (periodization engines over HTTP)

- **Status:** Draft
- **Milestone:** M-Platform / M5
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-08-04
- **Depends on:** R-0038 (the three engines this exposes), R-0015 (e1RM anchor),
                  R-0041 (the persistence + materialize pattern to mirror)
- **Realized by:** SPEC-0043 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

## 1. Statement

Expose R-0038's periodization engines over HTTP so a user can **generate several
different plans** — linear, undulating (DUP), and block — preview them against
their own e1RM, and keep one.

R-0038 merged in #75 with all three engines implemented and tested, and like
`core::authoring` before R-0041, it is currently unreachable: there is no
endpoint, so no client can generate a plan.

## 2. Rationale

"Different workout plans" is the user-visible payoff of R-0038. The engines are
built, architect-reviewed and tested; the only thing between them and the app is
a thin, owner-scoped HTTP layer that mirrors what R-0041 established for authored
programs.

## 3. Acceptance criteria

- **AC1. Preview a plan.** `POST /plans/preview` takes a scheme
  (`linear` | `undulating` | `block`) plus its parameters and returns the
  generated `PeriodizedProgram` **without persisting anything**, with loads
  computed against the caller's own e1RM.
- **AC2. Compare schemes.** The same parameters can be run through each scheme,
  so a client can show the three side by side. No endpoint state is carried
  between calls.
- **AC3. Typed rejection.** Invalid parameters return **422** with a fixed
  reason token derived from R-0038's `PlanError` — never a 500, never free text.
  Mapping is an exhaustive match, as in R-0041.
- **AC4. Same e1RM definition.** Loads use `core::aggregate::current_e1rm` over
  the caller's sessions — the single definition R-0041 established. A lift not
  trained inside the window yields a `null` load, never a fabricated one.
- **AC5. Bounded generation.** Plan size is bounded before generation: weeks,
  sessions per week, sets and lift count all carry explicit magnitude caps with
  typed errors. Learned from R-0041 — an unbounded client-controlled expansion
  behind an endpoint is a memory-exhaustion DoS.
- **AC6. Keep one.** A previewed plan can be saved to the caller's account and
  read back, reusing R-0041's storage pattern rather than inventing a second one.
- **AC7. Auth + ownership.** Every route requires a JWT (401); another user's
  saved plan is 404, never 403.
- **AC8. Tests.** Preview per scheme with a known e1RM asserting concrete loads;
  each `PlanError` → its token; the magnitude caps; the `null`-load path; auth;
  ownership; and that preview persists nothing.

## 4. Constraints & non-goals

- **Out of scope:** the plan-builder UI; assigning plans to other people (needs
  R-0040 roles); progression *between* mesocycles; merging a generated plan with
  an authored one; any ML-driven scheme selection.
- No changes to R-0038's engine math — this is an exposure layer only, plus the
  AC5 bounds if the engines lack them.

## 5. Open questions (deferred to SPEC-0043)

- **OQ-1:** Does a saved generated plan share the `authored_programs` table (they
  are different domain types — `PeriodizedProgram` vs `AuthoredProgram`) or get
  its own? (Lean: its own table; a shared table with a discriminator would make
  both round-trips lossy.)
- **OQ-2:** Should `POST /plans/preview` accept an explicit e1RM override so a
  user can explore "what if I could squat 150?" (Lean: yes, optional — it is
  read-only and makes the feature useful before you have logged data.)
- **OQ-3:** What are the concrete AC5 caps, and do they belong in `core` beside
  R-0039's `MAX_SETS_PER_LINE` / `MAX_TOTAL_SETS`? (Lean: yes — one place for
  every magnitude bound.)

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-04 | Preview is stateless and separate from saving | Comparing three schemes should not litter the account with rows the user never chose. |
| 2026-08-04 | Reuse `current_e1rm`, not a second derivation | R-0041 established one e1RM definition; a second would let `/plans` and `/training-summary` disagree. |
| 2026-08-04 | Magnitude caps are an acceptance criterion, not an afterthought | R-0041 shipped an unbounded `sets` expansion that a ~330-byte request could turn into a process abort. Same exposure shape here. |

## Changelog

- _2026-08-04 — created (Draft)._
