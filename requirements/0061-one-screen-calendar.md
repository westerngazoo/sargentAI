# R-0061 — One screen, built around a calendar

- **Status:** Accepted (owner, 2026-10-22: all five open questions answered)
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
  - **Supersedes** R-0011 (dashboard, Backlog); see OQ-5.
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
    - **planned:** a training day of the plan projected onto today or
      later, labelled with its name (for example "Pierna");
    - **rest:** a rest day of the plan projected onto today or later;
    - **nothing:** a past day with nothing logged, or any day for a user
      with no program.
  - There is no "missed" state. A training day that is not logged is not
    lost: the plan moves forward (AC6).
  - On a narrow window (phone) it shows **the current week**, and one tap
    opens the full month. On a wide window it shows the month.
  - The user can move to earlier and later weeks or months.
- **AC5. The selected day.** Selecting a day shows its session: the planned
  exercises, or what was logged. It shows **one primary action**:
  - **Empezar sesión**, today. It starts the next pending training day of
    the plan, or a free session when there is no program or today is a
    rest day.
  - **Registrar tarde**, on a past day with nothing logged.
  - **Ver lo registrado**, on a done day.
  - A future day shows its projected plan and no action.
- **AC6. The plan is a sequence that moves with the user.** Dates do not
  matter in themselves; the training day does (owner, 2026-10-22).
  - A program is an **ordered sequence of days**: training days, each with
    a name and its exercises, and rest days. The sequence starts on the day
    the program is chosen.
  - **Logging** a training day of the plan completes it. That includes
    starting it from *Empezar sesión* or logging it late. A free workout
    does not advance the plan.
  - **The projection.** The next pending training day falls on today, or on
    tomorrow if a plan session was already logged today. The days after it
    follow in sequence order, one per calendar day.
  - **A training day that is not logged is not consumed.** It moves to the
    next day, and the whole projection shifts with it.
  - **Rest days are consumed by time passing,** so a plan never waits on a
    rest day.
  - **The server computes the projection.** For the current program, the
    API returns the logged past and the projected days over a requested
    range. Today no endpoint does this: programs have no start date or
    progress, and authored days are relative indices (`core::authoring`).
    The client does not compute it, so it stays a thin client (M3).
  - A user with no current program gets an empty projection, not an error.
  - The endpoint is owner-scoped like every other `/programs` read. Another
    user's projection is refused with 404, not 403, matching R-0006.
  - SPEC-0061 settles the edge cases:
    - a plan session started on a projected rest day;
    - two plan sessions logged on one day;
    - a late log dated before a later plan log;
    - how archetype programs, which today carry only a split description,
      become a day sequence.

### What sits on the main screen

- **AC7. Few controls.** Besides the calendar and the selected day, the main
  screen has only these:
  - **+ Comida**, which opens meal logging;
  - a **floating microphone**, which opens a dictation sheet and nothing
    more (OQ-4). SPEC-0061 places the rest of today's voice hub (action
    ring, presets, anatomy chart) in panels or drops it, and says which;
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
  - **The app follows the device locale.** It falls back to Spanish when the
    device language has no catalogue (OQ-1).
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
  2. The plan sequence and its projection (AC6, and the planned and rest
     states in AC4).
  3. The panels (AC8).
  4. The physics strip, when R-0059 lands (AC9).
  5. Focus mode (AC10).

  The requirement is Met only when every slice is merged and QA has
  verified every criterion.

## 5. Open questions

All answered by the owner on 2026-10-22:

- **OQ-1. Locale rule.** *Answer:* follow the device and fall back to
  Spanish (AC11).
- **OQ-2. The program's start date.** *Answer:* the day the program is
  chosen (AC6).
- **OQ-3. A day that is not logged.** *Answer:* the plan is a template of
  training days. A training day that is not logged moves forward and the
  plan shifts with it. The dates do not matter in themselves; the training
  day does (AC4, AC5, AC6). This replaces the recommendation to keep the
  plan on fixed dates.
- **OQ-4. The voice hub.** *Answer:* the floating microphone opens a
  dictation sheet only (AC7).
- **OQ-5. R-0011 (dashboard).** *Answer:* superseded by R-0061. Its
  "current program, weekly plan" scope is covered here, and its trends live
  in the Progress panel (AC8).

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-22 | **One screen around a calendar** replaces the separate-screen navigation, as in the interactive mockup. | Owner, after the first beta session: the app was unusable and "I'd like it to be a one-screen app". |
| 2026-10-22 | **Phone adjustments:** the week first with the month on a tap, a compact expandable physics strip, and the session in progress as a full-screen focus mode. | Proposed by Claude and accepted by the owner. The full month pushed the day's session below the fold, and a small panel is too cramped for earbud training. |
| 2026-10-22 | **The beta is in Spanish, and the app is ready for English.** | Owner. The audience is @fisicobuenfisico's Spanish-speaking following. |
| 2026-10-22 | **The app follows the device locale, with Spanish as the fallback.** | Owner (OQ-1). |
| 2026-10-22 | **The plan starts on the day the program is chosen.** | Owner (OQ-2). |
| 2026-10-22 | **The plan is a sequence of training days that shifts when a day is not logged.** There is no "missed" state. | Owner (OQ-3): "the dates are not important, the training day is", for example leg day. A fixed-date plan would silently skip a leg day the user never did. |
| 2026-10-22 | **The microphone opens a dictation sheet only.** | Owner (OQ-4). |
| 2026-10-22 | **R-0011 is superseded by R-0061.** | Owner (OQ-5). |

## Changelog

- 2026-10-22 — created (Draft), after the owner accepted the mockup direction.
- 2026-10-22 — the owner answered all five open questions. AC4, AC5 and AC6
  were rewritten for the shifting plan, and AC7 and AC11 take the answers.
  Status → Accepted.
