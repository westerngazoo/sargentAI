# SPEC-0009 — Live workout logger

- **Status:** Accepted
- **Realizes:** R-0009
- **Author:** Claude (main session), with owner
- **Created:** 2026-06-10
- **Depends on:** SPEC-0007 (Implemented — shell, router, Dio, Riverpod), SPEC-0008 (Implemented — `ApiException.fromDio` flat-body parser, failure-as-state controller pattern, `AsyncValue` shell idiom), SPEC-0004 (Implemented — the `/workouts` contract)
- **Module(s):** `mobile/lib/src/workout/**` (new feature module), `mobile/lib/src/shell/home_shell.dart` (Start action + sessions list), `mobile/lib/src/router/app_router.dart` (route), `mobile/test/workout/**` + `mobile/test/shell/**` (tests)

## 1. Motivation

Realizes [R-0009](../requirements/0009-live-workout-logger.md): the in-gym live
logger — the first fast-track requirement and the substrate the R-0027
earbud-guided mode drives by voice. The heart of the design is the **session
driver**: a widget-independent state machine owning the in-progress session.
Screens are thin renderers over it; R-0027 will attach a voice transport to the
same API without touching the driver.

## 2. Design

### 2.1 Module shape

```
lib/src/workout/
  domain/
    muscle_group.dart      # MuscleGroup enum ⇄ wire (chest/back/shoulders/arms/legs/core)
    set_draft.dart         # SetDraft + validators (reps/weight/rpe, backend-mirrored)
    exercise_draft.dart    # ExerciseDraft (name, muscleGroup?, sets) + name validator
    session_draft.dart     # SessionDraft (ordered exercises, validity, toRequest)
    workout_session.dart   # WorkoutSession/WorkoutExercise/WorkoutSet (response parse)
  data/
    workout_api.dart       # list() / create(req) / delete(id) over the shared Dio
    workout_repository.dart + workoutRepositoryProvider
  application/
    workouts_provider.dart # FutureProvider<List<WorkoutSession>> (home list)
    session_driver.dart    # SessionDriver (Notifier<SessionDriverState>) — the AC10 seam
    session_list_controller.dart # delete(id) with failure-as-state (architect finding 4)
  presentation/
    preset_exercises.dart      # presentation-only constant list (architect finding 5)
    live_session_screen.dart   # the in-gym screen over the driver
    add_exercise_sheet.dart    # preset chips + free text + muscle group
    set_entry_row.dart         # reps/weight/rpe inputs + repeat-last-set
    session_list.dart          # home: recent sessions + delete confirm
```

### 2.2 The session driver (AC4/AC10 — the R-0027 seam)

```dart
@immutable
class SessionDriverState {
  final SessionDraft? draft;     // null = no session in progress
  final int currentExercise;     // index into draft.exercises (ignored when draft == null)
  final bool submitting;
  final String? error;           // user-safe message (failure-as-state)
  final String? errorField;      // backend 400 field, when given
  final bool done;               // 201 landed; screen ref.listens → go home

  // Derived, reactive (pure functions of the fields above — architect finding 3):
  bool get canFinish;            // delegates to draft?.canFinish ?? false
  SetDraft? get lastSet;         // last set of the CURRENT exercise (OQ-F4)
}

class SessionDriver extends Notifier<SessionDriverState> {
  void start();                                  // today's empty session

  /// The driver is the single validation-enforcement point (architect
  /// finding 1): invalid input is REJECTED here — never appended — and the
  /// reason is returned synchronously (`null` = accepted) so any transport
  /// (screen now, voice in R-0027) can surface/speak it. The same message is
  /// NOT put on `state.error` (that channel is for finish/network failures).
  String? addExercise(String name, {MuscleGroup? group}); // validates name; appends + selects
  String? logSet(SetDraft set);                  // guards on set.valid; appends to current

  void selectExercise(int index);                // clamps: out-of-range is a no-op (finding 7)
  Future<void> finish();                         // POST /workouts; failure-as-state
  void abandon();                                // discard draft (confirm is UI's job)
}
```

- The driver is pure application state: no `BuildContext`, no widgets, no I/O
  except `finish()` via the repository. R-0027's voice driver calls exactly
  this API ("set done" → `logSet(lastSet-prefill)`, "next exercise" →
  `addExercise`/`selectExercise`, "finish workout" → `finish()`); a rejected
  call's returned message is what the voice transport speaks back.
