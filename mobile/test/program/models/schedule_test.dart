// R-0061 AC4, AC6 -> SPEC-0061 §2.4 (slice 2d): the client parses
// `GET /programs/me/schedule` exactly as the server writes it — the tagged
// cycle entries, the four tagged day states with their indices, and the
// session ids per day. Every index comes from the server (§2.5).

import 'package:fitai/src/program/models/day_state.dart';
import 'package:fitai/src/program/models/schedule.dart';
import 'package:fitai/src/workout/domain/muscle_group.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/program_fakes.dart';

void main() {
  group('Schedule.fromJson', () {
    test('parses the §2.4 example', () {
      final s = Schedule.fromJson(scheduleJson(
        programId: 'p-1',
        startsOn: '2026-10-23',
        cycle: [
          trainingDayJson(focus: ['legs'], exercises: ['Back Squat']),
          restDayJson,
        ],
        days: [
          dayJson('2026-10-21', state: 'done', completed: [], pending: 0),
          dayJson('2026-10-22', state: 'nothing', pending: null),
          dayJson('2026-10-23', state: 'planned', cycleIndex: 0),
        ],
        sessionIds: {
          '2026-10-21': ['s-1', 's-2']
        },
      ));

      expect(s.programId, 'p-1');
      expect(s.startsOn, DateTime(2026, 10, 23));
      expect(s.cycle, hasLength(2));
      final training = s.cycle[0] as TrainingPlanDay;
      expect(training.focus, [MuscleGroup.legs]);
      expect(training.exercises, ['Back Squat']);
      expect(s.cycle[1], isA<RestPlanDay>());

      final done = s.stateOn(DateTime(2026, 10, 21)) as DoneDay;
      expect(done.completed, isEmpty);
      expect(done.pending, 0);
      final nothing = s.stateOn(DateTime(2026, 10, 22)) as NothingDay;
      expect(nothing.pending, isNull);
      final planned = s.stateOn(DateTime(2026, 10, 23)) as PlannedDay;
      expect(planned.cycleIndex, 0);
      expect(s.sessionIds[DateTime(2026, 10, 21)], ['s-1', 's-2']);
    });

    test('a day outside the response has no state', () {
      final s = sampleSchedule();
      expect(s.stateOn(DateTime(2026, 1, 1)), isNull);
    });

    test('stateOn ignores the time of day', () {
      final s = sampleSchedule(
          days: [dayJson('2026-10-23', state: 'rest', cycleIndex: 2)]);
      final rest = s.stateOn(DateTime(2026, 10, 23, 18, 45)) as RestDay;
      expect(rest.cycleIndex, 2);
    });

    test('the no-program response is empty, not an error', () {
      final s = Schedule.fromJson(scheduleJson(
        programId: null,
        days: [
          dayJson('2026-10-21', state: 'done', completed: [], pending: null),
        ],
      ));
      expect(s.programId, isNull);
      expect(s.startsOn, isNull);
      expect(s.cycle, isEmpty);
      expect(s.stateOn(DateTime(2026, 10, 21)), isA<DoneDay>());
    });

    test('trainingDay returns the cycle entry by index, or null', () {
      final s = sampleSchedule(); // [chest+arms, legs, rest]
      expect(s.trainingDay(1)?.focus, [MuscleGroup.legs]);
      expect(s.trainingDay(2), isNull, reason: 'a rest entry');
      expect(s.trainingDay(7), isNull, reason: 'out of range');
      expect(s.trainingDay(-1), isNull);
    });

    test('planSeed seeds a session from a training day, or nothing (§2.7)', () {
      final s = sampleSchedule(); // prog-uuid-001, [chest+arms, legs, rest]
      final seed = s.planSeed(1);
      expect(seed?.userProgramId, 'prog-uuid-001');
      expect(seed?.index, 1);
      expect(seed?.exercises, ['Back Squat', 'Leg Press']);
      expect(s.planSeed(2), isNull, reason: 'a rest day seeds nothing');
      expect(s.planSeed(7), isNull, reason: 'out of range');
      expect(sampleSchedule(programId: null).planSeed(0), isNull,
          reason: 'no program, no plan session');
    });
  });

  group('DayState.fromJson', () {
    test('done carries its completed indices and the pending one', () {
      final d = DayState.fromJson(
          dayJson('2026-10-21', state: 'done', completed: [1, 2], pending: 3));
      expect(d, isA<DoneDay>());
      expect((d as DoneDay).completed, [1, 2]);
      expect(d.pending, 3);
    });

    test('nothing carries a pending index or null', () {
      final a =
          DayState.fromJson(dayJson('2026-10-21', state: 'nothing', pending: 1))
              as NothingDay;
      expect(a.pending, 1);
      final b = DayState.fromJson(
          dayJson('2026-10-21', state: 'nothing', pending: null)) as NothingDay;
      expect(b.pending, isNull);
    });

    test('an unknown state token is a FormatException, never a crash', () {
      expect(
        () => DayState.fromJson({'date': '2026-10-21', 'state': 'missed'}),
        throwsFormatException,
      );
    });
  });

  test('an unknown cycle kind is a FormatException', () {
    expect(
      () => PlanDay.fromJson({'kind': 'warmup'}),
      throwsFormatException,
    );
  });
}
