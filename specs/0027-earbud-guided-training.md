# SPEC-0027 — Earbud-guided training

- **Status:** Accepted (as-built — see §0)
- **Realizes:** R-0027
- **Author:** Claude (main session), with owner
- **Created:** 2026-06-21
- **Audited:** 2026-07-03 (R-0057) — reconciled to what merged to `main`.
- **Depends on:** R-0009 (SessionDriver), R-0014 (UserProgram)
- **Module(s) (as-built on `main`):**
  `mobile/lib/src/workout/application/voice_coach.dart` (the coach notifier)
  `mobile/lib/src/workout/application/session_voice_intent.dart` (dictation parser)
  `mobile/lib/src/hub/voice_output.dart` (TTS seam — `flutter_tts`)
  `mobile/lib/src/hub/speech_input.dart` (speech-recognition seam — `speech_to_text`)
  `mobile/lib/src/workout/presentation/live_session_screen.dart` (toggle + coach bar)
  `mobile/pubspec.yaml`

## 0. As-built note (audit R-0057, 2026-07-03)

**This spec has been reconciled to what actually merged to `main`.** The
original SPEC-0027 (drafted 2026-06-21, first realized in PR #48) specified an
`audio_service`-based **media-button / background-audio transport** with a
voice-**out-only** `VoiceSessionAdapter`. That design was merged in PR #48 but
was **subsequently dropped from `main`** by the R-0030 merge
(commit `6b5bb7e`, "merge main into R-0030 (prefer local): resolve voice stack
conflict"), which deleted `voice_session_adapter.dart`,
`voice_session_audio_handler.dart`, and their tests, and removed the
`audio_service` dependency, in favour of the R-0032 **`VoiceCoach`** stack.

The feature that ships on `main` today is therefore a **dictation-driven voice
coach**, not the earbud/media-button transport R-0027 describes. Sections §2–§10
below document the as-built `VoiceCoach`. The **material divergences from
R-0027's acceptance criteria are recorded honestly in §11** so QA can judge
sign-off against reality rather than against the superseded #48 design. Several
of R-0027's distinctive contracts (single-media-button advance, background
audio while pocketed, voice-**out-only** with no speech recognition) are **not
met on `main`**; see §11.

## 1. Overview

R-0027 targets hands-free in-gym guidance built on the widget-independent
`SessionDriver` (R-0009) and reading from the active `UserProgram` (R-0014).

As shipped on `main`, the orchestrator is `VoiceCoach`
(`voice_coach.dart`), a Riverpod `Notifier<VoiceCoachState>`. It is turned on by
a toggle on `LiveSessionScreen` (`headset_mic` icon, "Voice coach on/off"). On
enable it initialises TTS via `VoiceOutput` (`flutter_tts`), preloads the active
program's `highlightExercises` into an empty session (closing the
program → narrated-training loop), and speaks what is first. The user then
**dictates** sets by voice — `SpeechInput` (`speech_to_text`) captures "10 reps
at 60 kilos", "done", "next", "finish workout"; `parseSessionVoiceIntent`
converts the transcript to a typed intent; `VoiceCoach` applies it through the
existing `SessionDriver` and speaks the outcome.

This dictation model is a **deliberate substitution** for R-0027's original
media-button transport, inherited from the R-0032 voice hub. It means R-0027's
voice-out-only constraint and single-media-button contract are **not** honoured
as written (§11).

## 2. Flutter architecture (as-built)

`VoiceCoach` (`voice_coach.dart`) is the orchestrator.

- **Collaborators**: `VoiceCoach` ↔ `SessionDriver` (Riverpod
  `sessionDriverProvider`) ↔ `VoiceOutput`/`flutter_tts` (out) ↔
  `SpeechInput`/`speech_to_text` (in) ↔ `currentProgramProvider` (plan preload).
- **State**: `VoiceCoachState { enabled, handsFree, listening, transcript,
  coachLine }` — transient, not persisted across sessions.
- **Lifecycle**:
  1. Toggle off → user taps the `headset_mic` toggle on `LiveSessionScreen` →
     `VoiceCoach.enable()`.
  2. `enable()`: initialises TTS; if the session is empty, preloads the active
     program's `highlightExercises`, selects the first, and announces it; else
     announces the current exercise. Prompts the user to dictate.
  3. Listening: the mic button in `_CoachBar` (or the hands-free loop) calls
     `dictate()` → `speech_to_text` transcribes.
  4. Applying: `parseSessionVoiceIntent(transcript)` → intent →
     `_apply(...)` calls `SessionDriver.logSet` / `selectExercise` / `finish`.
     Partial dictation switches to **guided questions** (reps → kilos).
  5. Speaking the outcome: "Logged … Rest up.", "Next up: …", "Workout saved."
  6. Toggle on → `VoiceCoach.disable()` stops speech both directions and clears
     state.
- **Hands-free mode**: `enable(handsFree: true)` re-arms listening after each
  response (tolerating `_maxSilences` silent listens) so the whole session can
  run by voice without touching the screen.