- `finish()` mirrors the R-0008 controller contract: guard on
  `state.canFinish` (no-op otherwise), `submitting` flag,
  `await ref.read(repo).create(...)`, then `ref.invalidate(workoutsProvider)`
  + `await ref.read(workoutsProvider.future)` **before** setting `done` (list
  shows the session on arrival, AC5); on `ApiException` → `error`/`errorField`
  on state, draft untouched (AC6), no rethrow, no widget `try/catch`.
- **Date semantics (architect finding 2):** `finish()` stamps the **local**
  calendar date (`DateTime.now()` → y/m/d, the one clock read, at the driver
  edge). Local is correct for the target market (LATAM local dates are never
  ahead of the backend's UTC `today`, so the backend's future-date check
  cannot trip); `performed_on` is still mapped in `fieldTarget` → the finish
  area as a defensive catch-all. Domain validators remain clock-free.
- The `/session` route with a null draft (deep link / post-finish return)
  immediately redirects home — the driver state, not the route, is the source
  of truth for "a session is in progress" (OQ-F5, finding 7).

### 2.3 Domain validators (AC2/AC3 — backend-mirrored)

Ranges copied from `core::workout` and pinned by unit tests: reps `[1, 10_000]`
(int, required); weight `(0, 1000.0]` finite, optional; RPE `[6.0, 10.0]` in
exact `0.5` steps, optional (`(rpe * 2) == (rpe * 2).truncateToDouble()`);
exercise name **trimmed first, then counted in Unicode scalars**
(`trimmed.runes.length ≤ 100`, not UTF-16 `String.length` — the backend counts
`chars()` after trimming; architect finding 6), non-empty. Each validator
returns `String?` (null = ok), the `ProfileDraft` idiom.
`SessionDraft.toRequest()` is **total** (`SessionRequest?` — null until
`canFinish`), and its JSON omits absent optionals
(`weight_kg`/`rpe`/`muscle_group`), matching the `#[serde(default)]` backend
DTOs.

### 2.4 Data layer (AC5/AC6/AC8/AC9/AC12)

`WorkoutApi` over the shared `dioProvider`, all errors through
`ApiException.fromDio` (the single parsing authority):

- `list()` → `GET /workouts` → `List<WorkoutSession>` (server returns newest
  `performed_on` first; client does not re-sort).
- `create(SessionRequest)` → `POST /workouts` → `201 WorkoutSession`.
- `delete(id)` → `DELETE /workouts/:id` → `204`; a `404` surfaces as
  `ApiException(404)`. Failure ownership (architect finding 4): a small
  `SessionListController` (`Notifier`) owns `delete(id)` — submitting flag,
  `ApiException` caught to a state `error` ("that workout no longer exists" on
  404), list invalidated on success **and** on 404 (it is stale either way). No
  widget `try/catch`.

`WorkoutSession.fromJson` parses the R-0004 aggregate (transparent newtypes:
plain ints/doubles/strings on the wire): `id`, `user_id`, `performed_on`,
`exercises[{id, position, name, muscle_group?, sets[{id, position, reps,
weight_kg?, rpe?}]}]`, `created_at`, `updated_at`.

### 2.5 Presentation (AC1/AC3/AC7/AC8/AC9)

- **Home shell**: gains a `Start workout` FAB and the `SessionList` body
  (recent sessions over `workoutsProvider` via `AsyncValue.when`; empty state
  per AC8; delete via trailing icon → confirm dialog → `repo.delete` →
  invalidate). The R-0008 profile prompt renders above the list unchanged.
- **`/session` route** (authenticated, like `/onboarding`): the
  `LiveSessionScreen` renders the driver state — exercise tabs/cards in order,
  the current exercise's sets, a `SetEntryRow` (reps/weight/rpe + a
  `Repeat last set` button pre-filling from `driver.lastSet`), an
  `Add exercise` action opening the sheet (preset chips from
  `preset_exercises.dart`, a free-text field they pre-fill, optional muscle
  group chips), and a `Finish` button enabled by `canFinish` with the
  in-flight spinner-and-disable idiom.
- **Abandon (AC7)**: `PopScope` intercepts back/close while a draft exists →
  confirm dialog → `driver.abandon()` + leave. Starting over after a finish is
  a fresh `start()`.
