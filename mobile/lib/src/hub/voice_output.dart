// Voice-out seam (R-0027/R-0032) — mirror of [SpeechInput] for TTS.
//
// The coach talks to [VoiceOutput] only; `flutter_tts` is confined to
// [PluginVoiceOutput]. TTS failures are swallowed (R-0027 AC11 spirit): the
// session must continue silently, never crash, if the engine is unavailable.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_tts/flutter_tts.dart';

/// What the voice coach needs from a text-to-speech engine — nothing more.
abstract class VoiceOutput {
  /// One-time engine init. Returns false when TTS is unavailable.
  Future<bool> initialize();

  /// Speaks [text]; resolves when the utterance completes (or immediately on
  /// engine failure).
  Future<void> speak(String text);

  /// Stops any in-flight utterance.
  Future<void> stop();
}

/// Production implementation over the `flutter_tts` plugin
/// (SpeechSynthesis in the browser, native engines on device).
class PluginVoiceOutput implements VoiceOutput {
  final FlutterTts _tts = FlutterTts();

  @override
  Future<bool> initialize() async {
    try {
      await _tts.awaitSpeakCompletion(true);
      return true;
    } catch (_) {
      return false;
    }
  }

  @override
  Future<void> speak(String text) async {
    try {
      await _tts.speak(text);
    } catch (_) {
      // Silent continuation — voice is an enhancement, never a blocker.
    }
  }

  @override
  Future<void> stop() async {
    try {
      await _tts.stop();
    } catch (_) {}
  }
}

final voiceOutputProvider = Provider<VoiceOutput>((ref) => PluginVoiceOutput());
