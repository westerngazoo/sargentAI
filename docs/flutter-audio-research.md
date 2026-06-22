# flutter_tts + audio_service Integration Research
Date: 2026-06-22

## 1. iOS coexistence
Both `flutter_tts` and `audio_service` interact with the underlying iOS `AVAudioSession`. To ensure they coexist peacefully, you must instruct `flutter_tts` to use a shared audio session. According to the `flutter_tts` documentation, you need to call `await flutterTts.setSharedInstance(true);`.

Additionally, configuring the `AVAudioSession` appropriately via `flutterTts.setIosAudioCategory(...)` (e.g. `IosTextToSpeechAudioCategory.ambient` or `IosTextToSpeechAudioCategory.playback`) and using the `audio_session` package to define a global policy is strongly recommended so neither plugin overrides the other's settings.

## 2. Minimum AudioHandler
To leverage the media button and background service features without streaming real audio, you must implement a subclass of `BaseAudioHandler`. The minimum requirement involves overriding the media control callbacks that correspond to the earbud button presses. Typically, overriding `play()` and/or `pause()` is required to intercept the remote control events, while the other `AudioHandler` methods (e.g. `seek`, `skipToNext`) can remain empty stubs.

## 3. Media button with screen locked
On iOS, the `MPRemoteCommandCenter` requires the app to have active, playing audio to reliably deliver media button events when the screen is locked. The `audio_service` documentation notes: "The OS may kill your process if it sits idly without playing audio... consider playing a silent audio track to create that effect rather than using an idle timer." Without a "currently playing" item (e.g., a silent track), iOS will eventually suspend the app, and the button events will be dropped. The single-press maps to the `play()`/`pause()` intent.

## 4. flutter_tts background mode
Neither `flutter_tts` nor `audio_service` injects the required iOS plist entries automatically. You must explicitly declare the audio background mode in your `Info.plist`:
```xml
<key>UIBackgroundModes</key>
<array>
  <string>audio</string>
</array>
```
`flutter_tts` allows explicit configuration of the `AVAudioSession` category via `flutterTts.setIosAudioCategory()`.

## 5. Android KEYCODE_HEADSETHOOK
On Android, a headset button press (like `KEYCODE_MEDIA_PLAY_PAUSE` or `KEYCODE_HEADSETHOOK`) maps to the `play()` or `pause()` callback in your `AudioHandler`.
To receive these events in the background, you must declare the `MediaButtonReceiver` in `AndroidManifest.xml`:
```xml
<receiver android:name="com.ryanheise.audioservice.MediaButtonReceiver" android:exported="true">
  <intent-filter>
    <action android:name="android.intent.action.MEDIA_BUTTON" />
  </intent-filter>
</receiver>
```
For Android 14+, you must also declare the `FOREGROUND_SERVICE_MEDIA_PLAYBACK` permission and specify `android:foregroundServiceType="mediaPlayback"` in the service declaration.

## 6. Known issues
- **iOS Suspension:** iOS strictly terminates background audio apps that are not actively playing sound.
- **Audio Session Conflicts:** Without explicitly managing the shared session and categories, `flutter_tts` and other media players can steal focus or mute one another.

## Red flags (anything that might require a spec change)
- **Silent Track Workaround Needed:** To keep the app alive on iOS and receive the earbud button events between sets (while no TTS is actively speaking), the app must continuously play a 1ms silent audio track. This requires adding a dedicated media playback package (e.g., `just_audio`) to loop the silent track via `audio_service`. The spec implies this can be done without extra overhead, but the silent-track workaround is mandatory for iOS locked-screen reliability.
