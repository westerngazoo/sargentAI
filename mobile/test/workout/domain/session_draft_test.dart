// SAC5 -> AC5 (domain): SessionDraft is the in-progress session —
//   * canFinish mirrors WorkoutError::{ExercisesEmpty,SetsEmpty}: >= 1
//     exercise AND every exercise >= 1 set;
//   * toRequest(today) is TOTAL (`SessionRequest?` — null until canFinish, no
//     precondition throw);
//   * the request JSON matches the backend SessionRequest DTO exactly:
//     performed_on 'YYYY-MM-DD', exercises[{name, muscle_group?, sets[{reps,
//     weight_kg?, rpe?}]}] — omitted optionals are ABSENT KEYS, not nulls
//     (the `#[serde(default)]` DTOs in backend/crates/api/src/workout).
//
// RED until package:fitai/src/workout/domain/session_draft.dart defines
// SessionDraft and SessionRequest.

import 'package:fitai/src/workout/domain/exercise_draft.dart';
import 'package:fitai/src/workout/domain/muscle_group.dart';
import 'package:fitai/src/workout/domain/session_draft.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('SAC5 canFinish — backend structural rules', () {
    test('an empty session cannot finish (ExercisesEmpty)', () {
      expect(const SessionDraft().canFinish, isFalse);
    });

    test('a set-less exercise blocks finishing (SetsEmpty)', () {
      final draft = SessionDraft(
        exercises: [ExerciseDraft(name: 'Squat')],
      );
      expect(draft.canFinish, isFalse);
    });

    test('ANY set-less exercise blocks finishing, not just the last', () {
      final draft = SessionDraft(exercises: [
        ExerciseDraft(name: 'Squat', sets: const [SetDraft(reps: 5)]),
        ExerciseDraft(name: 'Lunge'),
      ]);
      expect(draft.canFinish, isFalse);
    });

    test('one exercise with one set is finishable', () {
      final draft = SessionDraft(exercises: [
        ExerciseDraft(name: 'Squat', sets: const [SetDraft(reps: 5)]),
      ]);
      expect(draft.canFinish, isTrue);
    });
  });

  group('SAC5 toRequest — total, null until finishable', () {
    test('returns null for an empty session', () {
      expect(const SessionDraft().toRequest(DateTime(2026, 6, 10)), isNull);
    });

    test('returns null while an exercise has no sets', () {
      final draft = SessionDraft(
        exercises: [ExerciseDraft(name: 'Squat')],
      );
      expect(draft.toRequest(DateTime(2026, 6, 10)), isNull);
    });

    test('emits EXACTLY the logged content — omitted optionals are ABSENT', () {
      final draft = SessionDraft(exercises: [
        ExerciseDraft(
          name: 'Bench press',
          muscleGroup: MuscleGroup.chest,
          sets: const [
            SetDraft(reps: 8, weightKg: 80, rpe: 8.5),
            SetDraft(reps: 10),
          ],
        ),
        ExerciseDraft(name: 'Pull-up', sets: const [SetDraft(reps: 12)]),
      ]);

      final req = draft.toRequest(DateTime(2026, 6, 10));
      expect(req, isNotNull);
      expect(req!.toJson(), <String, dynamic>{
        'performed_on': '2026-06-10',
        'exercises': [
          {
            'name': 'Bench press',
            'muscle_group': 'chest',
            'sets': [
              {'reps': 8, 'weight_kg': 80.0, 'rpe': 8.5},
              {'reps': 10},
            ],
          },
          {
            'name': 'Pull-up',
            'sets': [
              {'reps': 12},
            ],
          },
        ],
      });
    });

    test('performed_on is zero-padded YYYY-MM-DD (NaiveDate wire format)', () {
      final draft = SessionDraft(exercises: [
        ExerciseDraft(name: 'Squat', sets: const [SetDraft(reps: 5)]),
      ]);
      final json = draft.toRequest(DateTime(2026, 1, 5))!.toJson();
      expect(json['performed_on'], '2026-01-05');
    });
  });
}