- A backend `400 {field}` shows the message on the screen; `errorField` is
  mapped by `fieldArea(String?)` (beside the driver, the R-0008 note) to the
  **area** the message names — `reps`/`weight_kg`/`rpe` → "your sets";
  `name`/`muscle_group` → "the exercise"; `exercises`/`sets`/`performed_on` →
  "the workout" (`performed_on` defensively, finding 2). The MVP realizes the
  routing as a **field-aware message** (the error banner names the area); a
  focus-jump/scroll to the exact input is deferred polish. `fieldArea` is the
  same context the R-0027 voice transport speaks back.

### 2.6 Preset list (AC2 — presentation-only)

A `const` list of ~20 common lifts (squat, front squat, leg press, bench press,
incline bench, overhead press, deadlift, Romanian deadlift, barbell row,
pull-up, lat pulldown, dip, lunge, hip thrust, leg curl, calf raise, biceps
curl, triceps extension, lateral raise, face pull), each with a suggested
`MuscleGroup`. It lives in `presentation/preset_exercises.dart` (it is a UI
asset, not domain — architect finding 5; importing `MuscleGroup` from domain is
fine). Picking one **pre-fills the same validated free-text path** — no
client-side schema, no backend dependency; explicitly replaced by the M4
library (R-0012+).

## 3. Code outline

Representative; final form reconciled in step 5. Tests are authored by `qa`
in step 3 against §6. (Snippets omitted where §2 already shows the shape —
the driver in §2.2 *is* the outline.)

```dart
// domain/set_draft.dart
@immutable
class SetDraft {
  const SetDraft({this.reps, this.weightKg, this.rpe});
  final int? reps; final double? weightKg; final double? rpe;

  String? repsError();     // required, [1, 10_000]
  String? weightError();   // optional, finite, (0, 1000]
  String? rpeError();      // optional, [6,10] in 0.5 steps
  bool get valid;          // all three pass
}

// domain/session_draft.dart
@immutable
class SessionDraft {
  const SessionDraft({this.exercises = const []});
  final List<ExerciseDraft> exercises;
  bool get canFinish =>
      exercises.isNotEmpty && exercises.every((e) => e.sets.isNotEmpty);
  SessionRequest? toRequest(DateTime today); // total; null until canFinish
}

// data/workout_api.dart — same pattern as ProfileApi, all errors via
// ApiException.fromDio; delete() maps 204 → void.
```

## 4. Non-goals

Inherits R-0009 §4: no session editing UI, no program awareness, no
voice/earbud, no rest timers/supersets/reordering/notes, no offline draft, no
pagination, metric-only. Also: no exercise search/history suggestions (the
preset list is static), no per-set timestamps.

## 5. Open questions

**Resolved by the architect review (2026-06-10, REQUEST CHANGES → applied).**
OQ-F1/F3/F4 approved as proposed; OQ-F2 amended (add `performed_on`, finding 2);
OQ-F5 amended (null-draft redirect + `selectExercise` clamp, finding 7). Two
major findings (driver-enforced validation; local-date stamping) and five minors
folded into §2.1–§2.6 above.

- **OQ-F1 — Driver state granularity.** RESOLVED → one `Notifier` holding the
  whole `SessionDriverState` (driver = aggregate root), screens `select` slices.
  Per-exercise providers rejected — fragments the R-0027 seam.
- **OQ-F2 — Where does the 400-field → input mapping live?** RESOLVED → a
  `fieldTarget(String?)` function beside the driver (R-0008 precedent),
  including `performed_on` → finish area; the screen `ref.listen`s.
- **OQ-F3 — Sessions list provider.** RESOLVED → `FutureProvider` +
  invalidate-on-write; an `AsyncNotifier` only if a later R needs optimistic
  updates. Delete failures live on `SessionListController` (finding 4).
- **OQ-F4 — `lastSet` source.** RESOLVED → last set of the *current* exercise,
  derived reactively on `SessionDriverState` (finding 3).
- **OQ-F5 — Route shape.** RESOLVED → `/session` top-level authenticated route;
  driver state is the source of truth — a null-draft `/session` redirects home;
  `selectExercise` clamps out-of-range to a no-op (finding 7).

## 6. Acceptance criteria

Each maps 1:1 to an R-0009 criterion and to the qa agent's tests.

- [ ] **SAC1 → AC1.** Home offers Start; starting opens `/session` with an
  empty today-session; no date input exists anywhere in the flow.
- [ ] **SAC2 → AC2.** Add-exercise accepts a preset pick or free text (trimmed,
  non-empty, ≤100 chars) + optional muscle group; preset pre-fills the same
  field; the six wire tokens are pinned.
