# SPEC-0061 — One screen, built around a calendar

- **Status:** Accepted (owner, 2026-10-25) — two architect reviews applied; design v2 accepted
- **Realizes:** R-0061 (Accepted, owner, 2026-10-22). Supersedes R-0011.
- **Author:** Claude (main session), for owner review
- **Created:** 2026-10-23
- **Depends on:**
  - SPEC-0007 (app shell, router auth gate)
  - SPEC-0008 (onboarding)
  - SPEC-0009 (session driver)
  - SPEC-0012 and SPEC-0014 (archetypes, current program)
  - SPEC-0032 (voice loop)
  - SPEC-0035 (earbud hands-free)
- **Module(s):**
  - `backend/crates/core`: `archetype`, and a new `plan` module
  - `backend/crates/api`: `program`, `synthetic`, `workout`, `voice`, `db`
  - `backend/migrations`
  - `mobile/lib/src`: `shell`, `router`, `core/theme`, `hub`, a new `l10n`,
    and the feature folders

## 1. Motivation

R-0061 replaces the separate-screen app with one calendar screen. The
requirement is mostly client work. Reading the code shows two facts that
shape the whole design.

1. **The current program has no days.**
   - `GeneratedProgram` (`core/src/program/mod.rs`) stores a prose `split`,
     `days_per_week` and `highlight_exercises`.
   - `days_per_week` is parsed back out of the prose
     (`days_per_week_from_split`).
   - The highlights are a generic list per split keyword
     (`highlight_exercises_for_split`, `program/mod.rs:172`).
   - The six archetype templates (`archetype/seed.rs`) describe their splits
     only in prose, for example "4-day split (delts/triceps, back,
     chest/biceps, legs)".
   - R-0061 AC6's "the training day matters, for example leg day" therefore
     needs a **structured day sequence** that does not exist yet.
2. **A workout is not linked to any program.**
   - `workout_sessions` (migration 00003) has `performed_on` and nothing
     else.
   - The R-0009 "program-aware" session driver never reads the program.

`core::authoring::Schedule` (R-0039) already holds the shape we need:
`days: Vec<Vec<Entry>>`, where an empty day is a rest day. This spec mirrors
that idea and does not invent a new one.

## 2. Design

### 2.1 The day sequence (core, pure)

A new archetype-level field gives every template an ordered cycle of days:

```rust
// core::archetype
pub enum PlanDay {
    Training { focus: Vec<MuscleGroup>, exercises: Vec<String> },
    Rest,
}
pub struct ProgramTemplate { /* existing fields */ pub cycle: Vec<PlanDay> }
```

- **`focus` reuses `core::workout::MuscleGroup`,** the existing closed
  vocabulary. The server never sends a day *name*. The client builds the
  name from `focus` in the user's language: `[legs]` → "Pierna",
  `[shoulders, arms]` → "Hombro y brazo". This keeps AC11 true for every
  string the server would otherwise send.
- **Validation in `ProgramTemplate::with_cycle(self, cycle) -> Result<Self, ArchetypeError>`.**
  It is a separate builder step, not two more arguments on
  `ProgramTemplate::new`, which would reach eight and trip clippy's
  `too_many_arguments` under `-D warnings` (second review, N8). It checks:
  - at least one `Training` day;
  - every `Training` day has a non-empty `focus` and at least one exercise;
  - the cycle has no more than 14 days. That is a sanity bound of two
    weeks, not a claim about any record. Every cycle in Annex A is 8 days
    or shorter.

  `seed::all` keeps its single justified `expect` (SPEC-0012), so an
  invalid record fails the build.
- **Where the exercises come from (architect finding 8, second review N7).**
  No archetype record carries per-day exercises.
  - **Each training day lists its own exercises** in Annex A. There is no
    generic per-muscle list, because a coarse list put skull crushers on a
    chest-and-biceps day.
  - The bodypart **order** comes from the record's split prose only for
    `heavy-duty-mass` (Yates) and `high-intensity-minimalist` (Mentzer).
    `mass-monster-volume`'s prose gives the structure (back and legs each
    split across two sessions) but no order. For the other three the
    order is ours. Annex A says which is which, per cycle.
  - Every cycle is labelled **reconstructed** in the record's provenance,
    including the two documented splits, since the exercises are ours.
  - **A seed test** computes, for every `MuscleGroup` in a cycle, how many
    times it is trained per 7 days (`count × 7 / len`). The result must lie
    within ±1 of the record's `weekly_frequency_per_muscle`. That catches a
    cycle that trains a muscle three times in a "once weekly" record.
  - The owner approves Annex A before slice 2b merges (OQ-A).
- **`days_per_week` stops being shown as a weekly target (second review,
  N6).** It is parsed from prose (`program/mod.rs:151–162`), so every seed
  reads as 4, while the cycles train from 3.5 to 6 days a week.
  - The Progreso panel's weekly target (`program_progress.dart:79`) becomes
    the number of `Training` days the projection places in the current
    Monday-to-Sunday week, read from `GET /programs/me/schedule`.
  - The field stays in `GeneratedProgram` for old clients. Removing it is a
    separate change.
- **`highlight_exercises` stays as it is,** for the same reason.

**Why the archetype and not the stored program.** `user_programs.program`
is a per-user JSONB snapshot. The cycle is read from the library through
`user_programs.archetype_id`, which both the photo choose and the synthetic
choose store (`synthetic/mod.rs:183`).
- Rows that already exist get a cycle with no backfill.
- A later curation change reaches existing users. That is accepted for the
  beta (§7).
- `archetype_id` is TEXT with no foreign key. An id the library does not
  know is treated as **no program** (§2.4), never a 500.

### 2.2 The projection (core, pure): `core::plan`

```rust
pub struct DateRange { from: NaiveDate, to: NaiveDate }    // DateRange::new: from <= to, at most 62 days
pub struct DayLog {
    pub plan: Vec<u32>,      // plan_day_index of each current-program plan session that day, in created_at order
    pub any_session: bool,   // any workout at all that day (plan or free)
}
pub enum DayState {
    Done { completed: Vec<u32>,            // cycle indices completed that day (empty: free workout(s) only)
           pending: Option<u32> },         // with free workouts only: the training day still pending (N1)
    Planned { cycle_index: u32 },
    Rest { cycle_index: u32 },
    Nothing { pending: Option<u32> },      // past day; the training day a late log would complete, if any
}
pub fn project(cycle: &[PlanDay], starts_on: NaiveDate,
               logs: &BTreeMap<NaiveDate, DayLog>, today: NaiveDate,
               range: DateRange) -> Result<Vec<(NaiveDate, DayState)>, PlanError>
```