## 3. TTS script (as-built strings)

The as-built coach lines are conversational and **differ from R-0027's exact
scripts** (which QA should note, §11). Variables come from `SessionDriver` /
the active program; the weight clause is omitted when weight is absent.

- **Plan loaded / first exercise**: "Plan loaded — {N} exercises. First up:
  {name}[ — target {muscle}]. {prompt}"
- **Enable, session already populated**: "Voice coach on. Current exercise:
  {name}. {prompt}"
- **Set logged**: "Logged {R} reps[ at {W} kilos]. Rest up[ — say done when the
  next set is in]."
- **Next exercise**: "Next up: {name}[ — target {muscle}]."
- **Last exercise**: "That was the last exercise. Say finish workout to save."
- **Finish**: "Saving your workout." → "Workout saved. Dismissed!"
- **Guided questions**: "How many reps?", "How many kilos? Or say bodyweight."

## 4. Voice transport (as-built)

There is **no `audio_service` and no media-button handler on `main`**. The
transport is:

- **Voice out**: `VoiceOutput` abstraction, `PluginVoiceOutput` over
  `flutter_tts` (`voice_output.dart`). `speak()`/`stop()`/`initialize()` all
  swallow engine errors so the session continues silently (R-0027 AC11 spirit).
- **Voice in**: `SpeechInput` over `speech_to_text` (`speech_input.dart`).
  Dictation is the advance mechanism — the user says "done"/"next"/"finish",
  **not** a headset media-button press. R-0027's `MPRemoteCommandCenter` /
  `KEYCODE_HEADSETHOOK` handling is **not implemented** (§11, AC4).
- **Interrupt model**: `stop()` on both seams; a new utterance follows the
  applied intent.

## 5. iOS platform config (as-built)

**Not configured for background audio on `main`.** `Info.plist` contains **no
`UIBackgroundModes`/`audio`** entry and no `AVAudioSession` `playback` category
setup. TTS therefore has no guarantee of continuing with the screen locked
(§11, AC7). Microphone/speech-recognition usage strings are present for
`speech_to_text`.

## 6. Android platform config (as-built)

**No foreground service and no `audio_service` on `main`.**
`AndroidManifest.xml` declares `RECORD_AUDIO`, the `RecognitionService` and
`TTS_SERVICE` queries (for `speech_to_text` / `flutter_tts`), but **no**
`FOREGROUND_SERVICE` / `FOREGROUND_SERVICE_MEDIA_PLAYBACK` permission and **no**
foreground-service component. Background survival while pocketed is not
guaranteed (§11, AC7).

## 7. Error handling (as-built)

- **TTS engine unavailable / speak failure**: `PluginVoiceOutput` catches and
  swallows; the session continues silently. Matches AC11.
- **Speech-input unavailable**: `_listenOnce` speaks "Voice input is not
  available here." and returns without crashing.
- **Driver rejection**: `SessionDriver.logSet` rejection strings are spoken
  verbatim (user-safe by contract) and the coach re-arms.
- **Earbud connect/disconnect**: **Not handled** — there is no media-button
  handler to release/re-register (§11, AC12).

## 8. pubspec.yaml (as-built)

- `flutter_tts: ^4.2.0` (voice out).
- `speech_to_text: ^7.0.0` (voice in — **added by the R-0032 stack; R-0027
  forbade speech recognition**, §11).
- **`audio_service` is absent** — the R-0030 merge removed it.

## 9. Tests (as-built)

- `mobile/test/workout/application/voice_coach_test.dart` — plan preload +
  announcements, dictated set logging, guided questions, finish flow, using
  fakes in `mobile/test/support/voice_fakes.dart`.
- `mobile/test/workout/application/session_voice_intent_test.dart` — the
  dictation parser.
- `mobile/test/workout/presentation/live_session_screen_test.dart` — the toggle
  and coach bar.
- **No test exists** for a media-button advance callback, background-audio
  keep-alive, or earbud disconnect/reconnect — because that transport is not on
  `main` (§11, AC13).

## 10. Acceptance criteria mapping (as-built)

See §11 for the honest MET/PARTIAL/GAP verdict. Section pointers:

- **AC1** (Voice mode toggle) -> §2 (toggle + `VoiceCoachState`)
- **AC2/AC3** (Exercise/set announcement) -> §3 (**strings differ**)
- **AC4** (Earbud media-button advance) -> **GAP**, §4/§11 (dictation instead)
- **AC5** (Rest cue) -> §3 ("Rest up." embedded in the logged line)
- **AC6** (Next-exercise / done) -> §2/§3
- **AC7** (Background audio) -> **GAP**, §5/§6/§11
- **AC8** (Screen-on fallback) -> §2 (mic + on-screen both advance)
- **AC9** (Mid-session toggle) -> §2 (`enable`/`disable`)
- **AC10** (Program-aware narration) -> §1/§2 (plan preload)
- **AC11** (TTS failure resilience) -> §4/§7
- **AC12** (Earbud disconnect resilience) -> **GAP**, §7/§11
- **AC13** (Tests) -> §9 (**earbud/background paths untested**)
- **AC14** (Scope guard) -> §1 (Flutter-only; **but speech recognition added**, §11)

