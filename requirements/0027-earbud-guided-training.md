# R-0027 — Earbud-guided training

- **Status:** Regressed — superseded by [R-0035](0035-earbud-handsfree-training.md) (2026-07-06, R-0057)
- **Milestone:** M3 (fast-track differentiator)
- **Owner:** see [`project-specifics.md`](../project-specifics.md)
- **Created:** 2026-06-21
- **Depends on:** R-0009 (Done — `SessionDriver` seam this plugs into),
  R-0014 (Done — `UserProgram` row this reads for the exercise plan)
- **Realized by:** [SPEC-0027](../specs/0027-earbud-guided-training.md) (as-built audit; documents the regression)
- **QA:** `qa` agent run scoped to this requirement

> **Regression note (R-0057, 2026-07-06).** The media-button + background-audio
> transport this requirement specifies was built (PR #48) then **reverted** by
> conflict-resolution commit `6b5bb7e` and replaced with the R-0032 voice-
> **dictation** coach — which violates this requirement's AC14 scope guard ("no
> speech recognition"). The hands-free earbud transport is being **rebuilt under
> [R-0035](0035-earbud-handsfree-training.md)** (coexisting with the R-0032
> dictation mode). SPEC-0027 has been updated to document the as-built regression;
> this requirement is retained for history and is not independently signed off.

## 1. Statement

During a workout session the app **narrates the session aloud through earbuds**:
the next exercise, set number, target weight/reps, and a rest cue. A **single
earbud media-button press** marks the current set done and advances to the
next — identical to tapping "Log set" on screen. The phone stays pocketed and
the screen can be off throughout.

Voice mode is a **toggle on `LiveSessionScreen`**. Turning it on activates TTS
+ button-advance for the running session; turning it off mid-session returns to
screen-only mode without losing state.

v1 is **voice-out only** — no speech recognition, no voice commands. The only
input is the single media-button press.

## 2. Rationale

The owner's stated differentiator is hands-free in-gym guidance: competitors
are screen-first trackers; this app speaks the session so the phone stays
pocketed. R-0009's `SessionDriver` was deliberately designed as the seam this
plugs into — R-0027 is the payoff of that investment.

Reading from the active `UserProgram` (R-0014) closes the loop from
photo → archetype → program → narrated training. When no program exists the
feature still works (free-form, speaks whatever exercises were added manually),
so voice mode degrades gracefully rather than being gated on onboarding
completion.

## 3. Acceptance criteria

- **AC1. Voice mode toggle.** `LiveSessionScreen` shows a voice/earbud icon
  button. Tapping it activates voice mode for the current session. Tapping again
  deactivates it. State is local to the session — not persisted across sessions.

- **AC2. Exercise announcement.** When voice mode is active and a new exercise
  begins (session start or transition from the previous exercise), TTS announces:
  *"Next: [exercise name]. [N] sets of [R] reps at [W] kg."*
  (If weight is not set, omit the weight clause.)

- **AC3. Set announcement.** Before each set, TTS announces:
  *"Set [X] of [N]. [R] reps at [W] kg. Go."*
  (or *"Set [X] of [N]. [R] reps. Go."* when no weight is set.)

- **AC4. Earbud button advance.** A single headset media-button press (iOS
  `MPRemoteCommandCenter` play/pause; Android `KEYCODE_HEADSETHOOK`) marks the
  current set as done and triggers the same state transition as tapping "Log set"
  on screen. The button is only active while voice mode is on.

- **AC5. Rest cue.** Immediately after a set is logged (button or screen), TTS
  announces: *"Rest."* (No countdown timer in v1.)

- **AC6. Next-exercise transition.** When all sets of an exercise are logged,
  TTS announces the next exercise (AC2). If there is no next exercise, TTS
  announces: *"Workout done."* and the session closes normally via the existing
  finish flow.

- **AC7. Background audio.** TTS speaks with the screen off / phone locked:
  - iOS: `AVAudioSession` category `playback` so audio continues in background.
  - Android: foreground service declared in `AndroidManifest.xml` keeps the
    process alive.
  Both platforms are handled via the `audio_service` package.

- **AC8. Screen-on fallback.** When the screen is on, both the earbud button
  and the on-screen "Log set" button work simultaneously — either advances the
  set. No UI elements are hidden when voice mode is active.

- **AC9. Mid-session toggle.** Turning voice mode off mid-session stops TTS
  immediately and releases the media-button handler. The session state (current
  exercise, logged sets) is unaffected. Turning voice mode back on re-registers
  the handler and resumes speaking from the current position.

- **AC10. Program-aware narration.** When the user has an active `UserProgram`
  (R-0014) and starts a session from that program's exercise list, TTS reads
  exercise names, set counts, rep targets, and weight targets from the program.
  When no program exists (free-form session), TTS reads whatever the user has
  added to the session manually.

- **AC11. TTS failure resilience.** If `flutter_tts` fails to speak (engine
  unavailable, audio focus denied), the session continues silently — no crash,
  no error dialog. A brief visual indicator may show that TTS is unavailable.

- **AC12. Earbud disconnect resilience.** If the earbud disconnects mid-session,
  the media-button handler is released. On-screen taps continue to advance sets
  normally. Reconnecting the earbud re-registers the handler.

- **AC13. Tests.** Flutter widget tests verify: voice-mode toggle activates/
  deactivates, earbud button callback triggers the same set-advance as screen
  tap, TTS method calls are verified via a mock, TTS failure is handled
  gracefully. Unit tests verify any voice-adapter logic added to `SessionDriver`.
  All lint and format gates green (`flutter analyze`, `dart format`, `flutter
  test`; Rust gates unchanged since no backend changes).

- **AC14. Scope guard.** R-0027 is Flutter-only — **no new backend endpoints,
  no new database migrations**. It reads `GET /programs/me/current` (already
  exists, R-0014) to load the active program and uses `POST /workouts` (R-0009)
  unchanged to log the session. No speech recognition, no ML, no program
  modification. No billing gates (M7).

## 4. Technology decisions

Settled in step-1 discussion (owner, 2026-06-21):

| Concern | Decision | Rationale |
|---------|----------|-----------|
| TTS engine | `flutter_tts` | Well-maintained, supports iOS + Android, background audio, simple API |
| Media-button + background audio | `audio_service` | Abstracts `MPRemoteCommandCenter` (iOS) and Android foreground service; established Flutter package for background audio use cases |
| Background audio requirement | **Hard requirement** (both platforms) | Phone staying pocketed is the core differentiator UX; silent fallback only on engine failure (AC11) |
| Set-advance gesture | Single press = log set + advance | Simplest contract; no double-press complexity in v1 |
| Voice mode entry | **Toggle on `LiveSessionScreen`** | Keeps the feature discoverable within the existing session flow; no separate screen or home shortcut in v1 |

## 5. Constraints & non-goals

- **No speech recognition** — voice-out only in v1.
- **No voice commands** — the only input is the media button (AC4).
- **No backend changes** — Flutter-only. Reads existing R-0014 and R-0009 APIs.
- **No countdown timer** — rest cue is "Rest." only; a timer is a v2 feature.
- **No per-set weight adjustment via voice** — the program's target weight is
  spoken; the user adjusts on screen if needed.
- **No billing/gating** — freemium is M7.

## 6. Open questions

Deferred to SPEC-0027:

- **OQ-H1 — `audio_service` integration depth:** does R-0027 wrap the full
  `AudioHandler` / `BaseAudioHandler` interface, or use `audio_service` only for
  the foreground service / media-button hook? (AC4/AC7)
- **OQ-H2 — TTS queue management:** does each announcement queue (non-
  interrupting) or interrupt the previous one? What happens if the user presses
  the button during a TTS utterance? (AC3/AC4)
- **OQ-H3 — `SessionDriver` extension surface:** which method/callback on the
  existing `SessionDriver` does the voice adapter hook — a listener, a wrapper,
  or a new method? (AC9/AC13)
- **OQ-H4 — iOS background entitlement:** does `flutter_tts` require
  `audio` background mode in `Info.plist`, or does `audio_service` handle the
  entitlement declaration? (AC7)
- **OQ-H5 — Android notification:** `audio_service` shows a media notification
  while the foreground service runs — what text/icon? (AC7)

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-21 | **`flutter_tts` for TTS.** | Well-maintained, cross-platform, background-capable. Owner confirmed. |
| 2026-06-21 | **`audio_service` for media-button + background audio.** | Abstracts both iOS and Android background audio; single package for two concerns. Owner confirmed. |
| 2026-06-21 | **Background audio is a hard requirement.** | The phone-pocketed UX is the differentiator; silent fallback only on engine error. Owner confirmed. |
| 2026-06-21 | **Single press = log set + advance.** | Simplest contract; no double-press in v1. Owner confirmed. |
| 2026-06-21 | **Voice mode toggle on `LiveSessionScreen`.** | Keeps feature in the existing session flow; no separate entry point in v1. Owner confirmed. |

## Changelog

- _2026-06-21 — created and **Accepted**. Step-1 discussion settled all five
  forks (TTS package, media-button package, background requirement, advance
  gesture, voice mode entry). Five HOW-level questions deferred to SPEC-0027._
