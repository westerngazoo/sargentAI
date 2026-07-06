# SPEC-0032 — Voice Logging Assistant

- **Status:** Accepted
- **Realizes:** R-0032
- **Author:** Claude (retro-spec, R-0057)
- **Created:** 2026-07-03
- **Depends on:** R-0004 (workout log — `NewWorkoutSession`, `db::insert_session`),
  R-0005 (nutrition log — `NewNutritionLog`, `db::insert_nutrition_log`),
  R-0027 (voice coach / hands-free session — `SpeechInput`, `VoiceOutput`,
  `SessionDriver`), R-0014 (`currentProgramProvider`, highlight exercises),
  R-0002 (`AuthenticatedUser`, `ApiError`, `AppState`).
- **Module(s):**
  - `mobile/lib/src/hub/` — `voice_hub_screen.dart`, `sergeant.dart`,
    `voice_intent.dart`, `voice_protocol.dart`, `speech_input.dart`,
    `voice_output.dart`, `voice_intent_service.dart`
  - `mobile/lib/src/workout/application/` — `voice_coach.dart`,
    `session_voice_intent.dart`
  - `mobile/lib/src/workout/domain/muscle_activation.dart`,
    `mobile/lib/src/workout/presentation/muscle_map.dart`,
    `mobile/lib/src/workout/presentation/preset_exercises.dart`
  - `mobile/lib/src/nutrition/domain/preset_meals.dart`,
    `mobile/lib/src/nutrition/models/food_info.dart`
  - `mobile/lib/src/shell/home_shell.dart` (mic entry point),
    `mobile/lib/src/router/app_router.dart` (`/hub` route)
  - `backend/crates/api/src/voice/` — `mod.rs`, `handlers.rs`, `parse.rs`
  - `backend/crates/api/src/nutrition/foods.rs` (USDA nutrient lookup)

> **Retro-spec note.** This spec was written *after* the implementation merged
> to `main` (PRs #39, #49, #50), reconciling the earlier draft in the still-open
> PR #37 with the code that actually shipped. Where the two diverge, this spec
> follows the shipped code; every divergence is recorded in the decision log
> (§7). It documents current behaviour rather than proposing new work.

## 1. Motivation

Realizes [R-0032](../requirements/0032-voice-assistant.md): remove the friction
of typing to log meals and workouts by making logging conversational and
hands-free. The shipped feature is a **voice hub** — a central speak button
ringed by every primary action — driven by a "sergeant" conversation loop that
prompts, listens, acts, and re-listens. Meals and workout sets can be logged by
voice alone; a mid-session **voice coach** turns a live workout into a
question-and-answer form ("done" → reps → kilos). Commands are terminated with a
military radio protocol ("over" / "out"). The intent pipeline is
client-first-with-a-backend-LLM-seam: the mobile client always has a local
keyword parser, and the backend `POST /voice/intent` endpoint upgrades that to an
LLM parse when an Anthropic key is configured.

R-0032's original second half — smart reminders for missing logs (former
AC6–AC8) — was **never implemented**. Under the R-0057 reconciliation the owner
**moved reminders out to [R-0036](../requirements/0036-voice-reminders.md)**;
this spec (and R-0032) now cover the voice-logging scope only. The former
AC6–AC8 are recorded in §6 as out-of-scope/moved, not as R-0032 gaps.

## 2. Design

### 2.1 Shape

```
mobile/hub
  VoiceHubScreen        /hub — central speak button + 6-option action ring +
                        agent chat thread. Consumes the sergeant's navigation.
  Sergeant (Notifier)   the hub conversation loop: prompt → listen → act →
                        re-listen. Backend intent first, local fallback.
  SpeechInput  (seam)   STT: PluginSpeechInput over `speech_to_text`
  VoiceOutput  (seam)   TTS: PluginVoiceOutput over `flutter_tts`
  voice_protocol        endsWithOver / stripOver / isOut — the radio protocol
  parseVoiceIntent      pure local keyword parser (sealed VoiceIntent)
  VoiceIntentService    POST /voice/intent client (VoiceIntentResult)

mobile/workout
  VoiceCoach (Notifier) in-session hands-free / guided-logging loop
  parseSessionVoiceIntent  pure in-session parser (sealed SessionVoiceIntent)
  muscle_activation     anatomy activations per lift (ported from goose-factor)
  MuscleMap             anatomy chart widget (TARGET MUSCLES card + picker preview)
  presetExercises/Meals preset lift + meal libraries

backend/api
  POST /voice/intent    transcript → structured IntentResponse; auto-logs
  voice::parse          keyword parser (CI-safe) + optional Claude LLM parse
  GET /nutrition/foods  USDA FoodData Central proxy → macros per 100 g
```

