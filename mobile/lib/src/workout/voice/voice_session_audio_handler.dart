import 'package:audio_service/audio_service.dart';

/// A minimal AudioHandler to capture media button presses (headset clicks).
/// We override `play()` and `pause()` since different OS/headsets might map
/// the single click to either action. We don't implement full playback controls.
class VoiceSessionAudioHandler extends BaseAudioHandler {
  VoiceSessionAudioHandler() {
    _setPlaybackState(playing: true);
  }

  void Function()? onMediaButtonPress;

  @override
  Future<void> play() async {
    onMediaButtonPress?.call();
    _setPlaybackState(playing: true);
  }

  @override
  Future<void> pause() async {
    onMediaButtonPress?.call();
    _setPlaybackState(playing: true);
  }

  void _setPlaybackState({required bool playing}) {
    playbackState.add(playbackState.value.copyWith(
      controls: [MediaControl.pause, MediaControl.play],
      systemActions: const {MediaAction.seek},
      androidCompactActionIndices: const [0],
      playing: playing,
      processingState: AudioProcessingState.ready,
    ));
  }
}
