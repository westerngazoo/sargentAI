# SPEC-0027 — Earbud-guided training

- **Status:** Draft
- **Realizes:** R-0027
- **Author:** Claude (main session), with owner
- **Created:** 2026-06-21
- **Depends on:** R-0009 (SessionDriver), R-0014 (UserProgram)
- **Module(s):**
  `mobile/lib/src/workout/` (new voice session adapter, existing driver)
  `mobile/lib/src/audio/` (new audio service integration)
  `mobile/pubspec.yaml`
  `mobile/ios/Runner/Info.plist`
  `mobile/android/app/src/main/AndroidManifest.xml`

## 1. Overview

This spec realizes R-0027, the earbud-guided training feature. It builds directly on top of the widget-independent `SessionDriver` (R-0009) and reads from the active `UserProgram` (R-0014).

The architecture consists of a `VoiceSessionAdapter` (or similar) that acts as a middleman. It listens to the `SessionDriver` state changes, speaks the appropriate TTS scripts via `flutter_tts`, and registers a background keep-alive and media button hook via `audio_service` to advance the session without the screen on. The entire feature is toggled from the existing `LiveSessionScreen`.

## 2. Flutter architecture

The `VoiceSessionAdapter` is the orchestrator.

- **Class diagram**: `VoiceSessionAdapter` ↔ `SessionDriver` (Riverpod provider) ↔ `flutter_tts` ↔ `audio_service`.
- **State machine**:
  1. `voice mode off` → user taps toggle on `LiveSessionScreen` → `on`.
  2. `on`: Reads the current state from `SessionDriver` and speaks "Next: [name]...".
  3. `set announced`: Waits for user input (either on-screen tap or earbud media button).
  4. `button pressed`: Triggers `SessionDriver.logSet(...)`.
  5. `rest cued`: Adapter detects the set was logged, speaks "Rest."
  6. `next set`: Adapter detects the next set, speaks "Set [X] of [N]...".
  7. user taps toggle → `voice mode off`, resources released.
- **Toggle**: The toggle button on `LiveSessionScreen` updates a Riverpod provider (`voiceModeProvider`). `VoiceSessionAdapter` watches this provider to activate/deactivate. State is transient and un-persisted.

## 3. TTS script (exact strings)

All variables are drawn from the `SessionDriver`'s active exercise/set. The `weight` clause is optionally omitted if no target weight is present.

- **Exercise start**: "Next: {name}. {N} sets of {R} reps[ at {W} kg]."
- **Set start**: "Set {X} of {N}. {R} reps[ at {W} kg]. Go."
- **Rest**: "Rest."
- **Workout done**: "Workout done."
- **Rules**:
  - `[ at {W} kg]` is omitted entirely if weight is null or not set.
  - `{name}` is taken verbatim from the session's exercise name.

## 4. `audio_service` integration

To satisfy **OQ-H1**, R-0027 will implement a minimal `BaseAudioHandler` interface. We do not need full playback controls (next/prev track, seek, etc). We only need it to keep the background service alive and intercept `onMediaButton`.

- **Minimum implementation**: A custom `AudioHandler` extending `BaseAudioHandler`. Override `play()` or `pause()` to handle the single media button press. When the button is pressed, the handler calls a callback that triggers `SessionDriver.logSet()`.
- **iOS (`MPRemoteCommandCenter`)**: Handled automatically by `audio_service`.
- **Android (`KEYCODE_HEADSETHOOK`)**: Handled automatically by `audio_service` mapping to play/pause intents.
- **Keep active**: We simulate a playing state. `audio_service` requires us to set `PlaybackState(playing: true, controls: [MediaControl.pause])` (or similar) to keep the command center active with the screen off.

To satisfy **OQ-H2**, the TTS queue management will use **interrupting** behavior. If the user presses the earbud button during a TTS utterance, the current utterance is stopped, the set is logged, and the next appropriate utterance (e.g., "Rest.") immediately begins. This ensures the app is responsive.