- [ ] **SAC3 → AC3.** Set validators mirror the backend at every boundary
  (1/10 000 reps; 0-exclusive/1000-inclusive weight; 6.0/10.0/0.5-step RPE,
  e.g. 7.5 ok / 7.3 rejected); invalid input blocks the add with a message;
  repeat-last-set pre-fills the previous set of the current exercise.
- [ ] **SAC4 → AC4.** Driver: start/add/select/log mutate ordered state;
  switching exercises preserves everything; the draft survives navigating away
  from `/session` and back (in-memory).
- [ ] **SAC5 → AC5.** `canFinish` is false for zero exercises or any set-less
  exercise; finishing POSTs exactly the logged JSON (omitted optionals absent);
  201 → list refreshed before navigation, draft cleared, home shows the session.
- [ ] **SAC6 → AC6.** 400 `{field}` → message + intact draft + field routed to
  its input; transport error → retryable message + intact draft; retry
  re-submits the same payload; 401 → interceptor sink → login. No rethrow into
  widgets.
- [ ] **SAC7 → AC7.** Leaving `/session` with a draft asks; cancel keeps the
  draft, confirm discards it; nothing is persisted either way.
- [ ] **SAC8 → AC8.** Home renders the list (date, exercise count, set count),
  newest first, with the AC8 empty state; prompt + list coexist.
- [ ] **SAC9 → AC9.** Delete confirms, calls DELETE, refreshes; a 404 shows a
  readable message; no edit affordance exists.
- [ ] **SAC10 → AC10.** The driver imports nothing from `presentation/`; its
  full API is exercised by pure-Dart unit tests with no widget pumped (the
  R-0027 seam proof).
- [ ] **SAC11 → AC11.** All gates green: `flutter analyze`,
  `dart format --set-exit-if-changed .`, `flutter test` (driver unit tests +
  widget tests per SAC1–SAC9).
- [ ] **SAC12 → AC12.** No `backend/` file changes; only `/workouts[/:id]`
  paths are called.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-10 | **Session driver = one widget-independent `Notifier` (aggregate root).** | The R-0027 voice transport consumes one API; fragmenting state across providers would smear the seam. (OQ-F1) |
| 2026-06-10 | **Clock read only at the driver edge (`finish()` stamps today).** | Domain stays deterministic/testable; honours the R-0008 architect note. |
| 2026-06-10 | **Preset list is a `const` presentation asset pre-filling the free-text path.** | Owner decision (R-0009 OQ2); no invented client schema; M4 replaces it. |
| 2026-06-10 | **All wire errors through the shared `ApiException.fromDio`; failure-as-state on the driver.** | One parsing authority (R-0008); widgets stay logic-free. |
| 2026-06-10 | **`FutureProvider` + invalidate for the sessions list.** | The `profileProvider` idiom; no optimistic updates needed yet. (OQ-F3) |
| 2026-06-10 | **(architect) The driver is the single validation-enforcement point: `addExercise`/`logSet` reject invalid input and return the reason (`String?`, null = accepted).** | A voice transport must not be able to corrupt the draft; the returned message is what R-0027 speaks. Keeps `state.error` reserved for finish/network failures. (finding 1) |
| 2026-06-10 | **(architect) `finish()` stamps the LOCAL calendar date; `performed_on` added to `fieldTarget` defensively.** | Local-midnight vs UTC edge would otherwise yield an unmappable 400 in UTC-ahead zones; local is correct for LATAM. (finding 2) |
| 2026-06-10 | **(architect) `canFinish`/`lastSet` derived reactively on `SessionDriverState`; delete failures owned by `SessionListController`; preset list moved to `presentation/`; name length = trimmed `runes.length`; `selectExercise` clamps; null-draft `/session` redirects home.** | Findings 3–7 — reactive enablement, no widget try/catch, honest layering, exact backend mirroring, total APIs. |

## Changelog

- _2026-06-10 — created (Draft). Realizes the accepted R-0009. Five HOW-level design questions (OQ-F1..F5) raised for the architect review; the driver API (§2.2) is the R-0027 seam._
- _2026-06-10 — **Accepted.** Architect review returned REQUEST CHANGES: two major (the driver must enforce validation itself — the R-0027 seam otherwise leaks; local-vs-UTC `performed_on` stamping) and five minor (reactive `canFinish`/`lastSet`; delete failure ownership; preset list to `presentation/`; `runes.length` name counting; `selectExercise`/null-draft semantics). All applied in lockstep across §2.1–§2.6/§5. OQ-F1/F3/F4 approved as proposed; OQ-F2/F5 amended per findings._
