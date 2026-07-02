// R-0032 (slice 1) — unit tests for the pure voice-intent parser.

import 'package:fitai/src/hub/voice_intent.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('workout intents', () {
    for (final phrase in [
      'log a workout',
      'start my session',
      'time to train',
      'gym time',
    ]) {
      test('"$phrase" → LogWorkoutIntent', () {
        expect(parseVoiceIntent(phrase), isA<LogWorkoutIntent>());
      });
    }
  });

  group('meal intents', () {
    test('"log a meal" → LogMealIntent with no macros', () {
      final intent = parseVoiceIntent('log a meal');
      expect(intent, isA<LogMealIntent>());
      final meal = intent as LogMealIntent;
      expect(meal.proteinG, isNull);
      expect(meal.carbsG, isNull);
      expect(meal.fatG, isNull);
    });

    test('dictated macros are extracted ("40 grams of protein …")', () {
      final intent = parseVoiceIntent(
          'log a meal with 40 grams of protein 60 carbs and 20 fat');
      final meal = intent as LogMealIntent;
      expect(meal.proteinG, 40);
      expect(meal.carbsG, 60);
      expect(meal.fatG, 20);
    });

    test('"protein 35" order also parses', () {
      final meal =
          parseVoiceIntent('I ate food with protein 35') as LogMealIntent;
      expect(meal.proteinG, 35);
    });

    test('meal wins over workout when both keywords appear', () {
      expect(
          parseVoiceIntent('log my post workout meal'), isA<LogMealIntent>());
    });
  });

  test('"show my program" → ShowProgramIntent', () {
    expect(parseVoiceIntent('show my program'), isA<ShowProgramIntent>());
  });

  test('"find my body type" → BodyMatchIntent', () {
    expect(parseVoiceIntent('find my body type'), isA<BodyMatchIntent>());
  });

  test('"show my history" → ShowHistoryIntent', () {
    expect(parseVoiceIntent('show my history'), isA<ShowHistoryIntent>());
  });

  test('"open my profile" → ShowProfileIntent', () {
    expect(parseVoiceIntent('open my profile'), isA<ShowProfileIntent>());
  });

  test('gibberish → UnknownIntent carrying the transcript', () {
    final intent = parseVoiceIntent('purple monkey dishwasher');
    expect(intent, isA<UnknownIntent>());
    expect((intent as UnknownIntent).transcript, 'purple monkey dishwasher');
  });

  test('empty transcript → UnknownIntent', () {
    expect(parseVoiceIntent('   '), isA<UnknownIntent>());
  });

  group('hub option label mapping', () {
    test('intent → label', () {
      expect(hubOptionLabelForIntent(const LogWorkoutIntent()), 'Workout');
      expect(hubOptionLabelForIntent(const ShowProgramIntent()), 'Program');
      expect(
          hubOptionLabelForIntent(const UnknownIntent('x')), isNull);
    });

    test('spoken option name highlights before full parse', () {
      expect(matchedHubOptionLabel('open program please'), 'Program');
      expect(matchedHubOptionLabel('start workout'), 'Workout');
    });

    test('keyword parse fills in when label not spoken verbatim', () {
      expect(matchedHubOptionLabel('time to train'), 'Workout');
    });
  });
}
