// Unit tests for the voice intent service response model.

import 'package:dio/dio.dart';
import 'package:fitai/src/hub/voice_intent_service.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

class MockDio extends Mock implements Dio {}

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

  test('VoiceIntentService.parse sends history properly', () async {
    final dio = MockDio();
    final service = VoiceIntentService(dio);

    when(() => dio.post<Map<String, dynamic>>(
      any(),
      data: any(named: 'data'),
    )).thenAnswer((_) async => Response<Map<String, dynamic>>(
      requestOptions: RequestOptions(path: ''),
      data: {'status': 'clarify', 'prompt': 'test'},
    ));

    final history = [
      (fromUser: true, text: 'log a meal'),
      (fromUser: false, text: 'Tell me more.'),
    ];

    await service.parse('chicken', history: history);

    final captured = verify(() => dio.post<Map<String, dynamic>>(
      any(),
      data: captureAny(named: 'data'),
    )).captured;

    final data = captured.first as Map<String, dynamic>;
    expect(data['transcript'], 'chicken');
    expect(data['history'], isA<List>());
    expect(data['history'][0]['from_user'], isTrue);
    expect(data['history'][0]['text'], 'log a meal');
    expect(data['history'][1]['from_user'], isFalse);
    expect(data['history'][1]['text'], 'Tell me more.');
  });
}
