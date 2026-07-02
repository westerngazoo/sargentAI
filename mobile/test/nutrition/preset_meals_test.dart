// Preset meals: catalog sanity and the fuzzy voice matcher.

import 'package:fitai/src/nutrition/domain/preset_meals.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('every preset has positive macros and a kcal derivation', () {
    for (final meal in presetMeals) {
      expect(meal.proteinG + meal.carbsG + meal.fatG, greaterThan(0),
          reason: meal.name);
      expect(meal.kcal, greaterThan(0));
    }
  });

  test('matches a full name inside a longer utterance', () {
    expect(matchPresetMeal('log a meal protein shake')!.name, 'Protein shake');
    expect(matchPresetMeal('i had chicken & rice')!.name, 'Chicken & rice');
  });

  test('matches on the distinctive head word', () {
    expect(matchPresetMeal('log my burrito')!.name, 'Burrito bowl');
    expect(matchPresetMeal('had some oats')!.name, 'Oats with whey');
  });

  test('returns null when nothing fits', () {
    expect(matchPresetMeal('log a meal'), isNull);
    expect(matchPresetMeal(''), isNull);
  });
}
