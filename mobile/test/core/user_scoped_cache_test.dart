// Regression — user-scoped caches must drop on account switch.
//
// A fresh registration after a logout must never render the previous user's
// program, workouts, or profile from stale FutureProvider caches. The three
// user-scoped providers watch [authUserIdProvider]; a userId change rebuilds
// them (fresh fetch under the new bearer token).

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/profile/application/profile_providers.dart';
import 'package:fitai/src/program/application/program_providers.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:fitai/src/workout/application/workouts_provider.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/profile_fakes.dart';
import '../support/program_fakes.dart';
import '../support/workout_fakes.dart';

void main() {
  setUpAll(registerFallbacks);
  setUpAll(registerProfileFallbacks);
  setUpAll(registerProgramFallbacks);

  late MockProgramService programService;
  late MockWorkoutRepository workoutRepo;
  late MockProfileRepository profileRepo;

  /// Container where [authUserIdProvider] is driven by a mutable state
  /// provider, standing in for logout → login as someone else.
  late StateProvider<String?> userId;
  late ProviderContainer container;

  setUp(() {
    programService = MockProgramService();
    workoutRepo = MockWorkoutRepository();
    profileRepo = MockProfileRepository();
    when(() => programService.getCurrent()).thenAnswer((_) async => null);
    when(() => workoutRepo.list()).thenAnswer((_) async => []);
    when(() => profileRepo.getMe()).thenAnswer((_) async => null);

    userId = StateProvider<String?>((_) => 'user-a');
    container = ProviderContainer(
      overrides: [
        authUserIdProvider.overrideWith((ref) => ref.watch(userId)),
        programServiceProvider.overrideWithValue(programService),
        workoutRepositoryProvider.overrideWithValue(workoutRepo),
        profileRepositoryProvider.overrideWithValue(profileRepo),
      ],
    );
    addTearDown(container.dispose);
  });

  test('currentProgramProvider refetches when the user changes', () async {
    await container.read(currentProgramProvider.future);
    verify(() => programService.getCurrent()).called(1);

    container.read(userId.notifier).state = 'user-b';
    await container.read(currentProgramProvider.future);
    verify(() => programService.getCurrent()).called(1);
  });

  test('workoutsProvider refetches when the user changes', () async {
    await container.read(workoutsProvider.future);
    verify(() => workoutRepo.list()).called(1);

    container.read(userId.notifier).state = 'user-b';
    await container.read(workoutsProvider.future);
    verify(() => workoutRepo.list()).called(1);
  });

  test('profileProvider refetches when the user changes', () async {
    await container.read(profileProvider.future);
    verify(() => profileRepo.getMe()).called(1);

    container.read(userId.notifier).state = 'user-b';
    await container.read(profileProvider.future);
    verify(() => profileRepo.getMe()).called(1);
  });

  test('same user does NOT refetch (cache preserved)', () async {
    await container.read(currentProgramProvider.future);
    await container.read(currentProgramProvider.future);
    verify(() => programService.getCurrent()).called(1);
  });
}
