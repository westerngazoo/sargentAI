// Shared voice fakes: a scriptable speech engine and a recording TTS.

import 'package:fitai/src/hub/speech_input.dart';
import 'package:fitai/src/hub/voice_output.dart';

/// Emits queued transcripts, one final result per `listen()` call.
class ScriptedSpeechInput implements SpeechInput {
  ScriptedSpeechInput(this.transcripts, {this.available = true});

  final List<String> transcripts;
  final bool available;
  int _next = 0;

  @override
  Future<bool> initialize() async => available;

  @override
  Future<void> listen(OnTranscript onTranscript) async {
    final t = _next < transcripts.length ? transcripts[_next++] : '';
    onTranscript(t, true);
  }

  @override
  Future<void> stop() async {}
}

/// Records everything spoken so tests can assert on announcements.
class RecordingVoiceOutput implements VoiceOutput {
  final List<String> spoken = [];

  @override
  Future<bool> initialize() async => true;

  @override
  Future<void> speak(String text) async => spoken.add(text);

  @override
  Future<void> stop() async {}
}
