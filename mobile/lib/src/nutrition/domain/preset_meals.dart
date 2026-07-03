// Preset meals — one tap (or one utterance) instead of typing macros.
// Gym-staple servings with sensible macro estimates; the user can still
// tweak the prefilled fields before saving.

/// A ready-made meal with macros per serving.
class PresetMeal {
  const PresetMeal(this.name, this.proteinG, this.carbsG, this.fatG);

  final String name;
  final double proteinG;
  final double carbsG;
  final double fatG;

  double get kcal => 4 * proteinG + 4 * carbsG + 9 * fatG;
}

const List<PresetMeal> presetMeals = <PresetMeal>[
  PresetMeal('Chicken & rice', 45, 65, 10),
  PresetMeal('Protein shake', 30, 8, 3),
  PresetMeal('Oats with whey', 35, 55, 9),
  PresetMeal('Eggs & toast', 22, 30, 18),
  PresetMeal('Greek yogurt & granola', 20, 35, 8),
  PresetMeal('Tuna salad', 35, 10, 12),
  PresetMeal('Beef & potatoes', 40, 50, 20),
  PresetMeal('Salmon & vegetables', 35, 15, 22),
  PresetMeal('Turkey sandwich', 28, 40, 10),
  PresetMeal('Burrito bowl', 35, 60, 18),
  PresetMeal('Pasta & chicken', 42, 70, 14),
  PresetMeal('Peanut butter sandwich', 15, 38, 18),
  PresetMeal('Cottage cheese & fruit', 25, 20, 5),
  PresetMeal('Casein & banana', 28, 30, 3),
];

/// Fuzzy match: the transcript contains the meal name (or vice versa for
/// short utterances like "shake"). Case-insensitive; null when nothing fits.
PresetMeal? matchPresetMeal(String text) {
  final t = text.toLowerCase().trim().replaceAll(' and ', ' & ');
  if (t.isEmpty) return null;
  for (final meal in presetMeals) {
    final name = meal.name.toLowerCase();
    if (t.contains(name)) return meal;
    // Loose match on the distinctive first word ("burrito", "oats"…) —
    // never on macro words ("protein" alone is a dictation, not a shake).
    final head = name.split(RegExp(r'[ &]')).first;
    const excluded = {'protein'};
    if (head.length >= 4 && !excluded.contains(head) && t.contains(head)) {
      return meal;
    }
  }
  return null;
}
