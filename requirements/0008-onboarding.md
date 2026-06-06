# R-0008 — Onboarding flow

- **Status:** Accepted
- **Milestone:** M3
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-04
- **Depends on:** R-0007 (Done — Flutter app shell: auth, router, Riverpod, Dio), R-0003 (Done — `GET`/`PUT /profile/me`)
- **Realized by:** SPEC-0008 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

A signed-in user who has **not yet completed their profile** is offered, from the
home shell, a **dismissible prompt** to do so. Acting on it opens a **multi-step
onboarding wizard** that collects the user's **body stats and training goals** and
saves them to the backend via `PUT /profile/me`; on success the user returns to
home, which now reflects the saved profile and no longer shows the prompt.

Profile absence is detected with the existing R-0003 contract: `GET /profile/me`
returns `404` when no profile exists. The prompt is **optional** — the user may
dismiss it and continue using the app — so onboarding never blocks the shell.
This is the **first M3 feature screen**, built entirely on the R-0007 shell
(router, Riverpod session, Dio client) and the R-0003 profile endpoints; it adds
**no backend changes**.

## 2. Rationale

The ML engine (M5) and the archetype prior (M4) need each user's body stats and
goals to personalize a program. R-0003 made the backend able to store them and
R-0007 gave the app an authenticated shell, but **there is still no in-app way to
enter a profile**. Onboarding closes that gap with a guided first-run flow, while
staying optional so a user who just wants to look around is never trapped behind
it. Keeping it to the fields the backend already supports (deferring training
history, which has no profile column yet) keeps this first feature screen thin
and shippable.

## 3. Acceptance criteria

- **AC1.** On the home shell, the app checks profile existence via
  `GET /profile/me`. If it returns **`404`** (no profile), a **"complete your
  profile" prompt** is shown; if a profile exists (`200`), no prompt is shown.
- **AC2.** The prompt is **dismissible** — dismissing it returns the user to a
  normal home shell for the session without saving anything; onboarding never
  blocks access to home.
- **AC3.** Acting on the prompt opens a **multi-step wizard** with at least:
  (1) **body stats**, (2) **goals**, (3) an **optional details** step. The wizard
  shows progress and supports **back/next**; data entered in earlier steps
  **persists** while navigating within the flow.
- **AC4.** The **body-stats** step collects **date of birth**, **height (cm)**,
  and **weight (kg)**, all required, with client-side validation **mirroring the
  backend**: age ∈ [13, 120], height ∈ [50, 300] cm, weight ∈ [20.0, 500.0] kg.
  Invalid or empty values block advancing and show a readable inline message.
- **AC5.** The **goals** step lets the user select **one or more** goals from
  `lose_fat`, `build_muscle`, `recomp`, `maintain`, `gain_strength`; **at least
  one** is required to finish.
- **AC6.** The **optional-details** step collects **sex** and **body-fat % ∈
  [1.0, 75.0]**; both may be **skipped** (left empty), and skipping is not an
  error.
- **AC7.** Completing the wizard calls **`PUT /profile/me`** with the collected
  fields (omitting skipped optionals). On `200` the app returns to **home**,
  which now shows the profile and **no prompt** (AC1 re-evaluated).
- **AC8.** A **failed save** keeps the user in the wizard with their entered data
  intact (no loss): a backend **`400`** maps the offending field to a readable
  inline message; a **network/timeout** error shows a retryable message; a
  **`401`** clears the session and routes to login (re-login, per R-0007 AC5).
- **AC9.** The flow uses the **R-0007 foundation** — Riverpod for wizard state,
  the shared Dio client (Bearer + 401 sink), and the existing router — and
  introduces **no on-device business logic** beyond presentation and call
  orchestration (thin client).
- **AC10.** **Tests:** widget tests cover the home prompt (shown on `404`, hidden
  on `200`, dismissible), each wizard step's validation, and the save flow
  (success → home; `400`/network/`401` handling) against a mocked HTTP layer. The
  mobile gates are green: `flutter analyze`, `dart format --set-exit-if-changed .`,
  `flutter test` (the `test/` unit + widget suite).
- **AC11.** **No backend changes** — uses the existing `GET`/`PUT /profile/me`.

## 4. Constraints & non-goals

- **No training-history capture** — the profile backend has no such field;
  deferred to its own requirement (backend + UI) when it lands.
- **No profile editing / settings screen** — onboarding is first-run capture
  (an authenticated upsert); a later "edit profile" surface is a separate R.
- **No mandatory/blocking onboarding** — the prompt is dismissible (owner
  decision); a future R may add gentle re-prompting.
- **No new profile fields** beyond R-0003's contract; **metric units only**;
  no imperial conversion.
- **No backend changes, no new endpoints** — `GET`/`PUT /profile/me` only.
- **No progress-photo / workout / nutrition entry** — those are R-0009+.

## 5. Open questions

Settled in the step-1 discussion (folded into §3/§6); none blocking `Accepted`:

- **OQ1 — Onboarding trigger?** RESOLVED → **optional, dismissible prompt from
  home** gated on `GET /profile/me` `404` (not a blocking gate). (AC1/AC2)
- **OQ2 — Field scope vs. "training history"?** RESOLVED → **profile fields only**
  (DOB, height, weight, goals; optional sex & body-fat); training history
  deferred (no backend field). (AC4–AC6, §4)
- **OQ3 — Flow shape?** RESOLVED → **multi-step wizard** (body stats → goals →
  optional details) with progress + back/next. (AC3)

Deferred to the SPEC-0008 design discussion (HOW, not WHAT): the exact step
count and grouping, how profile-existence is cached/observed (a Riverpod
provider over `GET /profile/me` vs. a one-shot check), date-of-birth input
widget, and whether a `Sex` value set beyond the backend's enum is needed.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-04 | **Onboarding is an optional, dismissible prompt from home, gated on `GET /profile/me` `404`.** | Guides new users without trapping anyone behind a blocking gate; reuses the existing profile-absence signal. (OQ1) |
| 2026-06-04 | **Scope to the R-0003 profile fields; defer training history.** | The profile backend has no training-history column; adding one is separate backend scope. Keeps the first feature screen thin. (OQ2) |
| 2026-06-04 | **Multi-step wizard (body stats → goals → optional details).** | A guided first-run flow matches "onboarding"; smaller, validated steps over one long form. (OQ3) |
| 2026-06-04 | **Client validation mirrors the backend exactly (age 13–120, height 50–300 cm, weight 20–500 kg, body-fat 1–75%).** | Fail fast in the UI; the backend stays the source of truth and still rejects bad data (AC8). |
| 2026-06-04 | **No backend changes; `GET`/`PUT /profile/me` only.** | R-0003 already provides the upsert + absence signal; M3 is thin-client work. |

## Changelog

- _2026-06-04 — created and **Accepted**. First M3 feature logger: an optional, dismissible onboarding prompt + multi-step wizard over the R-0003 profile endpoints. Three step-1 decisions captured (optional prompt; profile fields only; multi-step wizard)._
