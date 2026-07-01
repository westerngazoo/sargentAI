import 'package:audio_service/audio_service.dart';
import 'package:just_audio/just_audio.dart';

class AudioServiceHandler extends BaseAudioHandler {
  final AudioPlayer _player = AudioPlayer();
  final void Function() onMediaButtonPress;

  AudioServiceHandler({required this.onMediaButtonPress}) {
    _player.setAsset('assets/audio/silence.mp3');
    _player.setLoopMode(LoopMode.all);

    playbackState.add(PlaybackState(
      controls: [MediaControl.pause],
      playing: true,
      processingState: AudioProcessingState.ready,
    ));
  }

  Future<void> startSilentLoop() async {
    await _player.play();
  }

  @override
  Future<void> play() async {
    onMediaButtonPress();
  }

  @override
  Future<void> pause() async {
    onMediaButtonPress();
  }

  @override
  Future<void> stop() async {
    await _player.stop();
    await _player.dispose();
    await super.stop();
  }
}
