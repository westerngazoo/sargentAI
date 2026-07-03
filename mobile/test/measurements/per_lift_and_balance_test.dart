// Per-lift strength trends, PR detection, and muscle-group volume balance.

import 'package:fitai/src/measurements/application/strength_trend.dart';
import 'package:fitai/src/workout/domain/muscle_group.dart';
import 'package:fitai/src/workout/domain/workout_session.dart';
import 'package:flutter_test/flutter_test.dart';

WorkoutSession _s(
  DateTime on,
  String name,
  MuscleGroup? g,
  List<(int, double?)> sets,
) =>
    WorkoutSession(
      id: 'w-${on.toIso8601String()}-$name',
      userId: 'u',
      performedOn: on,
      exercises: [
        WorkoutExercise(
          id: 'e',
          position: 0,
          name: name,
          muscleGroup: g,
          sets: [
            for (var i = 0; i < sets.length; i++)
              WorkoutSet(
                id: 's$i',
                position: i,
                reps: sets[i].$1,
                weightKg: sets[i].$2,
              ),
          ],
        ),
      ],
      createdAt: on,
      updatedAt: on,
    );

void main() {
  final d1 = DateTime(2026, 5, 1);
  final d2 = DateTime(2026, 5, 8);
  final d3 = DateTime(2026, 5, 15);

  group('per-lift trends', () {
    test('keeps only lifts with >= 2 weighted sessions, name-cased', () {
      final lifts = computePerLiftTrends([
        _s(d1, 'Bench press', MuscleGroup.chest, [(5, 100.0)]),
        _s(d2, 'bench press', MuscleGroup.chest, [(5, 105.0)]),
        _s(d1, 'Squat', MuscleGroup.legs, [(5, 140.0)]), // only once
      ]);
      expect(lifts.map((l) => l.name), ['Bench press']);
      expect(lifts.single.sessions, 2);
    });

    test('detects a PR when the latest session is the all-time best', () {
      final up = computePerLiftTrends([
        _s(d1, 'Bench', null, [(5, 100.0)]),
        _s(d2, 'Bench', null, [(5, 105.0)]),
      ]).single;
      expect(up.isPr, isTrue);
      expect(up.gain.round(), 6); // 105*(7/6) - 100*(7/6) ≈ 5.83 → 6

      final down = computePerLiftTrends([
        _s(d1, 'Bench', null, [(5, 110.0)]),
        _s(d2, 'Bench', null, [(5, 100.0)]),
      ]).single;
      expect(down.isPr, isFalse);
    });

    test('ordered by most sessions first', () {
      final lifts = computePerLiftTrends([
        _s(d1, 'Squat', null, [(5, 140.0)]),
        _s(d2, 'Squat', null, [(5, 142.5)]),
        _s(d3, 'Squat', null, [(5, 145.0)]),
        _s(d1, 'Bench', null, [(5, 100.0)]),
        _s(d2, 'Bench', null, [(5, 102.5)]),
      ]);
      expect(lifts.first.name, 'Squat');
    });
  });

  group('muscle balance', () {
    test('sums volume per group, highest first, skips untagged', () {
      final bal = computeMuscleVolume([
        _s(d1, 'Squat', MuscleGroup.legs, [(5, 100.0), (5, 100.0)]), // 1000
        _s(d1, 'Bench', MuscleGroup.chest, [(5, 60.0)]), // 300
        _s(d1, 'Mystery', null, [(5, 50.0)]), // skipped
      ]);
      expect(bal.first.group, MuscleGroup.legs);
      expect(bal.first.volume, 1000);
      expect(bal.map((b) => b.group).contains(MuscleGroup.chest), isTrue);
      expect(bal.length, 2);
    });
  });
}
