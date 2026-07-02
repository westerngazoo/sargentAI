// SAC8 -> AC8 (provider layer): workoutsProvider is a
// FutureProvider<List<WorkoutSession>> over the repository (the
// profileProvider idiom, OQ-F3) — it resolves the GET /workouts list in
// SERVER order (newest performed_on first; the client never re-sorts) and
// propagates failures as an error AsyncValue.
//
// RED until package:fitai/src/workout/application/workouts_provider.dart
// defines workoutsProvider over workoutRepositoryProvider.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/workout/application/workouts_provider.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/workout_fakes.dart';

void main() {
  late MockWorkoutRepository repo;

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

  setUp(() {
    repo = MockWorkoutRepository();
  });

  test('SAC8: resolves the repository list PRESERVING server order', () async {
    when(() => repo.list()).thenAnswer((_) async => [
          sampleSession(id: 's-new', performedOn: '2026-06-10'),
          sampleSession(id: 's-old', performedOn: '2026-06-01'),
        ]);
    final container = makeContainer();

    final sessions = await container.read(workoutsProvider.future);

    expect(sessions.map((s) => s.id), ['s-new', 's-old'],
        reason: 'the server sorts newest-first; the client must not re-sort');
  });

  test(
      'SAC8: an empty backlog resolves to an empty list (drives AC8 empty '
      'state)', () async {
    when(() => repo.list()).thenAnswer((_) async => []);
    final container = makeContainer();
    expect(await container.read(workoutsProvider.future), isEmpty);
  });

  test('SAC8: a repository failure propagates as an error AsyncValue',
      () async {
    when(() => repo.list()).thenThrow(StateError('boom'));
    final container = makeContainer();
    await expectLater(
      container.read(workoutsProvider.future),
      throwsA(anything),
    );
  });
}
