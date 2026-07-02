// SAC9 -> AC9 (application): SessionListController owns delete(id) with the
// failure-as-state pattern (architect finding 4) — submitting flag,
// ApiException caught to a state `error` (a 404 becomes the friendly
// "that workout no longer exists"), and workoutsProvider invalidated on
// success AND on 404 (the list is stale either way). No widget try/catch; no
// rethrow.
//
// RED until package:fitai/src/workout/application/session_list_controller.dart
// defines sessionListControllerProvider and SessionListController.

import 'dart:async';

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/workout/application/session_list_controller.dart';
import 'package:fitai/src/workout/application/workouts_provider.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/workout_fakes.dart';

void main() {
  setUpAll(registerWorkoutFallbacks);

  late MockWorkoutRepository repo;

  setUp(() {
    repo = MockWorkoutRepository();
    when(() => repo.list()).thenAnswer((_) async => [sampleSession(id: 's-1')]);
  });

  ProviderContainer makeContainer() {
    final container = ProviderContainer(
      overrides: [
        workoutRepositoryProvider.overrideWithValue(repo),
        authUserIdProvider.overrideWith((_) => 'u-test'),
      ],
    );
    addTearDown(container.dispose);
    return container;
  }

  SessionListController controllerOf(ProviderContainer c) =>
      c.read(sessionListControllerProvider.notifier);

  test('SAC9: a successful delete calls DELETE and invalidates the list',
      () async {
    when(() => repo.delete('s-1')).thenAnswer((_) async {});
    final c = makeContainer();

    await c.read(workoutsProvider.future);
    verify(() => repo.list()).called(1);

    await controllerOf(c).delete('s-1');

    expect(c.read(sessionListControllerProvider).error, isNull);
    verify(() => repo.delete('s-1')).called(1);

    // A fresh read after the delete must hit the repository again — the
    // cached value was invalidated.
    await c.read(workoutsProvider.future);
    verify(() => repo.list()).called(greaterThanOrEqualTo(1));
  });

  test(
      'SAC9: a 404 becomes a friendly message AND still invalidates (the '
      'list is stale either way)', () async {
    when(() => repo.delete('s-gone'))
        .thenThrow(const ApiException('not found', statusCode: 404));
    final c = makeContainer();

    await c.read(workoutsProvider.future);
    verify(() => repo.list()).called(1);

    await controllerOf(c).delete('s-gone'); // must NOT throw

    final s = c.read(sessionListControllerProvider);
    expect(s.error, isNotNull);
    expect(s.error!.toLowerCase(), contains('no longer exists'));

    await c.read(workoutsProvider.future);
    verify(() => repo.list()).called(greaterThanOrEqualTo(1));
  });

  test('SAC9: a transport failure surfaces the retryable message as state',
      () async {
    when(() => repo.delete('s-1'))
        .thenThrow(const ApiException("can't reach the server — retry"));
    final c = makeContainer();

    await controllerOf(c).delete('s-1'); // must NOT throw

    expect(c.read(sessionListControllerProvider).error, contains('retry'));
  });

  test('SAC9: submitting reflects the in-flight DELETE', () async {
    final gate = Completer<void>();
    when(() => repo.delete('s-1')).thenAnswer((_) => gate.future);
    final c = makeContainer();

    final pending = controllerOf(c).delete('s-1');
    await pumpEventQueue();
    expect(c.read(sessionListControllerProvider).submitting, isTrue);

    gate.complete();
    await pending;
    expect(c.read(sessionListControllerProvider).submitting, isFalse);
  });
}
