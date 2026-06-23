# SPEC-0027 — Earbud-guided training

- **Status:** Draft
- **Realizes:** R-0027
- **Created:** 2026-06-22
- **Depends on:** R-0009 (SessionDriver), R-0014 (UserProgram)
- **Module(s):**
  `mobile/lib/src/workout/` (VoiceSessionAdapter, existing SessionDriver)
  `mobile/lib/src/audio/` (AudioServiceHandler)
  `mobile/pubspec.yaml`
  `mobile/ios/Runner/Info.plist`
  `mobile/android/app/src/main/AndroidManifest.xml`

---

## §1 Overview

This spec realizes R-0027. It builds on the widget-independent `SessionDriver`
(R-0009) and reads from the active `UserProgram` (R-0014).

A `VoiceSessionAdapter` sits between `SessionDriver` and the audio layer. It
listens to `SessionDriver` state changes, speaks TTS scripts via `flutter_tts`,
and keeps a silent audio track looping via `just_audio` through `audio_service`
so that iOS delivers earbud-button events with the screen locked. The entire
feature is toggled from the existing `LiveSessionScreen`. No backend changes.

---

## §2 Flutter architecture

### Class relationships

```
LiveSessionScreen
  └── voiceModeProvider (Riverpod, bool)
        └── VoiceSessionAdapter
              ├── ref.listen(sessionDriverProvider)   → drives TTS
              ├── FlutterTts                          → speaks cues
              ├── AudioServiceHandler (custom)
              │     └── just_audio Player (silent loop) → keeps iOS command center alive
              └── SessionDriver.logSet()              ← called on button press
```

### State machine

```
voice mode off
  │  user taps toggle
  ▼
voice mode on
  │  adapter reads current exercise from SessionDriver
  ▼
exercise announced  ("Next: {name}. {N} sets of {R} reps[ at {W} kg].")
  │
  ▼
set announced       ("Set {X} of {N}. {R} reps[ at {W} kg]. Go.")
  │  earbud button pressed OR on-screen tap
  ▼
set logged          (SessionDriver.logSet() called; TTS interrupted if mid-utterance)
  │  SessionDriver emits rest state
  ▼
rest cued           ("Rest.")
  │  rest period elapses OR next set starts
  ▼
set announced  ──── loops until workout done
  │
  ▼
workout done        ("Workout done.")
  │  user taps toggle OR session ends
  ▼
voice mode off      (TTS released, silent player stopped, handler deregistered)
```

### Toggle

The toggle button on `LiveSessionScreen` flips `voiceModeProvider` (a
`StateProvider<bool>`). `VoiceSessionAdapter` is a Riverpod provider that
watches `voiceModeProvider`; it initializes on `true` and disposes on `false`.
State is transient and never persisted.

### OQ-H2 resolution — TTS interruption

Each new utterance **interrupts** the previous one (`flutterTts.stop()` before
`flutterTts.speak()`). This keeps the adapter responsive to rapid button presses.

### OQ-H3 resolution — SessionDriver hook

The adapter hooks in as a **Riverpod listener** (`ref.listen(sessionDriverProvider, ...)`).
It calls the existing `SessionDriver.logSet()` method from the button callback.
No new wrapper or dedicated method on `SessionDriver`.

---

## §3 TTS script (exact strings)

| Trigger | String |
|---------|--------|
| Exercise start | `"Next: {name}. {N} sets of {R} reps[ at {W} kg]."` |
| Set start | `"Set {X} of {N}. {R} reps[ at {W} kg]. Go."` |
| Rest | `"Rest."` |
| Workout done | `"Workout done."` |

Rules:
- `[ at {W} kg]` is omitted entirely when the target weight is null or not set.
- `{name}` is taken verbatim from the session's exercise name.
- Strings are constants in `lib/src/audio/tts_scripts.dart`, unit-testable.

---

## §4 `audio_service` integration

### OQ-H1 resolution — AudioHandler scope

R-0027 implements a minimal `BaseAudioHandler` subclass (`AudioServiceHandler`).
Only these methods are non-stubs:

| Method | Purpose |
|--------|---------|
| `play()` | Called by earbud single-press (iOS play/pause, Android HEADSETHOOK). Triggers `SessionDriver.logSet()` and advances TTS. |
| `pause()` | Same as `play()` — both map to the button (OS may call either). |
| `stop()` | Stops the silent player and cleans up. Called when voice mode is toggled off. |

All other `AudioHandler` methods (seek, skipToNext, etc.) remain empty stubs.

### Silent track — iOS locked-screen requirement

iOS delivers `MPRemoteCommandCenter` events only while audio is actively
playing. Between TTS utterances (e.g. during a rest period) the device would
suspend the app and drop button presses.

**Fix:** `AudioServiceHandler` holds a `just_audio` `AudioPlayer` that loops a
1-second bundled silent audio asset (`assets/audio/silence.mp3`). The player
starts when voice mode activates and stops when it deactivates.
`PlaybackState(playing: true, controls: [MediaControl.pause])` is set so the
command center treats the app as actively playing.