**What is stored and what is derived (architect finding 3).**
- A plan session stores **the cycle index it was seeded with**
  (`plan_day_index`, §2.3). That is a fact: the user did that day.
- The *cursor*, meaning which day is next, is derived by a replay.
- Without the stored index, a late log could rewrite history. Take the
  cycle [Legs, Push, Pull]. On Tuesday the user does legs. Logging Monday
  late as legs would then make Tuesday read as Push, a day never trained.
  Storing the index closes that.

**The replay.** A cursor `c` points into the cycle and wraps (`c % len`).
It starts at 0 on `starts_on` and walks every day from `starts_on` to
`range.to`, and no further: the replay only moves forward, so no log after
the range can move the cursor inside it (second review, N9). Before the
walk, `last_plan` is the latest date in **all** of `logs` that holds a plan
session, including dates after the range: a later completion is a fact
about the past, and an unlogged day before it must not offer that day
again (rule 4, N2). Slice 2a pinned both readings. The range is at most 62
days, counted inclusively (`to − from + 1`).
Rules apply in this order:

1. **Days before `starts_on`** never touch the cursor. They are emitted
   only if they fall in the range, as `Done { [] }` when any session was
   logged, otherwise `Nothing { pending: None }` (finding 1).
2. **A day with plan sessions**, whatever its tense (finding 4), becomes
   `Done { completed: plan, pending: None }`. The cursor moves to
   `last(plan) + 1`.
3. **A day with only free sessions**, whatever its tense, becomes
   `Done { completed: [], pending }`. If `cycle[c]` is `Rest`, the rest is
   consumed by time, the cursor moves (finding 2) and `pending` is `None`.
   If `cycle[c]` is a training day, it stays pending, the cursor stays, and
   `pending` is `Some(c)` (second review, N1). The client uses that only for
   today's secondary *Empezar sesión*.
4. **A past day with nothing logged:**
   - If the day is **before `last_plan`**, the state is
     `Nothing { pending: None }`. A later plan session already completed
     whatever was pending here, so offering it again would make a late log
     repeat it (second review, N2). Take [Legs, Push]: Monday is not
     logged, and Tuesday is logged as Legs. Monday must not offer Legs
     again. A late log on Monday is a free workout. The cursor still
     follows rule 4's second bullet below, which keeps the replay identical
     for every later day.
   - Otherwise, when `cycle[c]` is a training day, the state is
     `Nothing { pending: Some(c) }`: that training day **shifts**, and the
     cursor stays. When `cycle[c]` is a rest day, the state is
     `Nothing { pending: None }`, the rest is consumed and the cursor moves.
5. **Today or later, with nothing logged.** The day becomes
   `Planned { c }` or `Rest { c }`, and the cursor moves. The projection
   assumes the user follows the plan.

**Today, stated explicitly (finding 10):**
- A **plan session today** makes today `Done`. The projection resumes
  tomorrow at `last + 1`, so if the next cycle day is a rest day, tomorrow
  shows Rest. That is the sequence order, read as "the next pending
  **day**". See OQ-C for the wording of R-0061 AC6.
- A **free workout today** makes today `Done { completed: [], pending: Some(c) }`
  when `cycle[c]` is a training day. Today's primary action is
  *Ver lo registrado* (AC5), and the pending plan day is shown below it
  with a secondary *Empezar sesión* that carries the server's `pending`
  index (N1). Without that secondary action, a warm-up would hide the plan
  for the rest of the day.

**Edge cases from R-0061 AC6:**

| Case | Result |
|------|--------|
| A plan session on a projected **rest** day | The session stores the training index it was seeded with. Rule 2 marks that day done and the cursor moves past it, skipping the rest. |
| **Two** plan sessions on one day | `completed` holds both indices, and the cursor follows the last one. |
| A **late log** dated before a later plan log | Each session keeps its own stored index, so neither rewrites the other. The cursor after the replay follows the latest session by date. An unlogged day before `last_plan` offers only a free late log (rule 4, N2). |
| A **program change** | The new `user_programs` row has its own `starts_on`. `DayLog.plan` counts only sessions of the **current** program (finding 11). Old plan sessions show as `Done { [] }`, so they appear on the calendar but do not advance the new plan. |
| `starts_on` in the future | Days up to it fall under rule 1, and projection starts on `starts_on`. |
| A cycle with one training day | `[Legs]` projects Legs every day, and shifts the same way. |

**Errors.** `PlanError` has four variants:
- `NoTrainingDay`: unreachable after §2.1 validation, but it keeps the
  function total;
- `RangeInverted`;
- `RangeTooLong`: more than 62 days;
- `RangeTooFarAhead`: `range.to` later than `today + 366 days` (N9). With
  the walk bounded by `range.to`, this caps the work of any request.

`BTreeMap` makes "sorted, one entry per date" a property of the type, not a
precondition. The cost is O(days from `starts_on` to `range.to`), and
`range.to` is at most a year ahead. That is fine for a beta, and the spec
does not optimize ahead of need.

### 2.3 Schema (migration `00012_plan_projection.sql`)

```sql
ALTER TABLE user_programs
  ADD COLUMN starts_on DATE NOT NULL DEFAULT ((now() AT TIME ZONE 'UTC')::date);
UPDATE user_programs SET starts_on = (chosen_at AT TIME ZONE 'UTC')::date;

ALTER TABLE workout_sessions
  ADD COLUMN user_program_id UUID REFERENCES user_programs(id) ON DELETE SET NULL,
  ADD COLUMN plan_day_index  SMALLINT;
CREATE INDEX idx_workout_sessions_user_program ON workout_sessions (user_program_id);
```