To satisfy **OQ-H3**, the voice adapter will hook into the `SessionDriver` as a **listener** (`ref.listen(sessionDriverProvider, ...)`). It observes state changes to trigger TTS announcements, and it calls the existing `SessionDriver.logSet()` method to advance. No new dedicated wrapper or method is needed.

## 5. iOS platform config

To satisfy **OQ-H4**, we must declare background audio support. `flutter_tts` requires `audio` background mode, and `audio_service` manages the lifecycle.

- **`Info.plist`**: Add `UIBackgroundModes` array with the `audio` item.
- **`AVAudioSession` category**: The category must be set to `playback`. `audio_service` initializes this implicitly, but `flutter_tts` also provides `setIosAudioCategory(IosTextToSpeechAudioCategory.playback)`.
- **Entitlement requirement**: None specific beyond the `Info.plist` background mode.

## 6. Android platform config

To satisfy **OQ-H5**, we must configure the foreground service for `audio_service`.

- **`AndroidManifest.xml`**: Declare the `FOREGROUND_SERVICE` and `FOREGROUND_SERVICE_MEDIA_PLAYBACK` permissions. Declare the foreground service component as required by `audio_service`.
- **Notification channel**: `audio_service` requires a notification while running in the foreground.
  - Title: "FitAI Workout" (or similar app name).
  - Icon: A minimal dumbbell or app logo.
  - Importance level: `LOW` (no sound/vibration for the notification itself).
- **Android 14+ permissions**: Ensure `FOREGROUND_SERVICE_MEDIA_PLAYBACK` is explicitly requested or declared in the manifest.

## 7. Error handling

- **TTS engine unavailable**: If `flutter_tts` fails to initialize or speak, log the error and **silently continue**. Display a brief visual indicator (e.g., a toast or icon change) that TTS is unavailable.
- **Audio focus denied**: If audio focus cannot be obtained, **silently continue**.
- **Earbud disconnect**: If the earbud disconnects (or `audio_service` stops), release the media-button handler. On-screen taps will continue to work normally.
- **Reconnect**: If the user toggles voice mode off and on, or if the audio route changes back to a headset, re-register the handler.

## 8. pubspec.yaml additions

- `flutter_tts`: `^4.0.2` (or latest compatible version).
- `audio_service`: `^0.18.12` (or latest compatible version).
- Ensure any transitive dependencies (like `just_audio` if required by `audio_service` implementation details) are pinned if needed, though `audio_service` alone should suffice for the handler.

## 9. Test plan hooks

- **Widget tests**: Provide a mock `FlutterTts` interface and a mock `AudioHandler`. The tests will trigger the `voiceModeProvider` toggle, simulate the button callback (by calling the mock's registered callback directly), and verify that `SessionDriver.logSet()` is called. They will also verify that TTS method calls are made with the correct string arguments.
- **Unit tests**: Test the `VoiceSessionAdapter` (or its pure logic components) state transitions independently of widgets. Verify that transitioning from exercise to rest cues the correct strings.
- **Earbud simulation**: In tests, the earbud button is simulated by invoking the `onMediaButton` or `play/pause` callback exposed by the mock `AudioHandler`.

## 10. Acceptance criteria mapping

- **AC1** (Voice mode toggle) -> §2 (Flutter architecture)
- **AC2** (Exercise announcement) -> §3 (TTS script)
- **AC3** (Set announcement) -> §3 (TTS script)
- **AC4** (Earbud button advance) -> §4 (`audio_service` integration)
- **AC5** (Rest cue) -> §3 (TTS script)
- **AC6** (Next-exercise transition) -> §2, §3 (State machine, TTS script)
- **AC7** (Background audio) -> §4, §5, §6 (Platform configs, audio_service)
- **AC8** (Screen-on fallback) -> §2 (Flutter architecture)
- **AC9** (Mid-session toggle) -> §2 (Flutter architecture)
- **AC10** (Program-aware narration) -> §1 (Overview)
- **AC11** (TTS failure resilience) -> §7 (Error handling)
- **AC12** (Earbud disconnect resilience) -> §7 (Error handling)
- **AC13** (Tests) -> §9 (Test plan hooks)
- **AC14** (Scope guard) -> §1 (Overview, no backend changes)