## 11. Divergence register (audit R-0057)

Honest reconciliation of R-0027's acceptance criteria against `main`. This is
the input QA should judge sign-off against.

| AC | R-0027 requires | On `main` | Verdict |
|----|-----------------|-----------|---------|
| AC1 | Voice-mode toggle on `LiveSessionScreen`, session-local | `headset_mic` toggle → `VoiceCoach.enable/disable`, transient state | **MET** |
| AC2 | *"Next: [name]. [N] sets of [R] reps at [W] kg."* | Announces next exercise + target muscle, but **different wording** and no explicit "N sets of R reps at W kg" from a program set-plan | **PARTIAL** |
| AC3 | Pre-set *"Set [X] of [N]. [R] reps at [W] kg. Go."* | No per-set pre-announcement; the coach speaks **after** a set is logged | **GAP** |
| AC4 | Single **media-button** press advances (iOS `MPRemoteCommandCenter` / Android `KEYCODE_HEADSETHOOK`) | **No media-button handler.** Advance is by **spoken dictation** ("done"/"next") | **GAP** |
| AC5 | *"Rest."* after each logged set | "…Rest up." embedded in the logged-set line | **PARTIAL** (met in spirit) |
| AC6 | Next-exercise announce; "Workout done." at end | "Next up: …" / "That was the last exercise…" / "Workout saved." | **MET** (wording differs) |
| AC7 | Background audio, screen off: iOS `UIBackgroundModes audio`, Android foreground service, via `audio_service` | **No `audio_service`, no iOS background mode, no Android foreground service** | **GAP** |
| AC8 | Screen-on: earbud button **and** on-screen "Log set" both advance | On-screen logging + mic dictation both work; **no earbud button exists** | **PARTIAL** |
| AC9 | Mid-session toggle off stops TTS, releases handler, preserves state; on re-registers | `disable()` stops both speech seams and clears coach state; session data preserved by `SessionDriver`. No media-button handler to release | **PARTIAL** |
| AC10 | Program-aware narration from active `UserProgram`; free-form fallback | `enable()` preloads `highlightExercises`; free-form when no plan | **MET** |
| AC11 | TTS failure → continue silently, no crash | `PluginVoiceOutput` swallows all TTS errors | **MET** |
| AC12 | Earbud disconnect releases handler; reconnect re-registers | **No media-button handler** to manage | **GAP** |
| AC13 | Widget/unit tests for toggle, button-callback == screen-advance, mocked TTS, TTS-failure | Coach/parser/screen tested; **no earbud-button or background-audio tests** | **PARTIAL** |
| AC14 | Flutter-only, no backend/db changes, **no speech recognition, no voice commands** | No backend/db changes — **but `speech_to_text` and voice commands were added**, directly contradicting the R-0027 scope guard | **GAP** (scope violated) |

**Bottom line for QA:** the R-0032 voice-coach substitution delivers a
hands-free-ish experience but does **not** implement R-0027's defining
contract (earbud media-button advance + guaranteed background audio while
pocketed) and **violates R-0027's explicit "no speech recognition, no voice
commands" scope guard**. Sign-off on R-0027 as written is **not** supportable
without either (a) building the media-button/background-audio transport, or
(b) the owner amending R-0027's acceptance criteria to bless the dictation
model.

## 12. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-21 | `flutter_tts` (out) + `audio_service` (media-button + background). | Original SPEC-0027 design, realized in PR #48. |
| ~2026-06 | **R-0030 merge dropped the `audio_service` `VoiceSessionAdapter`** in favour of the R-0032 `VoiceCoach` (dictation) stack. | "resolve voice stack conflict" (commit `6b5bb7e`); the two voice stacks collided and the local VoiceCoach was preferred. `audio_service` removed; `speech_to_text` added. |
| 2026-07-03 | **Audit (R-0057): spec reconciled to as-built `VoiceCoach`, Status `Accepted`, divergences from R-0027 recorded in §11.** | R-0027 shipped without QA sign-off; the spec must describe reality before QA can judge it. The earbud/background/voice-out-only contracts are flagged GAP, not rubber-stamped. |

## Changelog

- _2026-06-21 — created (Draft). Original `audio_service` media-button design (PR #48)._
- _2026-07-03 — **audited against merged #48 and current `main` (R-0057).**
  Spec had **drifted**: PR #48's `audio_service`/`VoiceSessionAdapter` transport
  was later removed from `main` by the R-0030 merge and replaced by the R-0032
  `VoiceCoach` dictation stack. Rewrote §§1–10 to describe the as-built coach,
  added §0 as-built note, §11 divergence register, and §12 decision log. Status
  moved Draft → Accepted (as-built). Material GAPs against R-0027 (AC3, AC4, AC7,
  AC12, AC14) recorded honestly for QA._
