// Weekly plan progress computation.

import 'package:fitai/src/program/application/program_progress.dart';
import 'package:fitai/src/workout/domain/workout_session.dart';
import 'package:flutter_test/flutter_test.dart';

WorkoutSession _session(DateTime on) => WorkoutSession(
      id: 'w-${on.toIso8601String()}',
      userId: 'u',
      performedOn: on,
      exercises: const [],
      createdAt: on,
      updatedAt: on,
    );

void main() {
  // A fixed Wednesday so week boundaries are unambiguous.
  final wed = DateTime(2026, 7, 1); // 2026-07-01 is a Wednesday
  final mon = DateTime(2026, 6, 29); // that week's Monday
  final lastWeek = DateTime(2026, 6, 24);

  test('counts distinct workout days in the current week', () {
    final p = computeWeeklyProgress(
      workouts: [_session(mon), _session(DateTime(2026, 6, 30)), _session(wed)],
      daysTarget: 4,
      now: wed,
    );
    expect(p.daysDone, 3);
    expect(p.daysTarget, 4);
    expect(p.percent, 75);
    expect(p.weekComplete, isFalse);
  });

  test('two sessions on the same day count once', () {
    final p = computeWeeklyProgress(
      workouts: [_session(mon), _session(mon)],
      daysTarget: 4,
      now: wed,
    );
    expect(p.daysDone, 1);
  });

  test('last week does not count toward this week', () {
    final p = computeWeeklyProgress(
      workouts: [_session(lastWeek)],
      daysTarget: 4,
      now: wed,
    );
    expect(p.daysDone, 0);
    expect(p.percent, 0);
  });

  test('ratio clamps at 100% when the target is exceeded', () {
    final p = computeWeeklyProgress(
      workouts: [
        _session(mon),
        _session(DateTime(2026, 6, 30)),
        _session(wed),
      ],
      daysTarget: 2,
      now: wed,
    );
    expect(p.ratio, 1.0);
    expect(p.percent, 100);
    expect(p.weekComplete, isTrue);
  });

  test('totalSessions counts only sessions since the program was chosen', () {
    final p = computeWeeklyProgress(
      workouts: [_session(lastWeek), _session(mon), _session(wed)],
      daysTarget: 4,
      now: wed,
      programChosenAt: DateTime(2026, 6, 28),
    );
    expect(p.totalSessions, 2); // mon + wed, not lastWeek
  });

  test('zero target yields zero ratio (no divide-by-zero)', () {
    final p = computeWeeklyProgress(workouts: [], daysTarget: 0, now: wed);
    expect(p.ratio, 0);
    expect(p.percent, 0);
    expect(p.weekComplete, isFalse);
  });
}
