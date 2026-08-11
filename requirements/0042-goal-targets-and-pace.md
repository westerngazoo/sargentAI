# R-0042 — Goal Targets & Pace Tracking

- **Status:** Draft
- **Milestone:** M5 (adaptive intelligence) — the measurement half
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-08-04
- **Depends on:** R-0015 (per-lift e1RM trend, body trend, adherence — the
                  measured signal), R-0004 (workout log), the body-measurements log (shipped in PR #52;
                  register id unassigned — flagged in #91)
- **Realized by:** SPEC-0042 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

## 1. Statement

Let a user set an explicit, dated **target** and tell them whether their actual
training is on pace to reach it.

Today the app knows a user's *direction* (`Goal::BuildMuscle`, `GainStrength`)
and can compute *trends* — "Squat trending +5.8 kg/week over 8 sessions" — but
nothing states *"squat 140 kg by 1 December"*, so nothing can answer the question
that actually matters: **am I going to make it?**

Three target kinds, all measured against signal R-0015 already produces:

- **Strength** — a per-lift e1RM target (`Squat → 140 kg`)
- **Body composition** — a bodyweight and/or body-fat target
- **Consistency** — sessions per week sustained over the window

## 2. Rationale

This is the missing half of M5. R-0015 measures, R-0017 suggests — but neither
knows what the user is *aiming at*, so neither can say whether the plan is
working. A trend without a target is trivia; a target without a trend is a wish.

It is also the honest foundation for anything adaptive later: a program cannot
sensibly auto-adjust toward a goal that was never stated.

## 3. Acceptance criteria

- **AC1. Set a target.** A user can create a goal with a kind, a target value, a
  target date, and (for strength) the lift it applies to. The **baseline** — the
  measured value at creation — is captured server-side, not supplied by the
  client, so pace is computed against what was actually true when the goal was
  set.
- **AC2. Three kinds, one shape.** `Strength { lift, target_e1rm_kg }`,
  `Body { target_weight_kg?, target_body_fat_pct? }`, `Consistency
  { sessions_per_week }`. At least one field must be set for `Body`.
- **AC3. Own goals only.** Goals are per-user; another user's goal is **404**,
  never 403 (consistent with R-0041 — ids are not enumerable).
- **AC4. Pace status.** For each active goal the API reports a typed status:
  `Ahead`, `OnTrack`, `Behind`, `AtRisk`, or `Achieved`, plus the numbers behind
  it — baseline, current, target, required pace, observed pace, and the value
  projected at the target date.
- **AC5. Honest about uncertainty.** When there is not enough data to establish a
  trend, the status is an explicit `InsufficientData` with the reason — **never**
  a fabricated projection. Same principle as a lift with no e1RM yielding a
  `null` load rather than a guess.
- **AC6. Deadlines are handled.** A goal whose target date has passed reports a
  terminal status (`Achieved` or `Missed`) and stops projecting.
- **AC7. Direction-aware.** A target may be *below* baseline (lose fat, cut
  weight). "Behind" means the wrong side of the required trajectory, not simply
  "smaller than target".
- **AC8. Read-only.** Computing status writes nothing and changes no program.
  This requirement does not adjust training (owner decision — status only).
- **AC9. Pure core.** Pace math lives in `fitai-core` as pure functions over
  already-aggregated inputs: deterministic, no I/O, no clock parameter beyond an
  explicit `today`, fully unit-testable without a database.
- **AC10. Tests.** Per kind: on-pace, behind, ahead, achieved-early, missed,
  insufficient-data, and a decreasing-target (fat loss) case. Plus ownership
  (404), auth (401), and typed rejection of an invalid goal (target date in the
  past at creation, non-finite / non-positive / absurd-magnitude target,
  blank or over-length lift name). *Amended 2026-08-08:* "unknown lift" was
  originally listed here but is unenforceable — lifts are free-text exercise
  names with no registry — so blank/over-length is the enforceable proxy and
  a goal for a not-yet-trained lift is a supported flow (SPEC-0042 §2.2.3).
- **AC12. Bounded goal count.** *(Added 2026-08-09 by code-review amendment,
  owner-approved.)* One user may hold at most 25 goals; the 26th create is
  rejected with the fixed token `too_many_goals`. Every stored goal costs an
  assessment on every list read and every expired goal its own aggregation
  anchor, so an unbounded count is a self-inflicted denial of service — the
  same magnitude-bound lesson as R-0041.
- **AC11. Abandon a goal.** *(Added 2026-08-08 by amendment.)*
  `DELETE` removes the caller's goal (204; non-owner 404). Without it, the
  "no update — delete and recreate" mutation story is impossible and dead
  `Missed` rows accumulate forever. This is user data hygiene, not status
  mutation; AC8's read-only rule governs status computation and is untouched.

## 4. Constraints & non-goals

- **Explicitly out of scope:**
  - **Auto-adjusting the program** when behind (owner decision — status only for
    v1). Routing pace into R-0017's adjustment engine is a later requirement.
  - Notifications / reminders when a goal slips (R-0036 territory).
  - Multi-user or trainer-assigned goals (needs R-0040 roles).
  - Goal history, streaks, or gamification.
  - Predicting *when* a target will be hit if the date is left open — v1 goals
    are dated.
- Pace is computed from the **existing** R-0015 aggregates. No second trend
  implementation, no new statistics.

## 5. Open questions (deferred to SPEC-0042)

- **OQ-1:** Is required pace linear from baseline→target, or should strength use
  a decelerating curve (gains slow as you approach a ceiling)? (Lean: linear in
  v1, documented as a simplification — a wrong curve is worse than an honest
  straight line.)
- **OQ-2:** What tolerance separates `OnTrack` from `Behind`? A fixed percentage
  of required pace, or a function of observed variance? (Lean: a fixed band,
  stated in the spec, since a variance-based band needs data we may not have.)
- **OQ-3:** How many sessions / measurements constitute "enough data" for AC5,
  and does it differ per kind? (Body weight is noisier than e1RM.)
- **OQ-4:** Can a user hold several active goals at once, and may two goals
  conflict (gain strength *and* lose weight)? (Lean: allow both; surface the
  tension rather than forbidding it — it is a real training situation.)

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-08-04 | All three kinds in v1 | Owner choice — strength, body composition and consistency are the three things users actually set, and they share one status shape. |
| 2026-08-04 | Status only; no auto-adjustment | Owner choice — a projection that silently rewrites training compounds its own error. Measure first, act later. |
| 2026-08-04 | Baseline captured server-side at creation | A client-supplied baseline is trivially gamed and makes pace meaningless. |
| 2026-08-04 | `InsufficientData` is a first-class status | Consistent with R-0041's `null` load: the system says "I don't know" rather than inventing a number. |

## Changelog

- _2026-08-04 — created (Draft)._
- _2026-08-09 — AC12 (goal cap, owner-approved: 25) added from the
  implementation code review._
- _2026-08-08 — amended with SPEC-0042's architect review: AC10 unknown-lift
  rejection replaced by the enforceable proxy; AC11 (delete) added; the
  dangling R-0034 dependency reference corrected (id never existed — #91)._
