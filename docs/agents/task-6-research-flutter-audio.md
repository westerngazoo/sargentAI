# Agent Task — Research: flutter_tts + audio_service Integration

**Project root:** `/Users/goose/projects/fitAI`
**Branch:** none — research only, no code
**Output:** paste your findings back in the chat (or write to `docs/flutter-audio-research.md` and commit to `main`)

---

## What you are doing

You are a research agent. Do NOT write any production code. Produce a technical
integration note that will be used to validate SPEC-0027 (earbud-guided training)
before implementation begins.

---

## Step 1 — Read these files first

```
requirements/0027-earbud-guided-training.md   ← the accepted requirement (14 ACs)
mobile/pubspec.yaml                           ← packages already in the project
mobile/lib/src/workout/                       ← SessionDriver, LiveSessionScreen
```

---

## Step 2 — Context

R-0027 adds hands-free, earbud-guided training to the fitAI Flutter app:
- A voice layer reads out each exercise/set/rest cue via TTS
- A single earbud button press logs the current set and advances to the next
- The app must continue speaking in the background when the phone is locked

**Technology already decided:**
- TTS: `flutter_tts`
- Background audio + media-button hook: `audio_service`
- Both iOS and Android must work with screen off
- No changes to the Rust backend — Flutter-only

---

## Step 3 — Research questions

Answer each question precisely. Cite pub.dev changelog entries, GitHub issues,
Flutter docs, or Apple/Android platform docs where relevant.

### 1. flutter_tts + audio_service coexistence on iOS

- Do `flutter_tts` and `audio_service` conflict over `AVAudioSession` category
  management on iOS?
- Is there a required initialization order (e.g. init `audio_service` first, set
  `AVAudioSession` category, then init `flutter_tts`)?
- Do they share or fight over the audio session when TTS speaks while
  `audio_service` has the session?

### 2. Minimum AudioHandler for button + foreground service

We do NOT want to stream audio through `audio_service`. We only need:
(a) A foreground service on Android (so the OS doesn't kill the app while TTS plays)
(b) Registration with `MPRemoteCommandCenter` on iOS (to receive earbud button events)

What is the absolute minimum `AudioHandler` subclass we need to implement?
Which `AudioHandler` methods must be non-stub? Which can be empty stubs?

### 3. Media button with screen locked on iOS

- Does `MPRemoteCommandCenter` fire events to the app when the screen is locked
  and there is no "currently playing" item registered?
- Is there a known workaround (e.g. setting a `MPNowPlayingInfoCenter` entry,
  playing a 1ms silent audio clip via `audio_service`) to keep the command center
  active?
- Does the earbud's single-press play/pause map to `onPlay`/`onPause` in
  `audio_service`, or to a different callback?

### 4. flutter_tts background mode on iOS

- Does `flutter_tts` itself require `UIBackgroundModes: [audio]` in `Info.plist`,
  or does `audio_service` inject this automatically?
- What is the exact `Info.plist` entry needed (`UIBackgroundModes`, value array)?
- Does `flutter_tts` set `AVAudioSession.category` itself, or does the app need
  to set it explicitly?

### 5. KEYCODE_HEADSETHOOK on Android

- Which `audio_service` callback fires when the earbud button is pressed on
  Android (`KEYCODE_HEADSETHOOK` / `KEYCODE_MEDIA_PLAY_PAUSE`)?
- Do we need to declare anything extra in `AndroidManifest.xml` to receive this
  event when the app is in the background?
- Android 14+ changes: any new permissions or foreground service type declarations
  required for a "media playback" foreground service in Android 14+?

### 6. Known issues / gotchas

List any:
- Open pub.dev issues on `flutter_tts` or `audio_service` that could affect this
  implementation (broken iOS background, Android 14 regression, etc.)
- Flutter SDK version incompatibilities
- Anything that would require choosing a different package (e.g. `just_audio`
  for the silent-clip trick, `flutter_background_service` instead of
  `audio_service` for the Android foreground service)

---

## Step 4 — Output format

Write a concise technical note (400–600 words):

```
# flutter_tts + audio_service Integration Research
Date: 2026-06-22

## 1. iOS coexistence
## 2. Minimum AudioHandler
## 3. Media button with screen locked
## 4. flutter_tts background mode
## 5. Android KEYCODE_HEADSETHOOK
## 6. Known issues
## Red flags (anything that might require a spec change)
```

The **Red flags** section is the most important — if anything you find suggests
the spec design needs to change (package substitution, additional platform step,
ABI conflict), call it out clearly.

---

## Step 5 — Commit (optional)

If you write the note to a file:

```bash
git checkout main
git add docs/flutter-audio-research.md
git commit -m "docs: flutter_tts + audio_service research note for R-0027"
git push
```
