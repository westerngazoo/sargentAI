# SPEC-0035 — Earbud-Guided Hands-Free Training (Rebuild)

- **Status:** Draft
- **Realizes:** R-0035
- **Author:** Jules
- **Created:** 2026-07-06
- **Depends on:** R-0009, R-0014
- **Module(s):** `mobile/lib/src/workout`

## 1. Motivation

This spec realizes R-0035 by rebuilding the earbud media-button hands-free transport that was reverted. It allows users to log workouts via a single media-button press (e.g., Bluetooth earbuds) while the app narrates the session over TTS. It must coexist with the voice dictation coach (R-0032) without removing it.

## 2. Design

- **Audio Backgrounding:** We use `audio_service` combined with `just_audio` playing a silent track to keep the iOS `MPRemoteCommandCenter` alive between TTS utterances when the screen is locked. Android uses a foreground service via `audio_service`.
- **`AudioServiceHandler`:** Implements `BaseAudioHandler`. Intercepts `play()` and `pause()` from the media button and triggers a callback.
- **`EarbudCoach`:** A Riverpod `Notifier` (similar to `VoiceCoach`) that manages the `AudioServiceHandler` and `flutter_tts` instance. It listens to `SessionDriver` state changes and triggers TTS announcements ("Next: bench press...", "Rest.").
- **Coexistence (OQ-3):** Earbud mode and dictation mode are mutually exclusive. Turning on `EarbudCoach` disables `VoiceCoach`, and vice-versa.
- **Set Targets (OQ-2):** For now, the coach narrate names + last-session weights / default targets as a first cut. If the user repeats the last set, it uses those values.

## 3. Code outline

```dart
class EarbudCoach extends Notifier<EarbudCoachState> {
  Future<void> enable() async { ... }
  Future<void> disable() async { ... }
  void _onMediaButton() {
    // Log the last set's reps/weight or an empty set
  }
}
```

## 4. Non-goals

- No speech recognition in this mode (handled by `VoiceCoach`).
- No complex program-model extensions for per-set targets (will just repeat last logged set or announce exercise name).

## 5. Open questions

All settled.

## 6. Acceptance criteria

- [ ] AC1: Toggle on `LiveSessionScreen` enables/disables earbud mode.
- [ ] AC2-AC6: TTS announces exercise, set, rest, and workout done.
- [ ] AC4: Media button advances the session.
- [ ] AC7: Background audio works (iOS & Android).
- [ ] AC8: Both screen and button advance session.
- [ ] AC9: Toggling off preserves state and releases handler.
- [ ] AC13: Unit/widget tests.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-06 | Mutually exclusive modes | Avoids TTS/microphone conflict over audio session. |

## Changelog

- _created_
