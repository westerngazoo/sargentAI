// R-0061 AC5 -> SPEC-0061 §3 (slice 2d): the one primary action of the
// selected day, as a pure function of (day vs today, state), over every
// (tense × state) pair. Every index comes from the server.
//
// R-0061 AC2 -> SPEC-0061 §2.8 (slice 3b): "start workout" by voice starts
// what Empezar sesión would: the planned day, or the day still pending
// behind a free workout, else a free session.

import 'package:fitai/src/home/day_action.dart';
import 'package:fitai/src/program/models/day_state.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  final today = DateTime(2026, 10, 22, 15, 30); // time must not matter
  final yesterday = DateTime(2026, 10, 21);
  final tomorrow = DateTime(2026, 10, 23);

  DayAction act(DateTime day, DayState state) =>
      primaryAction(day: day, today: today, state: state);

  group('today', () {
    final day = DateTime(2026, 10, 22);

    test('planned: start that cycle day', () {
      final a = act(day, const PlannedDay(3)) as StartSession;
      expect(a.planDayIndex, 3);
    });

    test('rest: start a free session', () {
      final a = act(day, const RestDay(1)) as StartSession;
      expect(a.planDayIndex, isNull);
    });

    test('nothing (no program): start a free session', () {
      final a = act(day, const NothingDay()) as StartSession;
      expect(a.planDayIndex, isNull);
    });

    test('done with a plan session: see what was logged', () {
      expect(act(day, const DoneDay(completed: [0])), isA<ViewLog>());
    });

    test(
        'done with only a free workout: see the log, and the pending day '
        'stays startable (N1)', () {
      final a = act(day, const DoneDay(completed: [], pending: 2))
          as ViewLogWithPending;
      expect(a.pending, 2);
    });
  });

  group('a past day', () {
    test('nothing with a pending day: log it late with that index', () {
      final a = act(yesterday, const NothingDay(pending: 1)) as LogLate;
      expect(a.planDayIndex, 1);
    });

    test('nothing before a later plan log: log a free session late (N2)', () {
      final a = act(yesterday, const NothingDay()) as LogLate;
      expect(a.planDayIndex, isNull);
    });

    test('done, with or without a pending index: see what was logged', () {
      expect(act(yesterday, const DoneDay(completed: [0])), isA<ViewLog>());
      expect(act(yesterday, const DoneDay(completed: [], pending: 1)),
          isA<ViewLog>());
    });

    test('planned or rest cannot be past; they offer nothing', () {
      expect(act(yesterday, const PlannedDay(0)), isA<NoAction>());
      expect(act(yesterday, const RestDay(0)), isA<NoAction>());
    });
  });

  group('a future day', () {
    test('planned, rest or nothing: its projection and no action', () {
      expect(act(tomorrow, const PlannedDay(0)), isA<NoAction>());
      expect(act(tomorrow, const RestDay(0)), isA<NoAction>());
      expect(act(tomorrow, const NothingDay()), isA<NoAction>());
    });

    test('done (a future-dated log): see what was logged', () {
      expect(act(tomorrow, const DoneDay(completed: [])), isA<ViewLog>());
      expect(act(tomorrow, const DoneDay(completed: [], pending: 0)),
          isA<ViewLog>());
    });
  });

  group('startIndexToday (voice)', () {
    test('a planned day: its index', () {
      expect(startIndexToday(const PlannedDay(3), today: today), 3);
    });

    test('done with only a free workout: the pending day (N1)', () {
      expect(
          startIndexToday(const DoneDay(completed: [], pending: 0),
              today: today),
          0);
    });

    test('done with a plan session, rest or nothing: a free session', () {
      expect(
          startIndexToday(const DoneDay(completed: [1]), today: today), isNull);
      expect(startIndexToday(const RestDay(2), today: today), isNull);
      expect(startIndexToday(const NothingDay(), today: today), isNull);
    });
  });
}
