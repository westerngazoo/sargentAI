// Unit tests for the voice intent service response model.

import 'package:fitai/src/hub/voice_intent_service.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('VoiceIntentResult.fromJson parses logged workout', () {
    final r = VoiceIntentResult.fromJson({
      'status': 'logged_workout',
      'message': 'Logged bench press.',
      'record_id': 'abc-123',
    });
    expect(r.isLoggedWorkout, isTrue);
    expect(r.message, 'Logged bench press.');
    expect(r.recordId, 'abc-123');
  });

  test('VoiceIntentResult.fromJson parses clarify', () {
    final r = VoiceIntentResult.fromJson({
      'status': 'clarify',
      'prompt': 'How many grams of protein?',
    });
    expect(r.isClarify, isTrue);
    expect(r.prompt, 'How many grams of protein?');
  });
}
