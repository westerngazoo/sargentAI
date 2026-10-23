# R-0061 — One screen, built around a calendar

- **Status:** Draft
- **Milestone:** M3 (Flutter client). Pulled forward by the beta (owner, 2026-10-22).
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-10-22
- **Depends on:**
  - R-0007 (app shell, Done) and R-0008 (onboarding, Done)
  - R-0009 (live workout logger, Done)
  - R-0014 and R-0041 (current program, Done)
  - R-0035 (earbud hands-free training, Done)
- **Related:**
  - R-0059 (the squat, animated in the app, Draft) is shown inside this screen.
  - R-0011 (dashboard, Backlog) overlaps; see OQ-5.
- **Realized by:** SPEC-0061 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

After sign-in and onboarding, the app is **one screen with a calendar at its
centre**. Each day on the calendar shows what was planned and what was done.
Selecting a day shows that day's session with **one main action**, which
depends on the day's state.

Everything else opens **on top of that screen** and closes back to it:
- the program, measurements and account panels;
- meal logging and voice dictation;
- the session in progress, which takes the full screen.

No screen is a dead end.

The interface is in **Spanish**, and the app is ready for English. The
visual style moves from generic green to a **"physics" style**: lab paper,
blueprint ink, and orange for forces.

The direction was agreed on the interactive mockup of 2026-10-22, with the
owner's adjustments for phones. The mockup is a reference, not a pixel spec.

## 2. Rationale

- **The app is unusable for beta testers today.** It grew screen by screen.
  Navigation uses `context.go`, which *replaces* the screen. So:
  - Program choice (`/programs/get`), progress and the voice hub have no
    reliable way back.
  - The home screen is a list of cards with five equally weighted buttons.
  - The owner got stuck in the first beta session (2026-10-22). #155 fixed
    onboarding on desktop, but the shape of the app is the problem.
- **The calendar is how a user already thinks:** "what is on today, and what
  did I do?" The product is a program with planned days, so the calendar is
  that program laid out over time.
- **Physics is the differentiator** (owner direction, 2026-09-06), but it is
  missing from the app. This screen gives it a permanent place.
- **The beta audience speaks Spanish.** @fisicobuenfisico publishes in
  Spanish, and every user-visible string is in English today. There is no
  localization at all.

## 3. Acceptance criteria

### Navigation

- **AC1. One main screen.** After sign-in and onboarding, every
  authenticated capability is reached from the main screen.
  - Nothing that opens from it replaces it without a way back.
  - Every panel or sheet closes with a visible ✕, with Esc on desktop and
    with the system back gesture on Android.
  - A panel or sheet also closes on a tap outside it.
  - Closing returns to the main screen with the selected day kept.
- **AC2. Nothing is lost.** Every capability reachable today is still
  reachable from the main screen:
  - start a workout;
  - log a meal;
  - voice dictation;
  - progress;
  - view and change the current program;
  - measurements;
  - onboarding, for a user without a profile;
  - sign out.
- **AC3. The screens before sign-in keep their routes:** login, register,
  forgot password, reset password and onboarding. #155's onboarding layout
  is kept.

### The calendar and the day

- **AC4. The calendar.**
  - Today is marked.
  - Each day shows exactly one state:
    - **done:** a workout is logged that day;
    - **planned:** the current program has a session that day and the day
      is today or later;
    - **missed:** a planned session in the past with nothing logged;
    - **nothing:** no session planned and nothing logged.
  - On a narrow window (phone) it shows **the current week**, and one tap
    opens the full month. On a wide window it shows the month.
  - The user can move to earlier and later weeks or months.
- **AC5. The selected day.** Selecting a day shows its session: the planned
  exercises, or what was logged. It shows **one primary action**:
  - **Empezar sesión**: today, planned or free.
  - **Registrar tarde**: a missed past day.
  - **Ver lo registrado**: a done day.
  - A future day shows the plan and no action.
- **AC6. Planned sessions by date come from the server.** For the user's
  current program, the API answers which session falls on which date over a
  requested range. Today no endpoint does this: programs have no start date,
  and authored days are relative indices (`core::authoring`).
  - The client does not compute the schedule. It stays a thin client (M3).
  - A user with no current program gets an empty schedule, not an error.
  - The endpoint is owner-scoped like every other `/programs` read. Another
    user's schedule is refused with 404, not 403, matching R-0006.

### What sits on the main screen

- **AC7. Few controls.** Besides the calendar and the selected day, the main
  screen has only these:
  - **+ Comida**, which opens meal logging;
  - a **floating microphone**, which opens dictation;
  - a **profile menu** at the top right, which opens the panels in AC8 and
    signs out.
