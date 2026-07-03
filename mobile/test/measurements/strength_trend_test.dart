// Strength trend from the workout log.

import 'package:fitai/src/measurements/application/strength_trend.dart';
import 'package:fitai/src/workout/domain/workout_session.dart';
import 'package:flutter_test/flutter_test.dart';

WorkoutSession _session(DateTime on, List<(int reps, double? w)> sets) =>
    WorkoutSession(
      id: 'w-${on.toIso8601String()}',
      userId: 'u',
      performedOn: on,
      exercises: [
        WorkoutExercise(
          id: 'e',
          position: 0,
          name: 'Bench',
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

  test('best 1RM uses Epley over the best weighted set, sorted by date', () {
    // Feed out of order; expect chronological output.
    final t = computeStrengthTrend([
      _session(d2, [(5, 100.0)]), // 100*(1+5/30)=116.67
      _session(d1, [(5, 80.0), (3, 90.0)]), // max(80*1.1667, 90*1.1)=99
    ]);
    expect(t.best1rm.length, 2);
    expect(t.best1rm.first.date, d1);
    expect(t.best1rm.first.value, closeTo(99.0, 0.1)); // 90*(1+3/30)=99
    expect(t.best1rm.last.value, closeTo(116.67, 0.1));
  });

  test('volume is reps × weight summed per session', () {
    final t = computeStrengthTrend([
      _session(d1, [(10, 50.0), (8, 60.0)]), // 500 + 480 = 980
    ]);
    expect(t.volume.single.value, 980);
  });

  test('bodyweight-only sessions are skipped for 1RM and volume', () {
    final t = computeStrengthTrend([
      _session(d1, [(10, null)]),
    ]);
    expect(t.best1rm, isEmpty);
    expect(t.volume, isEmpty);
    expect(t.hasData, isFalse);
  });

  test('hasData needs at least two points', () {
    final one = computeStrengthTrend([
      _session(d1, [(5, 100.0)])
    ]);
    expect(one.hasData, isFalse);
    final two = computeStrengthTrend([
      _session(d1, [(5, 100.0)]),
      _session(d2, [(5, 105.0)]),
    ]);
    expect(two.hasData, isTrue);
  });
}
