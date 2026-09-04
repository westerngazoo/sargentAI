// SAC2 -> AC2: the preset list is a PRESENTATION-ONLY constant (architect
// finding 5) — a UI convenience that pre-fills the same validated free-text
// path. No client-side schema is invented: every preset name must pass the
// exact validator a typed name passes, and each entry suggests one of the six
// backend muscle groups. Slated for replacement by the M4 library.
//
// RED until package:fitai/src/workout/domain/preset_exercises.dart
// defines the `presetExercises` constant (entries expose `name` + `group`,
// matching the addExercise(name, {group}) call it feeds).

import 'package:fitai/src/workout/domain/exercise_draft.dart';
import 'package:fitai/src/workout/domain/muscle_group.dart';
import 'package:fitai/src/workout/domain/preset_exercises.dart';
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

  // The voice coach seeded a planned session with bare names
  // (`_driver.addExercise(name)`), so every voice-planned exercise carried a
  // null muscle group and dropped out of per-muscle volume — the balance
  // graphics went blind to exactly the sessions logged hands-free. The lookup
  // below is how the seeding path recovers the group the catalogue already
  // knows.
  group('presetGroupFor', () {
    test('resolves a catalogue lift regardless of case or padding', () {
      expect(presetGroupFor('Squat'), MuscleGroup.legs);
      expect(presetGroupFor('squat'), MuscleGroup.legs);
      expect(presetGroupFor('  DEADLIFT  '), MuscleGroup.back);
    });

    test('returns null for a lift the catalogue does not know', () {
      expect(presetGroupFor('Zercher good morning'), isNull);
      expect(presetGroupFor(''), isNull);
    });

    test('every catalogue entry resolves to its own group', () {
      for (final preset in presetExercises) {
        expect(presetGroupFor(preset.name), preset.group, reason: preset.name);
      }
    });
  });
}