- **AC8. Panels over the screen.** Program, Measurements, Account and
  Progress open as panels over the main screen, under AC1's closing rules.
  - The Program panel shows the current program and lets the user choose a
    different one.
  - The whole choose flow happens inside the panel. Today's `/programs/get`
    is where the owner got trapped.
- **AC9. Physics strip.** The main screen has a place for R-0059's squat.
  - On a wide window it sits beside the calendar.
  - On a phone it is a compact strip with a small figure and one key figure,
    and it expands on a tap.
  - Until R-0059 is Met, the strip is absent. There is no placeholder.
- **AC10. Focus mode.** A session in progress takes the full screen.
  - The R-0035 earbud flow and voice dictation work in it as they do today.
  - Finishing the session returns to the main screen, with that day shown
    as done.
  - Leaving with unsaved sets asks for confirmation first.

### Language and style

- **AC11. Spanish, ready for English.**
  - Every user-visible string goes through Flutter's localization, with no
    literal UI strings in widgets.
  - A test fails if a widget under `lib/src` holds a literal UI string.
  - The Spanish catalogue is complete. The English catalogue is also
    complete, written from today's strings.
  - Dates, weekdays and numbers are formatted for the locale.
  - The locale rule is OQ-1.
- **AC12. The physics style.**
  - Light and dark themes: lab paper with a fine grid, blueprint-blue ink,
    orange kept for forces and highlights, and instrument-style numerals.
  - It replaces the green theme everywhere, including the screens before
    sign-in.
  - Text meets WCAG 2.1 AA contrast (4.5:1 body, 3:1 large text) in both
    themes. This is checked by a test over the theme's colour pairs.
- **AC13. Phone and desktop.**
  - At 390×844 and at 1440×900 the main screen, every panel and focus mode
    show no horizontal scroll and no clipped control.
  - The primary action of the selected day is visible without scrolling.
  - Widget tests check both sizes, as #155 did.

## 4. Constraints & non-goals

- **Not new product features.** This changes how existing capabilities are
  reached and shown. The only new backend capability is AC6.
- **The content of the physics strip belongs to R-0059.** This requirement
  only reserves and lays out its place.
- **No offline mode, no notifications.** Reminders are R-0036.
- **No trainer views.** Those are M-Platform.
- **Dependencies.** Localization uses the SDK's `flutter_localizations` with
  `gen-l10n` and `intl`, the first-party route. A third-party i18n package
  was rejected as unnecessary. Any font added for the instrument numerals
  must be under the OFL or a similar licence, and SPEC-0061 names it.
- **Delivery in slices.** Each slice is its own PR, testable in the beta:
  1. The main screen and calendar with logged history (AC1–AC5, without
     planned days; AC7; AC11; AC12; AC13).
  2. Planned sessions (AC6, and planned/missed in AC4).
  3. The panels (AC8).
  4. The physics strip, when R-0059 lands (AC9).
  5. Focus mode (AC10).

  The requirement is Met only when every slice is merged and QA has
  verified every criterion.

## 5. Open questions

- **OQ-1. Locale rule.** Should the app follow the device locale, falling
  back to Spanish? Or should it be Spanish always during the beta, with
  English selectable in Account? *Recommendation:* follow the device and
  fall back to Spanish.
- **OQ-2. The program's start date.** AC6 needs one. Is it the day the
  program is chosen, or does the user pick it when choosing? *Recommendation:*
  the day it is chosen, editable in the Program panel.
- **OQ-3. A missed day.** Does the plan stay on its dates, so the day is
  simply missed? Or does the rest of the program shift one day later?
  *Recommendation:* the plan stays on its dates for v1. Shifting is a
  periodization decision (R-0038), not a navigation one.
- **OQ-4. The voice hub.** Today `/hub` is a full screen with an action ring,
  presets and an anatomy chart. Should the floating microphone open a
  dictation sheet only, with the rest of the hub moved into panels? Or
  should it open the hub as a full-screen panel? *Recommendation:* a
  dictation sheet. SPEC-0061 places the rest of the hub.
- **OQ-5. R-0011 (dashboard).** Its "current program, weekly plan" scope is
  covered here. Should R-0011 shrink to trends only, or be superseded
  outright?

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-22 | **One screen around a calendar** replaces the separate-screen navigation, as in the interactive mockup. | Owner, after the first beta session: the app was unusable and "I'd like it to be a one-screen app". |
| 2026-10-22 | **Phone adjustments:** the week first with the month on a tap, a compact expandable physics strip, and the session in progress as a full-screen focus mode. | Proposed by Claude and accepted by the owner. The full month pushed the day's session below the fold, and a small panel is too cramped for earbud training. |
| 2026-10-22 | **The beta is in Spanish, and the app is ready for English.** | Owner. The audience is @fisicobuenfisico's Spanish-speaking following. |

## Changelog

- 2026-10-22 — created (Draft), after the owner accepted the mockup direction.