The client is thin at the transport layer but **not** a pure display client for
this feature: STT and TTS run on-device (via `speech_to_text` / `flutter_tts`),
and a local keyword parser guarantees the hub stays usable offline. The
**intelligence** (LLM intent parsing, persistence) lives server-side, honouring
the project's "intelligence is server-side" domain note.

### 2.2 The radio protocol (`hub/voice_protocol.dart`)

Three pure functions, shared by the sergeant and the coach:

- `endsWithOver(t)` — true when a (partial) transcript ends with the "over"
  terminator (`[,.!\s]*\bover\b[.!]?\s*$`, case-insensitive). Ending a command
  with "over" fires it **instantly**, without waiting for the STT engine's
  silence timeout.
- `stripOver(t)` — removes a trailing "over" and returns the bare command.
- `isOut(t)` — true when the transcript contains the word "out" (word-boundary)
  but **not** "workout" — so "workout" never accidentally signs off. "out" ends
  the whole conversation.

### 2.3 The Sergeant — hub conversation loop (`hub/sergeant.dart`)

`Sergeant` is a Riverpod `Notifier<SergeantState>`. `SergeantState` carries
`conversing`, `listening`, `transcript`, `line` (last spoken line), `history`
(chat bubbles, capped at 12), `awaitingMacros`, and `navigateTo` (a navigation
effect the screen consumes — the notifier holds no `BuildContext`).

Loop (`start` → `_listenOnce` → `_handle` → re-listen):

1. `start()` initializes `VoiceOutput`; if `SpeechInput.initialize()` fails it
   sets a "voice not available — tap an option instead" line and returns.
