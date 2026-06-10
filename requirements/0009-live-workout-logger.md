# R-0009 — Live workout logger

- **Status:** Accepted
- **Milestone:** M3 (differentiator fast-track, position 1)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-10
- **Depends on:** R-0004 (Done — `/workouts` CRUD), R-0007 (Done — app shell), R-0008 (Done — shell idioms: `AsyncValue`, shared `ApiException.fromDio`, failure-as-state controllers)
- **Realized by:** SPEC-0009 (to be written)
- **QA:** `qa` agent run scoped to this requirement

## 1. Statement

A signed-in user can **log a workout live, in the gym**: start a session from
the home shell, add exercises as they train (picked from a small preset list of
common lifts or typed free-text, with an optional muscle group), record each
set's reps — and optionally weight and RPE — and **finish** the session, which
persists it via `POST /workouts`. The home shell also lists the user's
**recent sessions** (newest first) and allows **deleting** one; editing a saved
session is deferred to a later requirement.

The in-progress session lives in an application-layer **session driver** — a
state machine that always knows the current exercise, the current set, and the
session's running content. This is a deliberate architectural requirement: the
**R-0027 earbud-guided mode** (the product's differentiator) will drive the
same machine by voice, so the driver must be consumable by a non-UI client.
In this requirement the driver is exercised only by screens; voice arrives in
R-0027, and program-awareness (a planned routine to follow) arrives with
R-0014.

## 2. Rationale

The workout log is the primary signal the M5 response model consumes, and the
backend for it (R-0004) has been Done for weeks with **no human way to feed
it**. This requirement closes that gap with the in-gym surface the owner chose
("live logger first") — and it is the substrate of the earbud differentiator:
get the session machinery right here, and R-0027 only adds a voice transport.

## 3. Acceptance criteria

- **AC1.** The home shell offers a **Start workout** action. Starting opens the
  live session screen with `performed_on` = today; the user never types a date
  (backdated entry belongs to the deferred session editor).
- **AC2.** The user can **add an exercise** by either picking from a built-in
  **preset list of common lifts** or typing a **free-text name** (trimmed,
  non-empty, ≤ 100 chars — the R-0004 cap), and may optionally tag one of the
  six backend muscle groups (`chest`/`back`/`shoulders`/`arms`/`legs`/`core`).
  The preset list is **presentation-only** (client-side constant): it
  pre-fills the same free-text name field and is explicitly slated for
  replacement by the M4 archetype/exercise library.
- **AC3.** Within an exercise the user can **log sets**: `reps` required
  (integer, [1, 10 000]); `weight_kg` optional ((0, 1000], fractional allowed);
  `rpe` optional ([6.0, 10.0] in 0.5 steps). Client-side validation mirrors the
  backend exactly; invalid input blocks adding the set with a readable inline
  message. A quick **"repeat last set"** affordance pre-fills the previous
  set's values.
- **AC4.** The **session driver** tracks the in-progress session as explicit
  state: ordered exercises, each with ordered sets, plus the *current* exercise
  position. The user can switch back to an earlier exercise to add more sets.
  The draft **survives navigation within the app** for the whole session
  (in-memory; surviving app kill is out of scope).
- **AC5.** **Finish** is enabled only when the session is valid per the backend
  rules: ≥ 1 exercise and every exercise has ≥ 1 set. Finishing issues
  `POST /workouts` with exactly the logged content (omitted optionals are
  omitted, not null); on `201` the user lands on the home shell with the
  session visible in the recent list and the in-progress draft cleared.
- **AC6.** A **failed finish** loses nothing: a backend `400 {field}` keeps the
  session intact and shows a readable, field-aware message; a network/timeout
  error shows a retryable message (retry re-submits the same session); a `401`
  clears the session token and routes to login (R-0007 behaviour). Failure is
  data on the driver's state — no raw exception reaches the UI.
- **AC7.** **Abandoning** an in-progress session (leaving the flow) asks for
  confirmation; confirming discards the draft; nothing is persisted.
- **AC8.** The home shell shows the user's **recent sessions** (from
  `GET /workouts`, newest `performed_on` first): date, exercise count, set
  count. Empty state shows a friendly "no workouts yet" with the Start action.
  The list and the R-0008 profile prompt coexist.
