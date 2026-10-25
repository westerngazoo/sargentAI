// SPEC-0061 §2.5 (slice 2d): `scheduleProvider(DateRange)` is a
// FutureProvider.family over GET /programs/me/schedule. The family caches
// on the range's value, the window asked for covers the calendar's month
// grid, and the cache drops when a workout is saved or deleted.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/program/application/schedule_provider.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:fitai/src/workout/application/session_driver.dart';
import 'package:fitai/src/workout/application/session_list_controller.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/program_fakes.dart';
import '../../support/workout_fakes.dart';

void main() {
  setUpAll(registerProgramFallbacks);
  setUpAll(registerWorkoutFallbacks);

  group('DateRange', () {
    test('is a value: equal dates are equal keys', () {
      final a = DateRange(DateTime(2026, 10, 1), DateTime(2026, 10, 31));
      final b = DateRange(DateTime(2026, 10, 1), DateTime(2026, 10, 31));
      expect(a, b);
      expect(a.hashCode, b.hashCode);
      expect(a, isNot(DateRange(DateTime(2026, 10, 1), DateTime(2026, 11, 1))));
    });
  });

  group('scheduleWindow', () {
    test('covers every cell of the month grid around the anchor', () {
      // A month grid is at most six weeks: up to six days before the 1st
      // and six after the last day (calendar.dart `_monthGrid`).
      for (final anchor in [
        DateTime(2026, 10, 25),
        DateTime(2026, 2, 1),
        DateTime(2024, 2, 29),
        DateTime(2026, 12, 31),
      ]) {
        final w = scheduleWindow(anchor);
        final first = DateTime(anchor.year, anchor.month, 1);
        final last = DateTime(anchor.year, anchor.month + 1, 0);
        expect(w.from.isAfter(DateTime(first.year, first.month, first.day - 6)),
            isFalse,
            reason: '$anchor');
        expect(w.to.isBefore(DateTime(last.year, last.month, last.day + 6)),
            isFalse,
            reason: '$anchor');
      }
    });

    test('never exceeds the endpoint\'s 62 inclusive days', () {
      for (var m = 1; m <= 12; m++) {
        final w = scheduleWindow(DateTime(2026, m, 15));
        final days = DateTime.utc(w.to.year, w.to.month, w.to.day)
                .difference(DateTime.utc(w.from.year, w.from.month, w.from.day))
                .inDays +
            1;
        expect(days, lessThanOrEqualTo(62), reason: 'month $m');
      }
    });

    test('is the same window for every day of one month (one request)', () {
      expect(scheduleWindow(DateTime(2026, 10, 3)),
          scheduleWindow(DateTime(2026, 10, 30)));
    });
  });

  group('scheduleProvider', () {
    late MockProgramService programService;
    late MockWorkoutRepository workoutRepo;
    late ProviderContainer container;
    final range = DateRange(DateTime(2026, 10, 1), DateTime(2026, 10, 31));

    setUp(() {
      programService = MockProgramService();
      workoutRepo = MockWorkoutRepository();
      whenGetSchedule(programService).thenAnswer((_) async => sampleSchedule());
      when(() => workoutRepo.list()).thenAnswer((_) async => []);
      when(() => workoutRepo.create(any()))
          .thenAnswer((_) async => sampleSession());
      when(() => workoutRepo.delete(any())).thenAnswer((_) async {});
      container = ProviderContainer(overrides: [
        authUserIdProvider.overrideWithValue('user-a'),
        programServiceProvider.overrideWithValue(programService),
        workoutRepositoryProvider.overrideWithValue(workoutRepo),
      ]);
      addTearDown(container.dispose);
    });

    test('asks the service for the range, with today as the local date',
        () async {
      await container.read(scheduleProvider(range).future);
      final now = DateTime.now();
      verify(() => programService.getSchedule(
            from: range.from,
            to: range.to,
            today: DateTime(now.year, now.month, now.day),
          )).called(1);
    });

    test('the same range is one request; an equal range object hits the cache',
        () async {
      await container.read(scheduleProvider(range).future);
      await container.read(scheduleProvider(
              DateRange(DateTime(2026, 10, 1), DateTime(2026, 10, 31)))
          .future);
      verify(() => programService.getSchedule(
            from: any(named: 'from'),
            to: any(named: 'to'),
            today: any(named: 'today'),
          )).called(1);
    });

    test('a saved workout drops the cache', () async {
      await container.read(scheduleProvider(range).future);
      final driver = container.read(sessionDriverProvider.notifier);
      driver.start();
      driver.addExercise('Bench press');
      driver.logSet(const SetDraft(reps: 5));
      await driver.finish();
      await container.read(scheduleProvider(range).future);
      verify(() => programService.getSchedule(
            from: any(named: 'from'),
            to: any(named: 'to'),
            today: any(named: 'today'),
          )).called(2);
    });

    test('a deleted workout drops the cache', () async {
      await container.read(scheduleProvider(range).future);
      await container.read(sessionListControllerProvider.notifier).delete('s');
      await container.read(scheduleProvider(range).future);
      verify(() => programService.getSchedule(
            from: any(named: 'from'),
            to: any(named: 'to'),
            today: any(named: 'today'),
          )).called(2);
    });
  });
}
