// SAC1-SAC7/SAC10 -> AC1-AC7/AC10: the session driver — the R-0027 seam.
//
// EVERY test in this file is a plain `test()`: no widget is ever pumped. That
// is the SAC10 proof that the driver's full API (start / addExercise / logSet /
// selectExercise / finish / abandon) is consumable by a non-UI transport (the
// R-0027 earbud mode drives exactly this surface by voice).
//
// The architect-amended contract (SPEC-0009 §2.2, findings 1/2/3/7) is pinned:
//   * the driver is the SINGLE validation-enforcement point: addExercise and
//     logSet REJECT invalid input — never appended — returning the reason
//     synchronously (`null` = accepted); the same message is NOT put on
//     state.error (that channel is reserved for finish/network failures);
//   * selectExercise clamps: out-of-range is a no-op;
//   * canFinish / lastSet are derived reactively on SessionDriverState;
//     lastSet is the last set of the CURRENT exercise (OQ-F4);
//   * finish() guards on canFinish (no-op otherwise), stamps the LOCAL
//     calendar date (the one clock read, at the driver edge — finding 2),
//     POSTs exactly the logged content, and re-reads workoutsProvider BEFORE
//     setting done (AC5); ApiException -> error/errorField as DATA on the
//     state, draft untouched, NO rethrow (AC6);
//   * abandon() discards the draft (confirmation is the UI's job — AC7).
//
// RED until package:fitai/src/workout/application/session_driver.dart defines
// sessionDriverProvider, SessionDriver, and SessionDriverState.

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/workout/application/session_driver.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:fitai/src/workout/domain/muscle_group.dart';
import 'package:fitai/src/workout/domain/session_draft.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:fitai/src/workout/domain/workout_session.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/workout_fakes.dart';