- **AC9.** The user can **delete** a session from the list after a
  confirmation; `DELETE /workouts/:id` → the list refreshes; deleting a foreign
  or missing id surfaces the backend's 404 as a readable message. **No editing
  UI** — full-replace editing is a deferred requirement.
- **AC10.** Architecture: the session driver is a Riverpod `Notifier` exposing
  a typed API (`start`, `addExercise`, `logSet`, `selectExercise`, `finish`,
  `abandon`) **independent of widgets** — no widget holds session business
  logic; screens render driver state and call the API. This is the seam R-0027
  consumes. The shared `ApiException.fromDio` and the failure-as-state pattern
  (R-0008) are reused.
- **AC11.** **Tests:** unit tests cover the driver's state machine (start /
  add / log / switch / finish-validity / abandon) and the set/exercise
  validators at every backend boundary; widget tests cover the live screen,
  the finish success path (list shows the session), each AC6 failure branch,
  the abandon confirm, and list + delete. Gates green: `flutter analyze`,
  `dart format --set-exit-if-changed .`, `flutter test`.
- **AC12.** **No backend changes** — only the existing R-0004 endpoints are
  called.

## 4. Constraints & non-goals

- **No editing of saved sessions** (`PUT /workouts/:id` UI) — deferred to its
  own requirement; the live flow plus delete covers the MVP.
- **No program/plan awareness yet** — the driver tracks the session being
  built; following a planned routine (next exercise prescription) arrives with
  R-0014, voice with R-0027.
- **No voice/earbud interaction in this R** — but the driver's API is designed
  for it (AC10).
- **No rest timers, supersets, exercise reordering, or notes** — deferred.
- **No offline queue / draft persistence across app restarts** — in-memory
  draft only (online-only MVP, per `project-specifics.md`).
- **No pagination on the sessions list** — matches the R-0004 list endpoint.
- **Metric units only** (kg), matching the backend.

## 5. Open questions

Settled in the step-1 discussion (owner, 2026-06-10):

- **OQ1 — Entry shape?** RESOLVED → **live in-gym logger first** (this R);
  after-the-fact entry/editing deferred. (AC1, §4)
- **OQ2 — Exercise names?** RESOLVED → **preset picker + free-text escape**;
  preset list is client-side and presentation-only until the M4 library. (AC2)

Deferred to the SPEC-0009 design discussion (HOW): the exact preset list
contents, the driver's state shape, screen composition (single screen vs
per-exercise pages), and how "repeat last set" pre-fill interacts with
validation.

## 6. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-10 | **Live in-gym logger first; session editing deferred.** | Owner decision; the live flow is the substrate the R-0027 earbud mode drives — the differentiator path (fast-track). |
| 2026-06-10 | **Preset picker + free-text escape; list is presentation-only.** | Owner decision; no exercise-library backend exists (M4). The preset pre-fills the same validated free-text path, so no schema is invented client-side. |
| 2026-06-10 | **Session driver as a widget-independent state machine (AC10).** | R-0027 must drive the same session by voice; designing the seam now avoids rebuilding the logger later. |
| 2026-06-10 | **`performed_on` = today, no date input in the live flow.** | A live session is today by definition; backdating belongs to the deferred editor. |
| 2026-06-10 | **Finish gated on the backend's ≥1-exercise / ≥1-set rules.** | Mirrors `WorkoutError::{ExercisesEmpty,SetsEmpty}`; the client never submits a session the backend will reject structurally. |

## Changelog

- _2026-06-10 — created (Draft). First fast-track requirement: the live workout logger as a program-aware-ready session driver. Two step-1 forks already owner-resolved (live-first; preset+free-text)._
- _2026-06-10 — **Accepted.** Owner accepted AC1–AC12. Next: step 2 — SPEC-0009 and the architect design review._
