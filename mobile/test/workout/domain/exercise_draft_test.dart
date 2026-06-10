// SAC2 -> AC2 (domain): the exercise-name validator mirrors
// `core::workout::ExerciseName::try_new` EXACTLY — trimmed first, then
// non-empty and counted in Unicode scalar values (`trimmed.runes.length`
// <= 100, the backend's `chars()` count — architect finding 6), NOT UTF-16
// `String.length`. Returns `String?` (null = ok), the ProfileDraft idiom.
//
// RED until package:fitai/src/workout/domain/exercise_draft.dart defines
// ExerciseDraft{name, muscleGroup?, sets} with nameError().

import 'package:fitai/src/workout/domain/exercise_draft.dart';
import 'package:fitai/src/workout/domain/muscle_group.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('SAC2 nameError — trimmed, non-empty, <= 100 Unicode scalars', () {
    test('a plain lift name is accepted', () {
      expect(ExerciseDraft(name: 'Bench press').nameError(), isNull);
    });

    test('empty and whitespace-only names are rejected', () {
      expect(ExerciseDraft(name: '').nameError(), isNotNull);
      expect(ExerciseDraft(name: '   ').nameError(), isNotNull);
    });

    test('length boundary: 100 chars accepted, 101 rejected', () {
      expect(ExerciseDraft(name: 'a' * 100).nameError(), isNull);
      expect(ExerciseDraft(name: 'a' * 101).nameError(), isNotNull);
    });

    test('the name is trimmed BEFORE counting (backend order)', () {
      // 104 raw chars, 100 after trim -> valid.
      expect(ExerciseDraft(name: '  ${'a' * 100}  ').nameError(), isNull);
    });

    test(
        'length is counted in Unicode scalars, not UTF-16 units '
        '(architect finding 6 pin)', () {
      final hundredFlexes = '💪' * 100;
      expect(
        hundredFlexes.length,
        200,
        reason: 'precondition: each 💪 is 2 UTF-16 code units',
      );
      expect(hundredFlexes.runes.length, 100);
      // 100 scalars must pass — a String.length implementation would
      // wrongly reject this at "200 chars".
      expect(ExerciseDraft(name: hundredFlexes).nameError(), isNull);
      expect(ExerciseDraft(name: '💪' * 101).nameError(), isNotNull);
    });
  });

  group('SAC2/SAC4 shape', () {
    test('holds an optional muscle group and ordered sets', () {
      final e = ExerciseDraft(
        name: 'Squat',
        muscleGroup: MuscleGroup.legs,
        sets: const [SetDraft(reps: 5), SetDraft(reps: 3)],
      );
      expect(e.name, 'Squat');
      expect(e.muscleGroup, MuscleGroup.legs);
      expect(e.sets.map((s) => s.reps), [5, 3]);
    });

    test('the muscle group defaults to absent (untagged free text)', () {
      expect(ExerciseDraft(name: 'Sled push').muscleGroup, isNull);
      expect(ExerciseDraft(name: 'Sled push').sets, isEmpty);
    });
  });
}
