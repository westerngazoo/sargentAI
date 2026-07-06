# R-0036 — Smart Missing-Log Reminders

- **Status:** Accepted
- **Milestone:** M9 (Voice Assistant & Automation)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-07-06
- **Depends on:** R-0032 (Voice logging — the entry the reminder drives into),
                  R-0004 (Workout log), R-0005 (Nutrition log),
                  R-0014 (UserProgram — source of the expected routine)
- **Realized by:** SPEC-0036 (to be written)
- **QA:** `qa` agent run scoped to this requirement

---

> **Origin (R-0057, 2026-07-06).** Split out of R-0032, whose original AC6–AC8
> described a proactive reminder system that was never built. PR #37's spec
> included a §2.3 cron + notification sketch that can seed the design here.

## 1. Statement

A smart alert system proactively reminds the user to log missing meals or
workouts based on their daily routine, so tracking stays consistent and the ML
model gets complete data. Tapping a reminder opens the app straight into
voice-logging mode.

## 2. Rationale

Even with fast voice logging (R-0032), users forget. Proactive, routine-aware
nudges — and a one-tap path from the notification into voice logging — close the
compliance gap that hurts data quality and retention.

## 3. Acceptance criteria

- **AC1. Routine model.** The system derives the user's expected daily routine
  (expected meal windows; scheduled workout days from the active `UserProgram`),
  either inferred from past logs or explicitly configured.
- **AC2. Scheduled evaluation.** A scheduled job (cron/worker) evaluates, per
  user, whether expected meals/workouts have been logged within a configurable
  grace period.
- **AC3. Notifications.** When an expected meal or workout is not logged within
  the grace period, the app sends a local or push notification reminding the
  user.
- **AC4. Voice-activated from notification.** Tapping the reminder opens the app
  directly into voice-listening mode (deep link into the R-0032 flow).
- **AC5. Quiet hours / rate limiting.** Reminders respect quiet hours and are
  rate-limited so a user is not spammed (no more than a sensible cap per day).
- **AC6. Opt-out.** The user can disable reminders (globally and/or per
  category) from settings.
- **AC7. Tests.** Backend tests cover the routine evaluation and grace-period
  logic (logged vs. missing → reminder or not); client tests cover the
  notification-tap → voice-mode deep link.
- **AC8. Privacy / scope guard.** Reminders use only the user's own routine and
  log data; no new biometric collection.

## 4. Constraints & non-goals

- Not always-on listening — the notification only *opens* voice mode; the user
  still initiates the mic.
- Not a general notification framework — scoped to missing-log nudges.
- Delivery mechanism (local notifications vs. push/FCM) to be settled in
  SPEC-0036.

## 5. Open questions (deferred to SPEC-0036)

- **OQ-1:** Routine inferred from past logs vs. explicitly configured meal times
  (or both)?
- **OQ-2:** Local notifications only, or push/FCM for server-triggered reminders
  when the app is closed?
- **OQ-3:** Where does the scheduled evaluation run (backend cron/worker vs.
  on-device scheduling)?
- **OQ-4:** Grace-period and quiet-hours defaults.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-06 | Split reminders out of R-0032 into R-0036 | The logging half shipped; reminders never did. A separate requirement lets each be tracked honestly. |

## Changelog

- _2026-07-06 — created and **Accepted** (R-0057); carries the former R-0032 AC6–AC8._
