// Weekly plan progress computation, and the weekly target read from the
// projection (SPEC-0061 §2.1, N6).

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/program/application/program_progress.dart';
import 'package:fitai/src/program/application/program_providers.dart';
import 'package:fitai/src/program/application/schedule_provider.dart';
import 'package:fitai/src/program/models/day_state.dart';
import 'package:fitai/src/program/models/schedule.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:fitai/src/workout/application/workouts_provider.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:fitai/src/workout/domain/workout_session.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/program_fakes.dart';
import '../../support/workout_fakes.dart';

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

  group('weeklyTrainingTarget (N6)', () {
    Schedule schedule(Map<DateTime, DayState> days) => Schedule(
          programId: 'p',
          startsOn: mon,
          cycle: const [],
          days: days,
          sessionIds: const {},
        );

    test('counts the planned days and the done-with-plan days of the week', () {
      final s = schedule({
        mon: const DoneDay(completed: [0]),
        DateTime(2026, 6, 30): const DoneDay(completed: [], pending: 1),
        wed: const PlannedDay(1),
        DateTime(2026, 7, 2): const RestDay(2),
        DateTime(2026, 7, 3): const PlannedDay(0),
        DateTime(2026, 7, 4): const NothingDay(),
        DateTime(2026, 7, 5): const PlannedDay(1), // Sunday, still this week
      });
      expect(weeklyTrainingTarget(schedule: s, now: wed), 4);
    });

    test('the week is Monday to Sunday: other weeks do not count', () {
      final s = schedule({
        DateTime(2026, 6, 28): const PlannedDay(0), // the Sunday before
        mon: const PlannedDay(0),
        DateTime(2026, 7, 6): const PlannedDay(1), // next Monday
      });
      expect(weeklyTrainingTarget(schedule: s, now: wed), 1);
    });

    test('a free workout, a rest day and an empty day are not training days',
        () {
      final s = schedule({
        mon: const DoneDay(completed: []),
        wed: const RestDay(0),
        DateTime(2026, 7, 2): const NothingDay(pending: 1),
      });
      expect(weeklyTrainingTarget(schedule: s, now: wed), 0);
    });

    test('a user with no program has a target of zero', () {
      expect(weeklyTrainingTarget(schedule: sampleSchedule(), now: wed), 0);
    });
  });

  group('weeklyProgressProvider', () {
    setUpAll(registerProgramFallbacks);

    test('feeds the target from the schedule, not from daysPerWeek', () async {
      final now = DateTime.now();
      final today = DateTime(now.year, now.month, now.day);
      final weekStart =
          DateTime(today.year, today.month, today.day - (today.weekday - 1));
      String iso(int offset) => isoDate(
          DateTime(weekStart.year, weekStart.month, weekStart.day + offset));

      final programService = MockProgramService();
      final workoutRepo = MockWorkoutRepository();
      when(() => programService.getCurrent())
          .thenAnswer((_) async => sampleUserProgram()); // daysPerWeek: 4
      when(() => workoutRepo.list()).thenAnswer((_) async => []);
      whenGetSchedule(programService).thenAnswer((_) async => sampleSchedule(
            days: [
              dayJson(iso(0), state: 'done', completed: [0]),
              dayJson(iso(2), state: 'planned', cycleIndex: 1),
              dayJson(iso(4), state: 'planned', cycleIndex: 0),
              dayJson(iso(6), state: 'rest', cycleIndex: 2),
            ],
          ));
      final container = ProviderContainer(overrides: [
        authUserIdProvider.overrideWithValue('u'),
        programServiceProvider.overrideWithValue(programService),
        workoutRepositoryProvider.overrideWithValue(workoutRepo),
      ]);
      addTearDown(container.dispose);

      expect(container.read(weeklyProgressProvider), isNull,
          reason: 'null while the sources load');
      await container.read(currentProgramProvider.future);
      await container.read(workoutsProvider.future);
      await container.read(scheduleProvider(scheduleWindow(today)).future);

      expect(container.read(weeklyProgressProvider)?.daysTarget, 3);
    });
  });
}