2. Announces the prompt ("Sergeant here. Say: start workout, plan workout, log a
   meal. Finish every command with over.") and calls `_listenOnce`.
3. `_listenOnce` streams partial transcripts. A command fires when the STT
   result is final **or** ends with "over"; "over" mid-stream stops the engine
   immediately (`stripOver` yields the bare command).
4. `_handle` routes: `isOut` → sign off; `awaitingMacros` → macro follow-up;
   otherwise **try the backend** (`VoiceIntentService.parse`) and, on any error,
   fall back to `_handleLocal`.
5. Idle guard: up to `_maxIdleRounds = 3` fruitless (silent/unknown) rounds
   re-arm the mic quietly, then the sergeant stands by without overwriting the
   last useful line.

**Backend result handling** (`_handleBackendResult`) maps the wire `status`:
`logged_nutrition` / `logged_workout` → speak the message and keep listening;
`clarify` → set `awaitingMacros` and speak the prompt; `navigate` → speak, set
`navigateTo`, and (for `/session`) start the `SessionDriver` and enable the
`VoiceCoach`; anything else → speak a fallback and keep listening.

**Local fallback** (`_handleLocal`) covers the offline path with richer
behaviour than the backend: named preset meals ("log a protein shake"), macro
dictation, USDA portion lookup ("200 grams of chicken breast"), and the same
navigation intents as the backend.

**"Activate again after each instruction" (R-0032 UX intent):** in the hub the
sergeant **re-listens automatically** after logging actions (the loop keeps the
mic open until a navigation/sign-off ends it), so the user does not re-key the
mic between spoken commands. Push-to-talk between *sets* applies only in the
in-session coach's non-hands-free mode (§2.4). See decision log.

### 2.4 The Voice Coach — in-session guided logging (`workout/application/voice_coach.dart`)

`VoiceCoach` is a `Notifier<VoiceCoachState>` gluing `SpeechInput`,
`VoiceOutput`, and the widget-independent `SessionDriver`. `enable({handsFree})`
preloads the active program's highlight exercises into an empty session
(`currentProgramProvider`), announces the first lift and its target muscles, and
— in `handsFree` mode — starts a self-re-arming listen loop.

- **Guided set logging** (`_answer`, `_Pending` state machine): "done" (a
  `SetDoneIntent`) starts the questions — "How many reps?" → "How many kilos? Or
  say bodyweight." — filling a `SetDraft` like a form, then logging via
  `SessionDriver.logSet`. Partial dictation ("100 kilos" alone) switches into
  the same guided questions for the missing field.
- **Direct dictation**: `parseSessionVoiceIntent` extracts reps / weight / RPE
  from "10 reps at 100 kilos RPE 8" and logs in one shot. Bare numbers fall back
  to reps-then-weight.
- **Navigation of the session**: `NextExerciseIntent`, `FinishSessionIntent`
  (needs the word "workout"/"session" so a bare "done" stays set-scoped),
  `PauseSessionIntent`. Driver rejection strings are spoken verbatim (they are
  the user-safe reasons by contract).
- **Push-to-talk vs hands-free**: in non-hands-free mode the user keys the mic
  per set between rests (the comment notes "later: earbud/watch button");
  hands-free re-arms after each response until paused, finished, or silent
  `_maxSilences = 3` times.

### 2.5 Local intent parsers (pure)

- `hub/voice_intent.dart` — `parseVoiceIntent` → sealed `VoiceIntent`
  (`LogWorkoutIntent`, `LogMealIntent`, `ShowProgramIntent`, `BodyMatchIntent`,
  `ShowHistoryIntent`, `ShowProfileIntent`, `StopIntent`, `UnknownIntent`).
  First-match order is meal → plan/program → workout → body match → history →
  profile. Helpers `parseFoodQuantity` ("N grams of X") and `parseMacros`
  ("40 protein, 60 carbs, 20 fat").
- `workout/application/session_voice_intent.dart` — `parseSessionVoiceIntent` →
  sealed `SessionVoiceIntent` (`LogSetIntent`, `NextExerciseIntent`,
  `FinishSessionIntent`, `SetDoneIntent`, `PauseSessionIntent`,
  `UnknownSessionIntent`).

These are the "seam that stays": the backend LLM parser (§2.6) is an upgrade
layered *in front of* them, not a replacement — the sealed types remain the
offline contract.

### 2.6 Backend intent pipeline (`backend/crates/api/src/voice/`)

**`POST /voice/intent`** (authenticated) — request `{ "transcript": String }`.

`AppState.voice: VoiceIntentSettings` carries an optional
`anthropic_api_key: Option<Arc<str>>` (from `ANTHROPIC_API_KEY` via
`from_env`) and a shared `reqwest::Client`.

Handler (`handlers::intent`):
1. `today = Utc::now().date_naive()`.
2. If a key is set: `parse::parse_with_llm(...).await` and, on **any** error,
   `unwrap_or_else` to `parse::parse_transcript(...)` — the keyword parser is
   the guaranteed fallback. No key → keyword parser directly.
3. Match the `ParsedAction`:
   - `Nutrition(NewNutritionLog)` → `db::insert_nutrition_log` →
     `IntentResponse::logged_nutrition(id, message)`.
   - `Workout(NewWorkoutSession)` → `db::insert_session` →
     `IntentResponse::logged_workout(id, summary)`.
   - `Response(IntentResponse)` → returned as-is (clarify / navigate / unknown).

**Wire response `IntentResponse`** (`snake_case` `status`, `None` fields
skipped): `status ∈ { logged_nutrition, logged_workout, clarify, navigate,
unknown }`, plus optional `message`, `prompt`, `route`, `record_id`.

**Keyword parser (`parse::parse_transcript`)** — CI-safe, no network. Mirrors the
mobile parser and adds workout-set extraction:
- `parse_workout_set` — "10 reps of 100 kg bench press" → `NewWorkoutSession`
  (regex reps + kg; `extract_exercise_name` recognises bench/squat/deadlift,
  else strips units and uses the remainder).
- meal keywords + `grams(...)` for protein/carbs/fat → `NewNutritionLog`, else a
  `clarify` prompt.
- navigation keywords → `navigate` responses (`/session`, `/programs/current`,
  `/programs/get`, `/home`, `/onboarding`).

**LLM parser (`parse::parse_with_llm`)** — posts to
`https://api.anthropic.com/v1/messages` (`model: claude-haiku-4-5-20251001`,
`max_tokens: 256`, 5 s timeout) with a JSON-only prompt enumerating the five
actions (`log_workout` / `log_meal` / `clarify` / `navigate` / `unknown`). Any
non-2xx, 429, timeout, or unparseable body → `ApiError::Upstream`, which the
handler swallows into the keyword fallback. `llm_json_to_action` maps the parsed
JSON to a `ParsedAction`, re-validating through the same `New*` constructors so
the LLM can never bypass domain validation.

**Mobile client (`hub/voice_intent_service.dart`)** — `VoiceIntentService.parse`
POSTs `{transcript}` to `/voice/intent`, deserializes `VoiceIntentResult`
(`isLoggedNutrition` / `isLoggedWorkout` / `isClarify` / `isNavigate`), and
throws `ApiException.fromDio` on transport error (which the sergeant catches to
fall back locally).

### 2.7 USDA nutrient lookup (`backend/crates/api/src/nutrition/foods.rs`)

**`GET /nutrition/foods?q=…`** (authenticated) proxies USDA FoodData Central
search (`FDC_API_KEY`, falling back to the rate-limited `DEMO_KEY`), reducing the
payload to per-100 g macros (`FoodMacros { name, protein_g_per_100g,
carbs_g_per_100g, fat_g_per_100g, kcal_per_100g }`) via the pure, unit-tested
`parse_fdc_response`. The sergeant's `_logFood` calls this (client model
`FoodInfo`), scales by `grams/100`, and logs the derived meal.