### OQ-H5 resolution — Android media notification

- **Title:** "FitAI Workout"
- **Icon:** app launcher icon (default; no custom dumbbell needed for MVP)
- **Importance:** `LOW` — no sound or vibration from the notification itself

### Media-button mapping

| Platform | Event | Handler |
|----------|-------|---------|
| iOS | `MPRemoteCommandCenter` play/pause | `AudioServiceHandler.play()` / `pause()` |
| Android | `KEYCODE_HEADSETHOOK` / `KEYCODE_MEDIA_PLAY_PAUSE` | Same, via `audio_service` mapping |

---

## §5 iOS platform config

### OQ-H4 resolution

`flutter_tts` and `audio_service` do **not** inject `Info.plist` entries
automatically. Add manually:

```xml
<!-- mobile/ios/Runner/Info.plist -->
<key>UIBackgroundModes</key>
<array>
  <string>audio</string>
</array>
```

**AVAudioSession category:** Set `playback` via both:
```dart
await flutterTts.setSharedInstance(true);
await flutterTts.setIosAudioCategory(IosTextToSpeechAudioCategory.playback);
```
Call these before the first `speak()`. `audio_service` initializes the session;
`flutter_tts` must be told to share it (`setSharedInstance(true)`) to avoid
the two fighting over the session.

**Entitlement:** None beyond the `Info.plist` background mode.

---

## §6 Android platform config

```xml
<!-- mobile/android/app/src/main/AndroidManifest.xml -->
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK" />

<service
  android:name="com.ryanheise.audioservice.AudioService"
  android:foregroundServiceType="mediaPlayback"
  android:exported="true">
  <intent-filter>
    <action android:name="android.intent.action.MEDIA_BUTTON" />
  </intent-filter>
</service>

<receiver android:name="com.ryanheise.audioservice.MediaButtonReceiver"
          android:exported="true">
  <intent-filter>
    <action android:name="android.intent.action.MEDIA_BUTTON" />
  </intent-filter>
</receiver>
```

`FOREGROUND_SERVICE_MEDIA_PLAYBACK` is required for Android 14+ to declare
the foreground service type.

---

## §7 Error handling

| Scenario | Behaviour |
|----------|-----------|
| TTS engine unavailable at init | Log error; show brief snackbar "Voice unavailable"; silent continue — on-screen taps still advance the session |
| TTS `speak()` fails mid-session | Log error; continue silently; visual set-counter still updates |
| Audio focus denied | Log warning; continue; TTS may not play but button still works |
| Earbud disconnect | `audio_service` fires a route-change event; release media-button handler; on-screen taps continue; silent player pauses |
| Reconnect / re-toggle | User toggling off then on re-initializes the handler and silent player |

---

## §8 pubspec.yaml additions

```yaml
dependencies:
  flutter_tts: ^4.0.2
  audio_service: ^0.18.12
  just_audio: ^0.9.40          # silent loop for iOS locked-screen button delivery
```

Add the silent asset:

```yaml
flutter:
  assets:
    - assets/audio/silence.mp3  # 1-second silent MP3, bundled in mobile/assets/audio/
```

---

## §9 Test plan hooks

### Unit tests — `VoiceSessionAdapter`

- State transitions: off → on → exercise announced → set logged → rest cued → next set
- TTS string construction for every branch (weight present, weight absent)
- `tts_scripts.dart` pure functions tested in isolation

### Widget tests — `LiveSessionScreen` voice mode

Mock interfaces:
- `FlutterTts` — verify `speak()` called with exact strings; verify `stop()` called on interrupt
- `AudioServiceHandler` — verify `play()` callback wired to `SessionDriver.logSet()`

Simulate earbud button in tests by calling the mock's `play()` / `pause()`
callback directly (no platform-channel involvement needed in unit/widget tests).

### Integration tests (device/emulator)

Out of scope for automated CI — covered by QA manual sign-off per AC7 (background audio).

---

## §10 Acceptance criteria mapping

| AC | Spec section |
|----|-------------|
| AC1 — voice mode toggle | §2 (toggle, `voiceModeProvider`) |
| AC2 — exercise announcement | §3 (TTS script, exercise start) |
| AC3 — set announcement | §3 (TTS script, set start) |
| AC4 — earbud button advance | §4 (`play()`/`pause()` → `logSet()`) |
| AC5 — rest cue | §3 (TTS script, rest) |
| AC6 — next-exercise transition | §2 (state machine), §3 |
| AC7 — background audio | §4 (silent loop), §5 iOS, §6 Android |
| AC8 — screen-on fallback | §2 (on-screen taps always work) |
| AC9 — mid-session toggle | §2 (dispose on `false`, re-init on `true`) |
| AC10 — program-aware narration | §1 (reads from UserProgram via SessionDriver) |
| AC11 — TTS failure resilience | §7 (error handling) |
| AC12 — earbud disconnect resilience | §7 (error handling) |
| AC13 — tests | §9 (test plan hooks) |
| AC14 — scope guard | §1 (no backend changes) |
