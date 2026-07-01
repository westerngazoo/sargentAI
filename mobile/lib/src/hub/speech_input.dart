// R-0032 (slice 1) — dependency-inverted speech-to-text seam.
//
// The hub screen talks to [SpeechInput] only; the `speech_to_text` package
// is confined to [PluginSpeechInput]. Tests override [speechInputProvider]
// with a fake — mirroring the `Arc<dyn PoseEstimator>` pattern backend-side.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:speech_to_text/speech_recognition_result.dart';
import 'package:speech_to_text/speech_to_text.dart' as stt;

/// Callback fired with the running transcript; `isFinal` marks the last one.
typedef OnTranscript = void Function(String transcript, bool isFinal);

/// What the voice hub needs from a speech engine — nothing more.
abstract class SpeechInput {
  /// One-time engine init. Returns false when speech is unavailable
  /// (no permission, unsupported browser/device).
  Future<bool> initialize();

  /// Starts a listening session; [onTranscript] receives partial and final
  /// results until the engine stops.
  Future<void> listen(OnTranscript onTranscript);

  /// Stops the current listening session.
  Future<void> stop();
}

/// Production implementation over the `speech_to_text` plugin
/// (Web Speech API in the browser, native recognisers on device).
class PluginSpeechInput implements SpeechInput {
  final stt.SpeechToText _engine = stt.SpeechToText();

  @override
  Future<bool> initialize() async {
    try {
      return await _engine.initialize();
    } catch (_) {
      return false;
    }
  }

  @override
  Future<void> listen(OnTranscript onTranscript) => _engine.listen(
        onResult: (SpeechRecognitionResult r) =>
            onTranscript(r.recognizedWords, r.finalResult),
      );

  @override
  Future<void> stop() => _engine.stop();
}

final speechInputProvider = Provider<SpeechInput>((ref) => PluginSpeechInput());
