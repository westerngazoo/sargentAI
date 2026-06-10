// SAC8 -> AC8 (domain): WorkoutSession.fromJson parses the R-0004 aggregate as
// the backend serializes it (`core::workout::{WorkoutSession,WorkoutExercise,
// WorkoutSet}` — transparent newtypes, plain scalars on the wire): id, user_id,
// performed_on, exercises[{id, position, name, muscle_group?, sets[{id,
// position, reps, weight_kg?, rpe?}]}], created_at, updated_at.
//
// RED until package:fitai/src/workout/domain/workout_session.dart defines the
// three read models with fromJson.

import 'package:fitai/src/workout/domain/muscle_group.dart';
import 'package:fitai/src/workout/domain/workout_session.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/workout_fakes.dart';

void main() {
  group('SAC8 WorkoutSession.fromJson', () {
    test('parses the full aggregate, preserving exercise and set order', () {
      final s = WorkoutSession.fromJson(sessionResponseJson(
        id: 'sess-9',
        userId: 'u-9',
        performedOn: '2026-06-08',
        exercises: [
          exerciseResponseJson(
            id: 'ex-1',
            position: 1,
            name: 'Bench press',
            muscleGroup: 'chest',
            sets: [
              setResponseJson(
                id: 'set-1',
                position: 1,
                reps: 8,
                weightKg: 80.0,
                rpe: 8.5,
              ),
              setResponseJson(id: 'set-2', position: 2, reps: 10),
            ],
          ),
          exerciseResponseJson(
            id: 'ex-2',
            position: 2,
            name: 'Pull-up',
            sets: [setResponseJson(id: 'set-3', reps: 12)],
          ),
        ],
        createdAt: '2026-06-08T10:00:00Z',
        updatedAt: '2026-06-08T11:00:00Z',
      ));

      expect(s.id, 'sess-9');
      expect(s.userId, 'u-9');
      expect(s.performedOn, DateTime.parse('2026-06-08'));
      expect(s.createdAt, DateTime.parse('2026-06-08T10:00:00Z'));
      expect(s.updatedAt, DateTime.parse('2026-06-08T11:00:00Z'));

      expect(s.exercises, hasLength(2));
      final bench = s.exercises[0];
      expect(bench.id, 'ex-1');
      expect(bench.position, 1);
      expect(bench.name, 'Bench press');
      expect(bench.muscleGroup, MuscleGroup.chest);
      expect(bench.sets, hasLength(2));
      expect(bench.sets[0].id, 'set-1');
      expect(bench.sets[0].position, 1);
      expect(bench.sets[0].reps, 8);
      expect(bench.sets[0].weightKg, 80.0);
      expect(bench.sets[0].rpe, 8.5);
      expect(bench.sets[1].reps, 10);
      expect(bench.sets[1].weightKg, isNull);
      expect(bench.sets[1].rpe, isNull);

      final pullUp = s.exercises[1];
      expect(pullUp.name, 'Pull-up');
      expect(pullUp.muscleGroup, isNull, reason: 'untagged exercise');
      expect(pullUp.sets.single.reps, 12);
    });

    test('an integer weight_kg on the wire parses as a double', () {
      // JSON has no int/double distinction — a whole-number f64 arrives as a
      // Dart int. The parser must go through `num` (the Profile.fromJson
      // idiom), not cast `as double`.
      final s = WorkoutSession.fromJson(sessionResponseJson(
        exercises: [
          exerciseResponseJson(
            sets: [
              <String, dynamic>{
                'id': 'set-1',
                'position': 1,
                'reps': 8,
                'weight_kg': 80,
                'rpe': null,
              },
            ],
          ),
        ],
      ));
      expect(s.exercises.single.sets.single.weightKg, 80.0);
    });
  });
}