- **The default on `starts_on` (finding 6).** An image rolled back past
  this migration still inserts programs without naming the column, and
  the default covers it. The migrator already has `ignore_missing` set
  (#111), so the old image also boots.
- **No CHECK pairing the two new columns.** `ON DELETE SET NULL` would
  break such a check. The API enforces the pairing (§2.4), and the replay
  counts only sessions of the current program.

### 2.4 Endpoints

**`GET /programs/me/schedule?from=YYYY-MM-DD&to=YYYY-MM-DD&today=YYYY-MM-DD`**
→ `200`

```json
{
  "program_id": "…", "starts_on": "2026-10-23",
  "cycle": [ { "kind": "training", "focus": ["legs"], "exercises": ["Back Squat", "…"] }, { "kind": "rest" } ],
  "days": [ { "date": "2026-10-21", "state": "done", "completed": [], "pending": 0 },
            { "date": "2026-10-22", "state": "nothing", "pending": null },
            { "date": "2026-10-23", "state": "planned", "cycle_index": 0 } ],
  "session_ids": { "2026-10-21": ["…"] }
}
```

- **Validation.**
  - `today` is the client's local date and must be within ±1 day of the
    server's UTC date. Every real local date satisfies that bound.
  - `DateRange::new` enforces `from ≤ to` and a range of at most 62 days.
  - `to` must be no later than `today + 366 days` (`PlanError::RangeTooFarAhead`,
    N9). An adversarial test asks for `from=9999-01-01&to=9999-01-31` and
    gets `422` at once.
  - Any violation returns `422` with the existing validation error shape.
- **No current program**, or an `archetype_id` the library does not know,
  returns `200` with `program_id: null`, an empty `cycle`, and `days` built
  from logs alone.
- **Owner-scoped.** The handler takes no id and reads only the caller's
  rows, so R-0006's 404 rule holds trivially. The adversarial test seeds
  user A's current program *and an A plan session*, then asserts that
  neither appears in B's response (finding 12).

**`POST /workouts`** gains optional `user_program_id` and `plan_day_index`
(finding 9). The checks run **in this order**, so an error never reveals
another user's data (second review, N10):
- They come **both or neither**. A free workout sends neither, so existing
  clients stay compatible. One without the other returns `422`.
- A `user_program_id` the caller does not own returns `404`, matching
  R-0006.
- The caller's own program that is **not current** returns `409`
  `program_not_current`. This happens when the program changed on another
  device mid-session. The client then saves the session as a free workout
  and says so, so a finished session is never lost.
- The current program's `archetype_id` is unknown to the library, so it has
  no cycle: `422` `plan_unavailable`. The client saves the session as free,
  as it does for a 409.
- `plan_day_index` must name a `Training` day of the cycle, and
  `performed_on ≥ starts_on`. Otherwise `422`.
- A test sends a **foreign** `user_program_id` with an out-of-range
  `plan_day_index`, and gets `404`, not `422`.

**`PUT /workouts/{id}`** (`workout/handlers.rs`, `replace`) keeps the stored
`user_program_id` and `plan_day_index`, because the body does not carry
them. It re-checks `performed_on ≥ starts_on` when they are set, and
returns `422` otherwise.

**Choose endpoints.** `POST` choose and `POST /programs/synthetic` take an
optional `starts_on` in the client's local date. It defaults to the
server's UTC date and has the same ±1 day bound.

### 2.5 Mobile: the shape of the app

**Routes.**
- Kept as they are: `/splash`, `/login`, `/register`, `/forgot-password`,
  `/reset-password` and `/onboarding`.
- `/home` becomes `MainScreen`.
- `/session` becomes the focus mode, opened with `context.push` so that
  back returns to `/home`.
- `/hub`, `/progress`, `/programs/current` and `/programs/get` are
  removed. They redirect to `/home`, so an old browser URL lands somewhere
  sane. Those redirects live **only in the `AuthAuthenticated` arm**.
- **R-0060's recovery routes.** The `recovering` arms that let
  `/forgot-password` and `/reset-password` through `AuthUnknown` and
  `AuthUnauthenticated` stay **verbatim**, and `/reset-password` stays
  reachable while authenticated. Router tests open
  `/reset-password?token=…` on a cold start and while authenticated, and
  assert that the token reaches the screen.
- **A cold start at `/session`.** The session draft lives in the
  `SessionDriver`'s memory, so a reload has none: `/session` without an
  active draft redirects to `/home`.

**The `context.go` guard (finding 13).** There are 26 calls today. A
source-scan test fails on any `context.go(` outside a path allowlist:
`lib/src/router/`, `lib/src/auth/presentation/` and
`lib/src/profile/presentation/onboarding_screen.dart`. Every other call
site moves to `context.push`, `showPanel` or a pop.

**`MainScreen`** is four regions, laid out as design v2 shows:
- **Calendar.** Week strip on a phone, month on a wide window. On a wide
  window it lives in a left rail with the nutrition card.
- **Selected day.** The hero: the day's name as a comic headline, its
  setup line, one primary action, the shift line ("¿Hoy no puedes? Pierna
  B pasa a mañana"), and the exercise list, each exercise with a **Física**
  button.
- **The physics check (AC9).** Opened by an exercise's Física button,
  inside the day region: beside the exercise list from 1180 px, below the
  exercise on narrower windows. For an exercise without a model the button
  reads *Pronto* and opens nothing.
- **Nutrition.** A protein ring with *+ Comida*, in the rail on a wide
  window and docked at the bottom on a phone.

Above them is the app bar, with the brand on the left and the profile menu
on the right. Below them are **+ Comida**, and a floating microphone at
the bottom.

**What `HomeShell` did, and where it goes (second review, N4).**
- **The liveness probe.** `MainScreen` watches `profileProvider` exactly as
  `HomeShell` does (`home_shell.dart:15–28`), so a restored but revoked
  token still meets a `401` and logs out (SPEC-0007 SAC5).
- **The onboarding prompt.** `ProfilePrompt` moves **inside the selected-day
  region**, above the day's content, with its dismiss kept
  (`onboardingDismissedProvider`). It belongs to the day region, so AC7's
  list of other controls is unchanged.

Two breakpoints, from design v2. Below 720 logical px the layout is a
column: the week strip, which opens the month on a tap, then the day with
its exercises and the open check below the exercise, and nutrition docked
at the bottom. From 720 up a 316 px rail holds the calendar and nutrition,
and the day takes the rest. From 1180 up the rail widens to 340 px and the
day splits, exercises on the left and the open check on the right. The
whole sits in a centered column, as #155 did.

**State (Riverpod).**
- `selectedDayProvider` (`StateProvider<DateTime>`, defaults to today). It
  survives panels because panels never replace the route.
- `scheduleProvider(DateRange)` is a `FutureProvider.family` over
  `GET /programs/me/schedule`. It is invalidated on:
  - a saved workout;
  - a program change;
  - app resume.
- **The primary action** is a pure function of `(day, today, DayState)`.
  Every index it needs comes from the server (`pending`, `cycle_index`),
  so the client never replays the plan (finding 5).

**Calendar widget.** It is written in-house: a grid of `DayCell`s over 7
columns.
- Glyphs: ● done, ○ planned (with its focus name), "—" rest, and nothing
  for an empty day.
- The first weekday comes from `MaterialLocalizations.firstDayOfWeekIndex`.
- The `table_calendar` package was **rejected**. A 7-column grid is roughly
  150 lines, and the package would bring its own theming and locale
  handling, which would both fight AC11 and AC12.

### 2.6 Panels (AC1, AC8)

One helper opens every panel:

```dart
Future<T?> showPanel<T>(BuildContext context, {required String title, required WidgetBuilder body});
```

- **Below 720 px** it is a `showModalBottomSheet`: draggable, full height
  on demand, with a ✕ in the title row.
- **From 720 px** it is a `showGeneralDialog` that slides in from the
  right, 480 px wide, with `barrierDismissible: true`.
- **Esc** is already handled by `ModalRoute`'s `DismissIntent`, so no
  extra `Shortcuts` are added (finding 13). Android back pops the modal
  route.
- **The nested choose flow.** The Programa panel hosts its choose flow in
  a nested `Navigator` wrapped in `NavigatorPopHandler`. Back and Esc
  then step back *within* the flow and close the panel only from its
  first page.
- **A tap outside always closes the whole panel,** from any page of the
  nested flow, as AC1 asks. The barrier calls the root navigator's `pop`,
  not `maybePop`, which would only step back (second review, N11).
- Tests cover both: back steps within the flow, and a barrier tap from its
  second page closes the panel.

| Panel | Body (moved from) | Notes |
|-------|-------------------|-------|
| Programa | `program_detail_screen.dart`, `program_proposals_screen.dart`, `body_type_picker_screen.dart`, `synthetic_proposals_screen.dart` | The nested flow (above). On choose it sends `starts_on` and invalidates `scheduleProvider`. |
| Medidas | `log_measurement_sheet.dart`, `coach_card.dart` | |
| Progreso | `progress_screen.dart`, `trend_chart.dart` | R-0011's trends live here (OQ-5). |
| Cuenta | new and small | Name, email and sign-out (moved from the app bar), plus *Editar perfil*, which opens onboarding in edit mode as the hub's "Profile" did. |
| Comida | `meal_quick_log_sheet.dart` | Already a sheet. It moves onto `showPanel`. |
| Dictado | new sheet over the voice loop (§2.8) | OQ-4: dictation only. |

### 2.7 Focus mode (AC10)

- `LiveSessionScreen` becomes the `/session` route, pushed full-screen.
- The `SessionDriver`, `VoiceCoach` and the R-0035 earbud transport are
  untouched. So are the exercise preset picker and `muscle_map.dart`,
  which already live in `live_session_screen.dart`.
- `SessionDriver.start(PlanSeed? seed, {DateTime? performedOn})`, where
  `PlanSeed { userProgramId, index, exercises }` is a value the caller
  builds from the schedule it already holds. The driver never reads the
  schedule itself (second review, N12).
  - With a seed, it pre-fills the exercises and sends `user_program_id` and
    `plan_day_index` on save.
  - Without one, the session is free, as today.
  - A `409 program_not_current` or a `422 plan_unavailable` saves the
    session as free and shows a notice (§2.4).
- **Every way to finish** goes through the driver's `done` state: the
  Finish button, voice, and the earbud's `finish()`
  (`earbud_coach.dart:141`). Focus mode listens for `done` and pops, so the
  earbud path cannot leave the user on a stale screen.
- **Leaving** with unsaved sets goes through a `PopScope` with
  `canPop: false` and a confirm dialog: "Salir sin guardar".
- **Finishing** pops back to `/home`, invalidates `scheduleProvider`, and
  selects the day the session was performed.
- **Registrar tarde** pushes the same screen with `performedOn` set to the
  selected past day. It is pre-filled with that day's server-provided
  `pending` training day, when there is one.

### 2.8 The voice hub (OQ-4; corrected after finding 7)

`voice_hub_screen.dart` is the hub *screen*: a radial ring and the chat
thread of the voice loop. The loop itself is `hub/sergeant.dart`, the
R-0032 hands-free conversation:
- it prompts, listens and acts;
- it logs meals by voice;
- "start workout" hands off to `VoiceCoach`'s own hands-free loop;
- it surfaces navigation as a `navigateTo` route string.

| Part | Goes to |
|------|---------|
| The voice loop (`Sergeant` notifier) | **Kept**, as the engine of the **Dictado** sheet. It is renamed `VoiceLoop` to match the #150 rebrand, and its persona strings move to the l10n catalogues. Every route string becomes a typed `VoiceEffect` (below). |
| Chat thread | Shown inside the Dictado sheet |
| Radial ring: Workout, Program, Body match, History, Profile | **Removed** (OQ-B). Each item has a home on the new screen: Workout is *Empezar sesión*, Program is the Programa panel, Body match is inside Programa's choose flow, History is the calendar, and Profile is *Editar perfil* in the Cuenta panel. |

**Route strings become effects (second review, N3).** Today the server
answers navigation intents with route strings:
- the keyword parser in `api/src/voice/parse.rs:106–189`;
- the LLM prompt's `"route":"/session|/home|/programs/current|/programs/get|/onboarding"`
  at `parse.rs:400`;
- the local intents in `sergeant.dart:236–257`.

After §2.5 most of those routes redirect to `/home`, so voice would
silently stop doing anything. The change:
- The wire field becomes an **effect token**: `open_focus_mode`,
  `open_program`, `open_onboarding`, `close`. `parse.rs`, its LLM prompt and
  its tests change to emit tokens. The old `route` field is kept for one
  release, filled from the token, for clients still in the field.
- The client maps tokens to a sealed `VoiceEffect`: `OpenFocusMode`,
  `OpenPanel(PanelKind.programa)`, `OpenOnboarding`, `Close`. The local
  intents in `sergeant.dart` emit the same values.
- **"Start workout" by voice does what *Empezar sesión* does.** It builds
  today's `PlanSeed` from the schedule (`Planned`, or a today `Done`'s
  `pending`) and opens focus mode with it, so a voice-first user advances
  the plan. With nothing pending it starts a free session.
- **Language.** The server's `message` field is English prose, and the
  client no longer displays or speaks it. The Dictado thread and the TTS
  use the client's localized string for each intent and effect, spoken in
  the resolved locale. The field stays on the wire for old clients.

### 2.9 Localization (AC11)

- **Dependencies:** `flutter_localizations` (SDK) and `intl: any`, which
  follows the SDK's pin (finding 14). Both go through `gen-l10n` with
  `l10n.yaml`. A third-party package (`easy_localization`, `slang`) was
  rejected because the first-party route covers the need.
- **Catalogues:** `lib/l10n/app_es.arb` is the template and
  `lib/l10n/app_en.arb` comes from today's strings. Both are complete.
  Completeness is checked in `flutter test` by a parity test over the two
  ARB files (every template key has an English value, and English has no
  extra key), which needs no build step. (Slice 1a: gen-l10n's
  untranslated-messages file is a build artifact, so a test over the
  sources replaces it.)
- **Locale rule (OQ-1):** `supportedLocales: [es, en]`. A
  `localeListResolutionCallback` walks the device's language list, takes
  the first with a catalogue, and returns `es` when nothing matches.
- **Formatting:** dates, weekdays and numbers use `intl` with the resolved
  locale.
- **Literal-string guard.** A test scans `lib/src/**.dart` for string
  literals passed to `Text(`, `tooltip:`, `label:`, `hintText:`,
  `labelText:`, `title:` and `semanticLabel:`, and to the voice loop's
  spoken lines (`_say(`), and fails with file:line.
  - It matches across line breaks (`Text\(\s*'`), because `dart format`
    puts a long literal on the line after `Text(` (second review, N13).
  - It has no allowlist, because `Brand.appName` is not a literal.
  - It is a regex smoke test and will miss strings built far from their
    widget. Reviews catch the rest.
  - During slices 1b–1d it carries a **shrinking allowlist** of folders not
    yet migrated. Slice 1d leaves it empty, and from then on an entry fails
    the test.
- **Focus names:** `focusName(List<MuscleGroup>, AppLocalizations)` joins
  the localized group names: "Pierna", "Hombro y brazo".
- **Exercise names stay data** in English (§4).

### 2.10 The @fisicobuenfisico look (AC12, OQ-D resolved)

`app_theme.dart` is rewritten around tokens. Its "drill-sergeant green"
header and palette go away. The values come from design v2, accepted by
the owner on 2026-10-25: black, white and red, as in the channel's latest
reels.

| Token | Light | Dark | Allowed use |
|-------|-------|------|-------------|
| `ground` | `#F4F3F1` | `#0B0B0B` | Background, with a faint halftone dot pattern |
| `panel` | `#FFFFFF` | `#161616` | Cards and panels; a hard 2 px border in `line` and a hard offset shadow |
| `text` | `#111111` | `#F2F2F2` | Body text, `onSurface` |
| `muted` | `#505050` | `#ABABAB` | Secondary text (8.1:1 and 7.9:1 on `panel`) |
| `line` | `#111111` | `#D9D9D9` | Borders, the figure's ink |
| `red` | `#C8102E` | `#FF4D4D` | The only colour: force, torque, the hot end of the tension ramp, the primary action, the stamp. Passes as text: 5.9:1 and 5.5:1 on `panel` |
| `on-red` | `#FFFFFF` | `#0B0B0B` | Text on the primary action (5.9:1 and 6.0:1) |
| `heat0 → heat1 → heat2` | `#BDBDBD → #7A7A7A → red` | `#4D4D4D → #8F8F8F → red` | The tension ramp. Marks only, never text: its low end is low-contrast on purpose |
| `bar` / `bar-text` / `bar-red` | `#0B0B0B` / `#F2F2F2` / `#FF4D4D` | same | The CONCEPTO footer and focus mode, which are dark in both themes |

- **Semantics.** Heat means tension magnitude and nothing else. No colour
  means good or bad, and no copy says safe or dangerous (R-0045 AC14).
- **Contrast test (first review, finding 14).** It takes its pairs from the
  built `ColorScheme`, every `on*` against its base in both modes, and
  adds the custom tokens above, each classified by its allowed use. Body
  text needs 4.5:1, large text and marks 3:1. It fails naming the pair.
- **Type, bundled as assets** so nothing is fetched at run time
  (`google_fonts` rejected, as before). Each licence is registered with
  `LicenseRegistry`.
  - **Bangers** (OFL): headlines, short and sparing.
  - **Luckiest Guy** (Apache 2.0): the big numbers in readout panels. The reels
    use it because Bangers draws 1 and 7 alike.
  - **IBM Plex Mono** (OFL): data, setup lines, the formula bar.
  - **Archivo** (OFL): UI text.
- **Motifs, with restraint:** the halftone ground, hard borders and
  shadows, the `panelito` readout (label on top, big number below), the
  FÓRMULA bar, the CONCEPTO footer, the stamp worded **COMPROBABLE**
  (checkable), never COMPROBADO (verified), and the signature "mecánica
  fbf · @fisicobuenfisico". Left out of an app UI: splatter, tape, grain,
  photo LUTs and issue numbers.
- **Focus mode is dark in both themes** (AC12), on the `bar` tokens.
- **Before sign-in:** the brand badge and buttons move to `red` on
  `ground`.

### 2.11 Responsiveness (AC13)

Widget tests at 390×844 and 1440×900, as #155 did, cover `MainScreen`, each
panel and focus mode. Each test asserts:
- no `RenderFlex` overflow;
- no horizontal scrollable wider than the screen;
- that the primary action's rect is inside the viewport with no scroll.

### 2.12 Slices (second review, N5)

R-0061 §4's first slice, as written, would be thousands of lines: the
shell, the calendar, more than 158 literals in a client of about 10k
lines, and the theme. It is cut into slices of roughly 100–400 lines.
Each merges on its own, with its tests, and each is usable or invisible
in the beta when it lands.

| Slice | Content | Stack | Depends on |
|-------|---------|-------|------------|
| 1a | l10n infrastructure: `l10n.yaml`, the two ARB files, the locale rule, and the literal guard with its shrinking allowlist (§2.9) | mobile | — |
| 1e | Theme tokens, the contrast test and the four bundled faces (§2.10). The halftone painter ships with 1f, its first user | mobile | — |
| 1b–1d | Move the strings to the catalogues one feature folder at a time; 1d empties the allowlist. 1b also deletes `Brand.tagline` and wires `appTagline` into `BrandHeader`, adding the delegates to the auth widget tests | mobile | 1a |
| 1f | Router, `MainScreen`, `showPanel`, the halftone ground, the calendar from logs only, the liveness probe and the prompt (§2.5–§2.6). Programa and Progreso push the existing `/programs/current` and `/progress` screens from the profile menu, Medidas opens its sheet, and `/programs/get` is reachable from the program screen's empty state, until slice 3 replaces them with panels, so AC2 holds at every merge; the `context.go` guard ships with a shrinking allowlist for the folders slices 3 and 5 migrate | mobile | 1a, 1e |
| 1f | Router, `MainScreen`, `showPanel`, the calendar from logs only, the liveness probe and the prompt (§2.5–§2.6) | mobile | 1a, 1e |
| 2a | `core::plan` and its tests (§2.2) | backend | — |
| 2b | Annex A seeds, `with_cycle`, and the frequency seed test (§2.1) | backend | 2a, OQ-A |
| 2c | Migration 00012, `GET /programs/me/schedule`, and the `POST`/`PUT /workouts` and choose changes (§2.3–§2.4) | backend | 2b |
| 2d | Planned and rest states on the client, the primary and secondary actions, the Progreso target (§2.5, N6) | mobile | 1f, 2c |
| 3 | Panels: Programa with its nested flow, Medidas, Progreso, Cuenta, Comida, Dictado with `VoiceEffect` and the server tokens (§2.6, §2.8) | both | 1f, 2c |
| 4 | The physics check per exercise (AC9): the Física buttons, the *Pronto* state, and the squat check from R-0059's endpoint, with vectors, lever, panels and the live graph | mobile | 1f, R-0059 |
| 5 | Focus mode with `PlanSeed`, the `done` path and the cold-start redirect (§2.7) | mobile | 2d |

## 3. Code outline

```rust
// core/src/plan.rs — pure, no I/O, no clock
pub fn project(cycle: &[PlanDay], starts_on: NaiveDate, logs: &BTreeMap<NaiveDate, DayLog>,
               today: NaiveDate, range: DateRange) -> Result<Vec<(NaiveDate, DayState)>, PlanError> {
    let mut cursor = Cursor::new(cycle)?;                     // NoTrainingDay; wraps modulo len
    let mut out = Vec::new();
    let emit = |out: &mut Vec<_>, d, s| if range.contains(d) { out.push((d, s)) };

    for d in range.from.iter_days().take_while(|d| *d < starts_on) {          // rule 1
        let any = logs.get(&d).is_some_and(|l| l.any_session);
        emit(&mut out, d, if any { DayState::Done { completed: vec![] } } else { DayState::Nothing { pending: None } });
    }
    // DateRange::new(from, to, today) already rejected an inverted, over-long or too-far-ahead range.
    let last_plan = logs.iter().rev().find(|(_, l)| !l.plan.is_empty()).map(|(d, _)| *d);
    for d in starts_on.iter_days().take_while(|d| *d <= range.to) {           // stops at range.to (N9)
        let state = match logs.get(&d) {
            Some(l) if !l.plan.is_empty() => { cursor.complete_after(*l.plan.last().unwrap_or(&0)); // rule 2 (non-empty by guard)
                                               DayState::Done { completed: l.plan.clone(), pending: None } }
            Some(l) if l.any_session => DayState::Done { completed: vec![], pending: cursor.pass_free() }, // rule 3 (N1)
            _ if d < today => cursor.pass_day_unlogged(last_plan.is_some_and(|lp| d < lp)),            // rule 4 (N2)
            _ => cursor.project_one(),                                                                    // rule 5
        };
        emit(&mut out, d, state);
    }
    Ok(out)
}
```

The `unwrap_or` sits behind a non-empty guard. A deliberate, commented
`expect` would satisfy §6 just as well. The architect picks one at PR
review.

```dart
// mobile/lib/src/home/primary_action.dart — pure; every index comes from the server
sealed class DayAction { const DayAction(); }
final class StartSession extends DayAction { const StartSession(this.planDayIndex); final int? planDayIndex; }
final class ViewLogWithPending extends DayAction { const ViewLogWithPending(this.pending); final int pending; } // N1: secondary Empezar sesión
final class LogLate extends DayAction { const LogLate(this.planDayIndex); final int? planDayIndex; }
final class ViewLog extends DayAction { const ViewLog(); }
final class NoAction extends DayAction { const NoAction(); }

DayAction primaryAction({required DateTime day, required DateTime today, required DayState state}) =>
    switch ((day.compareTo(today), state)) {
      (0, Done(pending: final p?)) => ViewLogWithPending(p),
      (_, Done()) => const ViewLog(),
      (0, Planned(:final cycleIndex)) => StartSession(cycleIndex),
      (0, _) => const StartSession(null),
      (< 0, Nothing(:final pending)) => LogLate(pending),
      _ => const NoAction(),
    };
```

## 4. Non-goals

- **Translating exercise names.** They are curated data in English. A
  catalogue for them is a separate requirement, if the beta asks for one.
- **Freezing the cycle per user** (§2.1, §7).
- **Editing `starts_on`** after choosing. R-0061 OQ-2 made it "the day the
  program is chosen". To restart a plan, the user chooses it again.
- **Authored programs (R-0039/R-0041) as the current program.** That is
  M-Platform work.
- **Offline, notifications** (R-0036), **trainer views.**
- **The squat check's numbers** come from R-0059's endpoint. The deadlift
  and bench checks come from R-0045's garust work; until then they read
  *Pronto*.

## 5. Open questions

- **OQ-A. The six cycles, with exercises per day.** *Resolved 2026-10-25:
  Annex A v2 ships as tabled under the owner's authorization of the spec.*
  Days are named by focus only (owner). Mentzer's spacing and the exercise
  lists stay open as data fixes in `seed.rs` (Annex A §4).
- **OQ-B. Removing the radial ring** (§2.8). *Resolved 2026-10-24 (owner):
  removed.* Each of its five items has a home on the new screen.
- **OQ-C. The wording of R-0061 AC6.** *Resolved 2026-10-24 (owner):* the
  projection follows the next pending **day** of the sequence, rest
  included (§2.2). R-0061 AC6 is amended by one word, noted in its
  changelog.
- **OQ-D. The visual identity of @fisicobuenfisico** (§2.10). *Resolved
  2026-10-25 (owner): design v2 accepted.* R-0061 AC7, AC9 and AC12 are
  amended, and §2.10 carries the tokens.

## 6. Acceptance criteria

| R-0061 AC | Verified by |
|-----------|-------------|
| AC1 | Widget tests: each panel closes on ✕, Esc, a tap outside and back, with `selectedDayProvider` unchanged. The nested choose flow steps back within itself, and a barrier tap from its second page closes the whole panel (N11). The `context.go` allowlist scan. |
| AC2 | A widget test walks every capability in R-0061 AC2's list from `MainScreen`. Dictating a meal through the Dictado sheet logs it. Saying "start workout" opens focus mode **with today's `PlanSeed`**, and a voice effect test covers each token (N3). `parse.rs` tests assert tokens, never route strings. `MainScreen` keeps the 401 liveness probe, and shows the onboarding prompt to a user with no profile (N4). |
| AC3 | Existing router tests stay green, and the removed routes redirect to `/home` only when authenticated. `/reset-password?token=…` reaches its screen on a cold start and while authenticated (R-0060). |
| AC4 | `core::plan` unit tests, one per rule in §2.2 and per edge-case row: shift; rest consumed by time; **a free workout consuming a rest day**; days before `starts_on` leaving the cursor alone; a future `starts_on`; a range wholly before `starts_on`; a one-training-day cycle; a future-dated log shown as Done; the late-log case from finding 3; **a free workout on a training day leaves it pending** (`Done { pending: Some }`, N1); **an unlogged day before a later plan log offers no pending** (N2); `RangeTooFarAhead`. Also a `DayCell` golden test per state, and week and month widget tests. A seed test checks each cycle's per-muscle weekly frequency against `weekly_frequency_per_muscle` (§2.1, N7). |
| AC5 | `primaryAction` unit tests over every (day vs today) × state pair, plus `ViewLogWithPending` and its secondary *Empezar sesión* on a today that holds only a free workout, with the server's index (N1). |
| AC6 | API tests for `GET /programs/me/schedule`: happy path; no program; unknown `archetype_id`; inverted and over-long range; `today` at ±1 day accepted and ±2 rejected; the cross-user adversarial test; `from=9999-01-01` → 422 (N9). `POST /workouts`: both-or-neither; foreign id → 404, **even with an invalid index** (N10); own non-current program → 409; unknown `archetype_id` → 422 `plan_unavailable`; rest index or `performed_on < starts_on` → 422. `PUT` keeps the plan fields and re-checks. Choose and synthetic choose: the `starts_on` default, ±1 day accepted, ±2 rejected. The migration test checks the backfill and the default. |
| AC7 | A widget test lists exactly the controls on `MainScreen`, with the day's Física buttons and an open check's play/pause, scrubber and *Tus medidas* counted inside the day region. |
| AC8 | Widget tests: each panel opens and holds its body, and choosing a program completes inside the Programa panel. |
| AC9 | Widget tests: every exercise shows a Física button; an unmodelled exercise reads *Pronto* and opens nothing; the squat check opens beside the list at 1440 px and below the exercise at 390 px; the worked sum equals the hip panel's value for the current frame; no copy in the check matches a safe/dangerous word list. |
| AC10 | Widget tests: focus mode is pushed; unsaved sets prompt before leaving; finishing by button, by voice and by the earbud's `finish()` returns to the day, shown as done (N12); a 409 or a `plan_unavailable` saves the session as free and shows a notice; a cold start at `/session` lands on `/home`. The R-0035 and R-0032 suites stay green. |
| AC12 | The `ColorScheme`-derived contrast test over §2.10's pairs, structural theme tests in both modes (token values, primary and surface, the faces; no pixel goldens, which rasterize differently on the Linux runner), focus mode dark in both, and a licence-registry test for the four faces. |
| AC11 | The literal-string guard, including multi-line `Text(` and `_say(` (N13), the ARB parity test, and locale resolution tests (`es`, `en`, `fr` → `es`). |
| AC13 | Widget tests at 390×844 and 1440×900 (§2.11). |

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-10-23 | The day sequence lives in the **archetype template** (`cycle`) and is read through `archetype_id`. An unknown id is treated as no program. | Existing users get a plan with no backfill. A curation change reaching existing users is accepted for the beta. |
| 2026-10-23 | A plan session **stores the cycle index it was seeded with**. The cursor is derived by replay. | Architect finding 3: with only a derived index, a late log can rewrite history and mark a day done that was never trained. That is the failure "leg day matters" exists to prevent. What was seeded is a fact and cannot go stale. |
| 2026-10-23 | Rest days are consumed by any day without a plan session, **including a day with a free workout**. | Architect finding 2, and R-0061 AC6: "rest days are consumed by time passing". |
| 2026-10-23 | Day names come from **`focus: Vec<MuscleGroup>`**, localized on the client. | The server sends no user-visible prose, so AC11 holds end to end. It reuses the existing closed vocabulary. |
| 2026-10-23 | The voice loop (`sergeant.dart`) is **kept and renamed**, not deleted. Only the hub screen's ring is removed. | Architect finding 7: the loop carries R-0032's voice meal logging and the hand-off to hands-free training, so deleting it would break AC2. |
| 2026-10-23 | In-house calendar grid. `table_calendar` and `google_fonts` were rejected, and `flutter_localizations` and `intl: any` were added. | A small grid does not justify a dependency that fights AC11 and AC12. Fonts are bundled so nothing is fetched at run time. The l10n packages are first-party. |
| 2026-10-23 | `starts_on` and `today` come from the client, bounded to ±1 day of the server's UTC date. `starts_on` has a column default. | "Today" is the user's local date, and the bound is exactly the span of real time zones. The default keeps an image rollback able to choose programs (architect finding 6). |
| 2026-10-24 | The radial ring is removed (OQ-B), and AC6 reads "the next pending day of the sequence" (OQ-C). | Owner. |
| 2026-10-24 | An unlogged past day before the latest plan log offers only a free late log. | Second review N2: otherwise a late log repeats a training day that a later session already completed, which is the very failure "leg day matters" exists to prevent. |
| 2026-10-24 | Voice navigation is a token on the wire and a typed `VoiceEffect` on the client; the client never shows the server's English prose. | Second review N3: route strings would silently redirect to `/home`, and server prose would break AC11. |
| 2026-10-24 | Each training day lists its own exercises; the generic per-muscle list is dropped. A seed test checks per-muscle frequency against the record. | Second review N7: a coarse list put triceps work on a biceps day, and several cycles contradicted their record's frequency. |
| 2026-10-24 | The weekly target comes from the projection, not from `days_per_week`. | Second review N6: `days_per_week` reads 4 for every seed. |
| 2026-10-24 | The work ships in eleven slices (§2.12). | Second review N5: one first slice would have been thousands of lines. |
| 2026-10-25 | Design v2 accepted: the @fisicobuenfisico black, white and red look; the physics check per exercise inside the day; "Pronto" for unmodelled exercises. | Owner. The channel's 22k followers come for the physics; the app must look like the reels and put the check with the training. R-0061 AC7, AC9 and AC12 amended. |
| 2026-10-25 | Days are named by focus only. | Owner: programs come in many styles (5×5, high intensity, volume). The physics check attaches to the exercise. |

## 8. Architect review — REQUEST CHANGES (2026-10-23), applied

The architect returned 15 findings: 2 blocking, 7 major, 6 minor. Every one
is folded in above.

| # | Severity | Finding | Where it landed |
|---|----------|---------|-----------------|
| 1 | Blocking | Days before `starts_on` moved the cursor, a stale log stalled the iterator, and a plan log before `starts_on` was accepted | §2.2 rule 1, `BTreeMap`, §2.4 `performed_on ≥ starts_on` |
| 2 | Blocking | A free workout did not consume a rest day | §2.2 rule 3, §7 |
| 3 | Major | A derived-only index lets a late log rewrite history | §2.2, §2.3 `plan_day_index`, §7 |
| 4 | Major | A future-dated log was shown as Nothing | §2.2 rules 2–3, "whatever its tense" |
| 5 | Major | The client computed `pending` | `Nothing { pending }`, §2.5, §3 |
| 6 | Major | NOT NULL with no default broke choose after a rollback | §2.3 default |
| 7 | Major | Wrong hub inventory, and deleting `sergeant.dart` broke AC2 | §2.8 rewritten |
| 8 | Major | `highlight_exercises` per record does not exist | §2.1 `default_exercises`, labelled reconstructed |
| 9 | Major | 404 on own non-current program lost a session; PUT not covered | §2.4 409 and save as free, PUT rule |
| 10 | Minor | "Today" cases unstated | §2.2 "Today, stated explicitly", OQ-C |
| 11 | Minor | Preconditions not enforced | `BTreeMap`, `DateRange::new`, current-program-only counting |
| 12 | Minor | Adversarial test should seed A's plan session | §2.4 |
| 13 | Minor | `context.go` allowlist, redundant Esc shortcuts, nested-Navigator back | §2.5, §2.6 |
| 14 | Minor | Contrast pairs from `ColorScheme`, moot allowlist, `intl: any`, OFL registration | §2.9, §2.10 |
| 15 | Minor | Missing tests | §6 |

**Baseline gates on `main`, as the architect ran them:**
- `cargo fmt --check` is green.
- `clippy -D warnings` is green on the pinned toolchain.
- `cargo test -p fitai-core` passes 360 tests.
- **The `api` crate tests were not run.** They fail to link on this machine
  (`ort_sys`, ONNX runtime missing). That is an environment gap, not a
  result, so CI is the gate for them.

## 9. Second architect review — REQUEST CHANGES (2026-10-24), applied

None of the findings was blocking. Of the 15 first-review findings, 10 were
confirmed resolved and 5 partially (5, 7, 10, 13 and 15). The partial ones
are closed by N1, N3, N11 and the §6 additions. Thirteen new findings: 7
major and 6 minor.

| # | Severity | Finding | Where it landed |
|---|----------|---------|-----------------|
| N1 | Major | `Done` had no `pending`, so the secondary *Empezar sesión* needed a client replay | §2.2 rule 3, §3 `ViewLogWithPending`, §6 AC4/AC5 |
| N2 | Major | An unlogged day before a later plan log still offered its pending day, so a late log repeated it | §2.2 rule 4, `last_plan` |
| N3 | Major | Voice route strings would silently redirect to `/home`; server prose broke AC11 | §2.8 tokens and `VoiceEffect` |
| N4 | Major | `HomeShell`'s liveness probe and onboarding prompt had no home | §2.5 |
| N5 | Major | No slice plan | §2.12 |
| N6 | Major | `days_per_week` contradicts the cycles | §2.1 |
| N7 | Major | Annex A's provenance claim and per-muscle frequency | §2.1, Annex A v2 |
| N8 | Minor | Eight-argument constructor | §2.1 `with_cycle` |
| N9 | Minor | Unbounded range walk | §2.2, §2.4, `RangeTooFarAhead` |
| N10 | Minor | Check order could leak another user's cycle | §2.4 |
| N11 | Minor | Barrier tap stepped back instead of closing | §2.6 |
| N12 | Minor | The driver read the schedule; earbud finish and cold start | §2.7, §2.5 |
| N13 | Minor | The guard missed multi-line literals and spoken lines | §2.9 |

It also confirmed `intl: any` acceptable (the lockfile is tracked, and
`flutter_localizations` pins `intl` exactly), and asked to close #159, which
was done with a note on what to salvage.

## Changelog

- 2026-10-23 — created (Draft).
- 2026-10-23 — architect review (REQUEST CHANGES, 15 findings) applied in
  full. §2.2 algorithm reworked, a stored `plan_day_index` added, §2.8
  corrected, OQ-C added.
- 2026-10-23 — Annex A drafted: the six day cycles and `default_exercises`,
  for owner approval (OQ-A).
- 2026-10-24 — second architect review (REQUEST CHANGES, 13 findings, none
  blocking) applied in full. OQ-B and OQ-C resolved by the owner. OQ-D
  added for the @fisicobuenfisico visual identity. §2.12 slices added.
  Annex A v2 with exercises per day.
- 2026-10-25 — design v2 accepted by the owner. §2.5 regions and
  breakpoints, §2.10 tokens and type, slices 1e and 4, §6 rows for AC7,
  AC9 and AC12 updated. OQ-D resolved. Status → Accepted.
- 2026-10-25 — slice 2a: `last_plan` reads every log (N2 over N9), the
  62-day range is inclusive, and range errors live in `DateRange::new`.
- 2026-10-25 — slice 1f: the removed routes' redirects move to slice 3 (the old screens stay reachable, pushed, until the panels exist); the `context.go` guard carries a shrinking allowlist like the l10n guard.
- 2026-10-25 — slice 2b: Annex A accepted as drafted (OQ-A); `GET /archetypes` now carries `program_template.cycle`.
- 2026-10-25 — slice 2c: the `422` reasons are fixed tokens the client may match on: `today_out_of_bounds`, `starts_on_out_of_bounds`, `range_inverted`, `range_too_long`, `range_too_far_ahead`, `plan_fields_both_or_neither`, `plan_unavailable`, `plan_day_not_training`, `performed_on_before_starts_on`; the `409` is `program_not_current`.
- 2026-10-25 — slice 3 (backend): `POST /voice/intent` answers navigation with an `effect` token — `open_focus_mode`, `open_program`, `open_onboarding`, `close` — serialized beside the deprecated `route`, which stays for one release filled from the token (`VoiceEffect::legacy_route`); the LLM prompt asks for the token and anything else the model writes maps to `close`.
