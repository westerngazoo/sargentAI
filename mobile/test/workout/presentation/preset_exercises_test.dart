// SAC2 -> AC2: the preset list is a PRESENTATION-ONLY constant (architect
// finding 5) — a UI convenience that pre-fills the same validated free-text
// path. No client-side schema is invented: every preset name must pass the
// exact validator a typed name passes, and each entry suggests one of the six
// backend muscle groups. Slated for replacement by the M4 library.
//
// RED until package:fitai/src/workout/presentation/preset_exercises.dart
// defines the `presetExercises` constant (entries expose `name` + `group`,
// matching the addExercise(name, {group}) call it feeds).

import 'package:fitai/src/workout/domain/exercise_draft.dart';
import 'package:fitai/src/workout/presentation/preset_exercises.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('SAC2: offers a non-trivial set of common lifts', () {
    expect(presetExercises.length, greaterThanOrEqualTo(10));
  });

  test(
      'SAC2: every preset name passes the SAME free-text validator '
      '(no parallel schema)', () {
    for (final preset in presetExercises) {
      expect(ExerciseDraft(name: preset.name).nameError(), isNull,
          reason: preset.name);
    }
  });

  test('SAC2: preset names are unique', () {
    final names = presetExercises.map((p) => p.name).toList();
    expect(names.toSet().length, names.length);
  });

  test('SAC2: each preset suggests a muscle group for the tag chip', () {
    for (final preset in presetExercises) {
      expect(preset.group, isNotNull, reason: preset.name);
    }
  });
}