### 2.8 Anatomy chart & presets

- `muscle_activation.dart` — `Region` enum + curated `activationFor(name)`
  returning primary/secondary regions (ported from the-goose-factor MuscleMap);
  unknown lifts fall back to their coarse `MuscleGroup`.
- `muscle_map.dart` — the anatomy widget; rendered as a "TARGET MUSCLES —
  `<EXERCISE>`" card in `live_session_screen.dart` and as a live preview in the
  exercise picker (primary movers olive, assisters brass).
- `preset_exercises.dart` (~72 lifts) and `preset_meals.dart` (14 meals, with
  `matchPresetMeal` fuzzy matching) back both tap and voice logging.

### 2.9 Entry point & routing

- `home_shell.dart` — a `Icons.mic` AppBar action (tooltip "Voice hub")
  navigating to `/hub`.
- `app_router.dart` — `GoRoute('/hub') → VoiceHubScreen`.

## 3. Code outline

Representative — the shipped handler seam and the sergeant's route decision.

```rust
// backend/crates/api/src/voice/handlers.rs
pub(crate) async fn intent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    req: Result<Json<IntentRequest>, JsonRejection>,
) -> ApiResult<Json<IntentResponse>> {
    let Json(req) = req.map_err(|_| ApiError::Validation { field: "body" })?;
    let today = Utc::now().date_naive();
    let action = if let Some(key) = state.voice.anthropic_api_key.as_deref() {
        parse::parse_with_llm(&state.voice.http, key, &req.transcript, today)
            .await
            .unwrap_or_else(|_| parse::parse_transcript(&req.transcript, today))
    } else {
        parse::parse_transcript(&req.transcript, today)
    };
    let response = match action {
        ParsedAction::Nutrition(new) => { /* insert_nutrition_log → logged_nutrition */ }
        ParsedAction::Workout(new)   => { /* insert_session → logged_workout */ }
        ParsedAction::Response(r)    => r,
    };
    Ok(Json(response))
}
```