void main() {
  setUpAll(registerWorkoutFallbacks);

  late MockWorkoutRepository repo;

  setUp(() {
    repo = MockWorkoutRepository();
  });

  ProviderContainer makeContainer() {
    final container = ProviderContainer(
      overrides: [
        workoutRepositoryProvider.overrideWithValue(repo),
      ],
    );
    addTearDown(container.dispose);
    return container;
  }

  SessionDriver driverOf(ProviderContainer c) =>
      c.read(sessionDriverProvider.notifier);
  SessionDriverState stateOf(ProviderContainer c) =>
      c.read(sessionDriverProvider);

  void stubHappyNetwork() {
    when(() => repo.create(any())).thenAnswer((_) async => sampleSession());
    when(() => repo.list()).thenAnswer((_) async => [sampleSession()]);
  }

  group('SAC1/SAC4 start + initial state', () {
    test('before start there is no draft and nothing is finishable', () {
      final c = makeContainer();
      final s = stateOf(c);
      expect(s.draft, isNull);
      expect(s.submitting, isFalse);
      expect(s.error, isNull);
      expect(s.errorField, isNull);
      expect(s.done, isFalse);
      expect(s.canFinish, isFalse);
      expect(s.lastSet, isNull);
    });

    test(
        'start() opens an empty in-memory session (no date is typed — '
        'performed_on is stamped only at finish)', () {
      final c = makeContainer();
      driverOf(c).start();
      final s = stateOf(c);
      expect(s.draft, isNotNull);
      expect(s.draft!.exercises, isEmpty);
      expect(s.canFinish, isFalse);
      expect(s.error, isNull);
      expect(s.done, isFalse);
    });
  });

  group('SAC2 addExercise — driver-enforced name validation (finding 1)', () {
    test('a valid name is accepted (null), appended, and selected', () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();

      expect(d.addExercise('Bench press'), isNull);
      expect(stateOf(c).draft!.exercises.single.name, 'Bench press');
      expect(stateOf(c).currentExercise, 0);

      expect(d.addExercise('Row'), isNull);
      expect(
        stateOf(c).draft!.exercises.map((e) => e.name),
        ['Bench press', 'Row'],
        reason: 'exercises stay ordered',
      );
      expect(stateOf(c).currentExercise, 1,
          reason: 'adding selects the new exercise');
    });

    test('the stored name is trimmed (backend semantics)', () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      expect(d.addExercise('  Bench press  '), isNull);
      expect(stateOf(c).draft!.exercises.single.name, 'Bench press');
    });

    test('an optional muscle group is recorded; absent stays null', () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Squat', group: MuscleGroup.legs);
      d.addExercise('Sled push');
      final exercises = stateOf(c).draft!.exercises;
      expect(exercises[0].muscleGroup, MuscleGroup.legs);
      expect(exercises[1].muscleGroup, isNull);
    });

    test(
        'an invalid name is REJECTED with a reason — draft untouched, '
        'error channel clean', () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();

      expect(d.addExercise('   '), isNotNull);
      expect(d.addExercise('a' * 101), isNotNull);

      final s = stateOf(c);
      expect(s.draft!.exercises, isEmpty, reason: 'never appended');
      expect(s.error, isNull,
          reason: 'state.error is reserved for finish/network failures');
    });

    test('name length is counted in Unicode scalars at the driver edge too',
        () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      expect(d.addExercise('💪' * 100), isNull,
          reason: '100 scalars (200 UTF-16 units) must pass');
      expect(d.addExercise('💪' * 101), isNotNull);
    });
  });

  group('SAC3 logSet — driver-enforced set validation (finding 1)', () {
    test('a valid set is accepted (null) and appended to the current exercise',
        () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Bench press');

      expect(d.logSet(const SetDraft(reps: 8, weightKg: 80, rpe: 8.5)), isNull);

      final sets = stateOf(c).draft!.exercises.single.sets;
      expect(sets, hasLength(1));
      expect(sets.single.reps, 8);
      expect(sets.single.weightKg, 80.0);
      expect(sets.single.rpe, 8.5);
    });

    test('an invalid set is rejected with a reason and never appended', () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Bench press');

      expect(d.logSet(const SetDraft(reps: 0)), isNotNull);
      expect(d.logSet(const SetDraft(reps: 8, rpe: 7.3)), isNotNull,
          reason: '7.3 is off the 0.5 grid');
      expect(d.logSet(const SetDraft(reps: 8, weightKg: 0)), isNotNull,
          reason: 'weight lower bound is exclusive');

      expect(stateOf(c).draft!.exercises.single.sets, isEmpty);
      expect(stateOf(c).error, isNull, reason: 'reserved channel untouched');

      expect(d.logSet(const SetDraft(reps: 8, rpe: 7.5)), isNull,
          reason: '7.5 is on the 0.5 grid');
    });

    test('logSet with no current exercise cannot be accepted', () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      expect(d.logSet(const SetDraft(reps: 5)), isNotNull,
          reason: 'null means accepted — nothing was');
      expect(stateOf(c).draft!.exercises, isEmpty);
    });

    test('logSet before start() cannot be accepted (total API, no throw)', () {
      final c = makeContainer();
      final d = driverOf(c);
      expect(d.logSet(const SetDraft(reps: 5)), isNotNull);
      expect(stateOf(c).draft, isNull);
    });
  });

  group('SAC4 selectExercise + lastSet', () {
    test('switching back to an earlier exercise appends sets THERE', () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Bench press');
      d.logSet(const SetDraft(reps: 8));
      d.addExercise('Row');
      d.logSet(const SetDraft(reps: 12));

      d.selectExercise(0);
      expect(stateOf(c).currentExercise, 0);
      d.logSet(const SetDraft(reps: 6));

      final exercises = stateOf(c).draft!.exercises;
      expect(exercises[0].sets.map((s) => s.reps), [8, 6]);
      expect(exercises[1].sets.map((s) => s.reps), [12],
          reason: 'nothing leaked into the other exercise');
    });

    test('selectExercise clamps — out-of-range is a no-op (finding 7)', () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Bench press');
      d.addExercise('Row');
      expect(stateOf(c).currentExercise, 1);

      d.selectExercise(5);
      expect(stateOf(c).currentExercise, 1, reason: 'past the end: no-op');
      d.selectExercise(-1);
      expect(stateOf(c).currentExercise, 1, reason: 'negative: no-op');
      d.selectExercise(0);
      expect(stateOf(c).currentExercise, 0, reason: 'in range: switches');
    });

    test('lastSet is the last set of the CURRENT exercise (OQ-F4)', () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Bench press');
      d.logSet(const SetDraft(reps: 8, weightKg: 80, rpe: 8.5));

      d.addExercise('Row');
      expect(stateOf(c).lastSet, isNull,
          reason: 'the newly selected exercise has no sets yet');

      d.selectExercise(0);
      final last = stateOf(c).lastSet;
      expect(last, isNotNull);
      expect(last!.reps, 8);
      expect(last.weightKg, 80.0);
      expect(last.rpe, 8.5);

      d.logSet(const SetDraft(reps: 10));
      expect(stateOf(c).lastSet!.reps, 10, reason: 'derived reactively');
    });
  });

  group('SAC5 canFinish + finish', () {
    test('canFinish: >= 1 exercise AND every exercise >= 1 set', () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      expect(stateOf(c).canFinish, isFalse, reason: 'no exercises');

      d.addExercise('Bench press');
      expect(stateOf(c).canFinish, isFalse, reason: 'set-less exercise');

      d.logSet(const SetDraft(reps: 8));
      expect(stateOf(c).canFinish, isTrue);

      d.addExercise('Row');
      expect(stateOf(c).canFinish, isFalse,
          reason: 'ANY set-less exercise blocks finishing');

      d.logSet(const SetDraft(reps: 12));
      expect(stateOf(c).canFinish, isTrue);
    });

    test('finish() is a no-op while the session is not finishable', () async {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Bench press'); // no sets

      await d.finish();

      verifyNever(() => repo.create(any()));
      expect(stateOf(c).done, isFalse);
    });

    test(
        'finish() POSTs exactly the logged content with a LOCAL performed_on '
        '(finding 2)', () async {
      stubHappyNetwork();
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Bench press', group: MuscleGroup.chest);
      d.logSet(const SetDraft(reps: 8, weightKg: 80, rpe: 8.5));
      d.logSet(const SetDraft(reps: 10));
      d.addExercise('Pull-up');
      d.logSet(const SetDraft(reps: 12));

      final before = DateTime.now();
      await d.finish();
      final after = DateTime.now();

      final req = verify(() => repo.create(captureAny())).captured.single
          as SessionRequest;
      final json = req.toJson();
      expect(
        [isoDate(before), isoDate(after)],
        contains(json['performed_on']),
        reason: 'the LOCAL calendar date, stamped at the driver edge',
      );
      expect(json['exercises'], [
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
      ]);
    });

    test(
        'the sessions list is re-read BEFORE done flips (home shows the '
        'session on arrival)', () async {
      final listGate = Completer<List<WorkoutSession>>();
      when(() => repo.create(any())).thenAnswer((_) async => sampleSession());
      when(() => repo.list()).thenAnswer((_) => listGate.future);
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Squat');
      d.logSet(const SetDraft(reps: 5));

      final pending = d.finish();
      await pumpEventQueue();

      verify(() => repo.create(any())).called(1);
      expect(stateOf(c).done, isFalse,
          reason: 'the list re-read is still in flight — done must wait');

      listGate.complete([sampleSession()]);
      await pending;
      expect(stateOf(c).done, isTrue);
    });

    test('a 201 clears the draft and sets done', () async {
      stubHappyNetwork();
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Squat');
      d.logSet(const SetDraft(reps: 5));

      await d.finish();

      final s = stateOf(c);
      expect(s.done, isTrue);
      expect(s.draft, isNull,
          reason: 'post-finish /session must redirect home (OQ-F5)');
      expect(s.submitting, isFalse);
      expect(s.error, isNull);
    });

    test('submitting reflects the in-flight POST', () async {
      final createGate = Completer<WorkoutSession>();
      when(() => repo.create(any())).thenAnswer((_) => createGate.future);
      when(() => repo.list()).thenAnswer((_) async => [sampleSession()]);
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Squat');
      d.logSet(const SetDraft(reps: 5));

      final pending = d.finish();
      await pumpEventQueue();
      expect(stateOf(c).submitting, isTrue);

      createGate.complete(sampleSession());
      await pending;
      expect(stateOf(c).submitting, isFalse);
    });
  });

  group('SAC6 finish failure is DATA on the state (no rethrow, no data loss)',
      () {
    test('a 400{field} surfaces error + errorField; the draft is intact',
        () async {
      when(() => repo.create(any())).thenThrow(
        const ApiException('please check your details',
            statusCode: 400, field: 'rpe'),
      );
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Bench press');
      d.logSet(const SetDraft(reps: 8, rpe: 8.5));

      await d.finish(); // must NOT throw

      final s = stateOf(c);
      expect(s.error, 'please check your details');
      expect(s.errorField, 'rpe');
      expect(s.done, isFalse);
      expect(s.submitting, isFalse);
      expect(s.draft, isNotNull, reason: 'a failed finish loses NOTHING');
      expect(s.draft!.exercises.single.name, 'Bench press');
      expect(s.draft!.exercises.single.sets.single.reps, 8);
    });

    test(
        'a transport failure is retryable — retry re-submits the SAME '
        'session', () async {
      when(() => repo.create(any()))
          .thenThrow(const ApiException("can't reach the server — retry"));
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Bench press', group: MuscleGroup.chest);
      d.logSet(const SetDraft(reps: 8, weightKg: 80, rpe: 8.5));

      await d.finish();
      expect(stateOf(c).error, contains('retry'));
      expect(stateOf(c).done, isFalse);
      expect(stateOf(c).draft, isNotNull);

      when(() => repo.create(any())).thenAnswer((_) async => sampleSession());
      when(() => repo.list()).thenAnswer((_) async => [sampleSession()]);
      await d.finish();

      final captured = verify(() => repo.create(captureAny()))
          .captured
          .cast<SessionRequest>();
      expect(captured, hasLength(2));
      expect(
        jsonEncode(captured[0].toJson()['exercises']),
        jsonEncode(captured[1].toJson()['exercises']),
        reason: 'the retry submits the same logged content',
      );
      expect(stateOf(c).done, isTrue);
      expect(stateOf(c).error, isNull, reason: 'failure clears on success');
    });

    test(
        'a 401 is data too — no rethrow, no done (the shared interceptor '
        'owns the session sink)', () async {
      when(() => repo.create(any()))
          .thenThrow(const ApiException('unauthorized', statusCode: 401));
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Squat');
      d.logSet(const SetDraft(reps: 5));

      await d.finish(); // must NOT throw

      expect(stateOf(c).done, isFalse);
      expect(stateOf(c).error, isNotNull);
      expect(stateOf(c).draft, isNotNull);
    });
  });

  group('SAC7 abandon', () {
    test('abandon() discards the draft; nothing was persisted', () {
      final c = makeContainer();
      final d = driverOf(c);
      d.start();
      d.addExercise('Squat');
      d.logSet(const SetDraft(reps: 5));

      d.abandon();

      expect(stateOf(c).draft, isNull);
      expect(stateOf(c).canFinish, isFalse);
      verifyNever(() => repo.create(any()));
    });

    test('abandon() with no draft is harmless (total API)', () {
      final c = makeContainer();
      driverOf(c).abandon();
      expect(stateOf(c).draft, isNull);
    });
  });

  group('SAC10 the R-0027 seam', () {
    test(
        'the driver source imports no widget layer and nothing from '
        'presentation/', () {
      final file = File('lib/src/workout/application/session_driver.dart');
      expect(file.existsSync(), isTrue,
          reason: 'the driver lives in application/, not behind a widget');
      final imports = file
          .readAsLinesSync()
          .where((l) => l.trimLeft().startsWith('import '));
      for (final line in imports) {
        expect(line, isNot(contains('package:flutter/material.dart')),
            reason: line);
        expect(line, isNot(contains('package:flutter/widgets.dart')),
            reason: line);
        expect(line, isNot(contains('package:flutter/cupertino.dart')),
            reason: line);
        expect(line, isNot(contains('presentation/')), reason: line);
      }
    });
  });
}
