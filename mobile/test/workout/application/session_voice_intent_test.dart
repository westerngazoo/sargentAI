// Unit tests for the pure in-session dictation parser.

import 'package:fitai/src/workout/application/session_voice_intent.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('set dictations', () {
    test('"10 reps at 100 kilos" → reps + weight', () {
      final intent =
          parseSessionVoiceIntent('10 reps at 100 kilos') as LogSetIntent;
      expect(intent.reps, 10);
      expect(intent.weightKg, 100);
      expect(intent.rpe, isNull);
    });

    test('"8 reps at 62.5 kg rpe 9" → all three fields', () {
      final intent =
          parseSessionVoiceIntent('8 reps at 62.5 kg rpe 9') as LogSetIntent;
      expect(intent.reps, 8);
      expect(intent.weightKg, 62.5);
      expect(intent.rpe, 9);
    });

    test('"12 reps" → reps only (bodyweight)', () {
      final intent = parseSessionVoiceIntent('12 reps') as LogSetIntent;
      expect(intent.reps, 12);
      expect(intent.weightKg, isNull);
    });

    test('bare "10 100" fallback → reps then weight', () {
      final intent = parseSessionVoiceIntent('10 100') as LogSetIntent;
      expect(intent.reps, 10);
      expect(intent.weightKg, 100);
    });
  });

  group('commands', () {
    test('"next exercise" → NextExerciseIntent', () {
      expect(
          parseSessionVoiceIntent('next exercise'), isA<NextExerciseIntent>());
    });

    test('"finish workout" → FinishSessionIntent', () {
      expect(parseSessionVoiceIntent('finish workout'),
          isA<FinishSessionIntent>());
    });

    test('"done" alone starts guided set logging', () {
      expect(parseSessionVoiceIntent('done'), isA<SetDoneIntent>());
      expect(parseSessionVoiceIntent('finished set'), isA<SetDoneIntent>());
    });

    test('"save workout" also finishes', () {
      expect(
          parseSessionVoiceIntent('save workout'), isA<FinishSessionIntent>());
    });

    test('finish wins over numbers ("finish workout at 5") ', () {
      expect(parseSessionVoiceIntent('finish workout at 5'),
          isA<FinishSessionIntent>());
    });
  });

  test('gibberish without numbers → UnknownSessionIntent', () {
    final intent = parseSessionVoiceIntent('purple monkey dishwasher');
    expect(intent, isA<UnknownSessionIntent>());
    expect((intent as UnknownSessionIntent).transcript,
        'purple monkey dishwasher');
  });

  test('empty → UnknownSessionIntent', () {
    expect(parseSessionVoiceIntent('  '), isA<UnknownSessionIntent>());
  });

  test('"pause" → PauseSessionIntent', () {
    expect(parseSessionVoiceIntent('pause'), isA<PauseSessionIntent>());
  });
}