```dart
// mobile/lib/src/hub/sergeant.dart — backend first, local fallback
Future<bool> _handle(String transcript) async {
  if (isOut(transcript)) { await _say('Roger. Out.'); /* stop */ return false; }
  if (state.awaitingMacros) return _handleMacros(transcript);
  try {
    final result = await ref.read(voiceIntentServiceProvider).parse(transcript);
    return _handleBackendResult(result);
  } catch (_) { /* offline / upstream failure */ }
  return _handleLocal(transcript);
}
```

## 4. Non-goals

- **General conversational chatbot** — the assistant is constrained to
  fitness/nutrition logging and navigation (R-0032 §4). The LLM prompt returns
  JSON only, one of five fixed actions.
- **On-device LLM inference** — intent parsing is a local keyword parser or a
  server-side LLM; no on-device model.
- **Backend audio upload / server-side STT** — STT runs on-device via
  `speech_to_text`; only the **transcript** reaches the backend, never audio.
  (Divergence from PR #37 — see §7.)
- **Always-on listening** — the mic is only active while the user is in a
  conversation started by tapping the speak button (R-0032 §4).

## 5. Open questions

Resolved as shipped:

| OQ (R-0032) | Resolution as shipped |
|---|---|
| OQ-1 STT engine | On-device `speech_to_text` (Web Speech API in browser) behind the `SpeechInput` seam. No cloud STT / Whisper. |
| OQ-2 LLM prompt | JSON-only prompt, five fixed actions, `claude-haiku-4-5`; keyword parser is the always-present fallback (`parse.rs`). |
| OQ-3 daily-routine model | **Moved to R-0036.** The reminder routine model is no longer in R-0032's scope. |
| OQ-4 cron service | **Moved to R-0036.** |

## 6. Acceptance criteria

Mapping each R-0032 acceptance criterion to the shipped code and its covering
test. Status: **MET** / **PARTIAL** / **GAP**.

- [x] **AC1 — Voice Input Button (MET).** Mic action in `home_shell.dart:48`
  → `/hub`; hub speak button in `voice_hub_screen.dart` (`_SpeakButton`).
  *Test:* `voice_hub_screen_test.dart` "ring renders all six options and the
  mic"; `home_shell_test.dart`. *Caveat:* the button lives on Home and inside
  the hub, not literally overlaid on *every* primary screen — see §7.
- [x] **AC2 — Speech-to-Text (MET).** `SpeechInput` seam + `PluginSpeechInput`
  over `speech_to_text` (`hub/speech_input.dart`); partial + final results feed
  the loop. *Test:* `FakeSpeechInput` (`test/support/voice_fakes.dart`) drives
  every hub/coach widget test.
- [x] **AC3 — Intent Parsing / LLM (MET).** `POST /voice/intent` →
  `parse_with_llm` (Claude) with `parse_transcript` fallback (`voice/parse.rs`);
  client `VoiceIntentService`. *Test:* `voice_intent.rs`
  ("voice_intent_logs_workout_from_natural_language",
  "voice_intent_logs_meal_when_macros_present"), `parse.rs` unit tests,
  `voice_intent_service_test.dart`. *Note:* LLM is **optional** (key-gated);
  keyword parser always present.
- [x] **AC4 — Automatic Logging (MET).** Handler inserts via
  `db::insert_nutrition_log` / `db::insert_session` (`handlers.rs:59-88`);
  sergeant's local path logs via `NutritionService`. *Test:* `voice_intent.rs`
  asserts DB rows; `voice_hub_screen_test.dart` "a dictated meal with macros logs
  directly".
- [x] **AC5 — Confirmation / Fallback (MET).** `clarify` status +
  `awaitingMacros` follow-up (`sergeant.dart:_handleMacros`; `parse.rs`
  clarify). *Test:* `voice_intent.rs` "voice_intent_clarifies_incomplete_meal";
  `voice_hub_screen_test.dart` "'log a meal' without macros asks, then logs the
  follow-up"; `parse.rs` "meal_without_macros_clarifies".
- **Former AC6–AC8 — Reminders (MOVED to R-0036).** Missing-log evaluation,
  alert notifications, and voice-activation-from-notification were never built
  and are no longer part of R-0032; they are tracked in
  [R-0036](../requirements/0036-voice-reminders.md) (SPEC-0036, to be written).
  Not counted as R-0032 gaps.
- [x] **AC6 — Tests (MET).** Flutter widget tests for the
  mic + listening states (`voice_hub_screen_test.dart`, 13 cases;
  `voice_coach_test.dart`, 10 cases; `voice_protocol_test.dart`, 6 cases).
  Backend tests for prompt construction / record creation from structured
  output (`voice_intent.rs`, `parse.rs`, `foods.rs`).
- [x] **AC7 — Privacy & Scope Guard (MET by construction).** No audio is
  stored: STT is on-device and only the transcript is transmitted; the backend
  persists only structured logs (`voice/` never writes an audio blob) and
  exposes no audio-upload endpoint. The 5-action JSON-only prompt keeps it from
  being a general chatbot, and the mic is user-initiated (no always-on listener).
  Holds by construction; a dedicated non-storage/out-of-scope guard test is a
  recommended future addition (QA note, R-0057).

**Scope summary:** R-0032 shipped its voice-logging scope (AC1–AC7). The former
smart-reminders half (original AC6–AC8) was never built and is now R-0036 — it
remains
open work; this spec records that explicitly rather than implying coverage.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-03 | **On-device STT, transcript-only to backend** (not PR #37's "multipart audio → backend Whisper"). | Keeps audio off the wire (AC10 by construction), avoids a cloud-STT dependency, and works offline behind the `SpeechInput` seam. Diverges from #37 §2.2. |
| 2026-07-03 | **Anthropic Claude (`claude-haiku-4-5`), not "e.g. Claude/OpenAI".** | Cheap, fast JSON extraction; key-gated so CI and offline both fall back to the keyword parser. #37 left the provider open. |
| 2026-07-03 | **Client-first-with-backend-seam pipeline.** The mobile keyword parser is permanent; the backend LLM is an upgrade layered in front, and the handler falls back to a keyword parser too. | The hub stays fully usable offline / on LLM failure; the LLM never becomes a hard dependency. Neither #37 nor R-0032 specified this dual fallback. |
| 2026-07-03 | **"over"/"out" radio protocol** terminates commands instantly. | UX choice not in R-0032/#37; lets a command fire without waiting for the STT silence timeout, and gives a clear conversation-end word (`voice_protocol.dart`). |
| 2026-07-03 | **Voice hub screen + action ring** as the home for voice, plus a mic entry on Home. | R-0032/AC1 asks for a "persistent mic accessible across primary screens"; shipped as a dedicated `/hub` reachable from the Home AppBar rather than an overlay on every screen. Documented as an AC1 caveat. |
| 2026-07-03 | **In-session voice coach with guided Q&A** ("done" → reps → kilos). | Realizes the hands-free-logging spirit of R-0032 for live workouts, sharing the R-0027 `SessionDriver`/seams. Beyond #37's scope. |
| 2026-07-03 | **USDA FoodData Central lookup** for "N grams of X". | Turns natural portions into macros without the user knowing them; pure parser is unit-tested (`foods.rs`). Beyond #37's scope. |
| 2026-07-06 | **Reminders split out to R-0036** (were the original R-0032 AC6–AC8). | No cron/notification code shipped in #39/#49/#50; rather than leave R-0032 permanently unmet, the reminder scope became its own requirement (R-0036), seeded by #37's §2.3 sketch. R-0032 is now scoped to voice logging only. |
| 2026-07-03 | **LLM output re-validated through `New*` constructors.** | The LLM cannot bypass domain validation; malformed JSON degrades to `ApiError::Upstream` → keyword fallback (`parse.rs::llm_json_to_action`). |

## Changelog

- _2026-07-03 — created as a retro-spec (R-0057) reconciling PR #37's draft with
  the implementation merged via PRs #39, #49, #50. Status set directly to
  Accepted to match shipped `main`._
- _2026-07-06 — scoped to voice logging (AC1–AC7); reminders split to R-0036;
  §6 renumbered to the amended 7-AC set (architect review)._
