// SAC1 -> AC1: HomeShell observes profile existence via profileProvider
//   (GET /profile/me). When it resolves to null (no profile / 404) the
//   ProfilePrompt is shown; when it resolves to a Profile (200) no prompt is
//   shown.
// SAC2 -> AC2: the prompt is dismissible — dismissing hides it for the session
//   (onboardingDismissedProvider true) and leaves the shell usable; nothing is
//   saved (no PUT).
// SAC11 -> AC11: the shell renders no feature logger UI; the AppBar is titled
//   'fitAI' and offers Logout.
//
// The refactored shell (SPEC-0008 §2.2/§3.5) drives the prompt through
// `AsyncValue.when` over profileProvider and drops the old GET /auth/me read —
// profileProvider's GET /profile/me is the new cold-start liveness probe
// (a 401 there 401-sinks via the shared AuthInterceptor; preserved in
// app_router_test). The user identity, if shown, reads the in-memory session.
//
// RED until package:fitai/src/shell/home_shell.dart is refactored to a
// ConsumerWidget over profileProvider and package:fitai/src/profile/** exists.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/profile/application/profile_providers.dart';
import 'package:fitai/src/profile/presentation/profile_prompt.dart';
import 'package:fitai/src/shell/home_shell.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/profile_fakes.dart';
import '../support/workout_fakes.dart';

void main() {
  setUpAll(registerFallbacks);
  setUpAll(registerProfileFallbacks);

  late MockTokenStore tokenStore;
  late MockAuthRepository repo;
  late MockProfileRepository profileRepo;
  late MockWorkoutRepository workoutRepo;

  setUp(() {
    tokenStore = MockTokenStore();
    repo = MockAuthRepository();
    profileRepo = MockProfileRepository();
    workoutRepo = MockWorkoutRepository();
    when(() => tokenStore.read())
        .thenAnswer((_) async => sampleToken(userId: 'u-1'));
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
    when(() => workoutRepo.list()).thenAnswer((_) async => []);
  });

  Future<ProviderContainer> pumpShell(WidgetTester tester) async {
    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
        profileRepositoryProvider.overrideWithValue(profileRepo),
        workoutRepositoryProvider.overrideWithValue(workoutRepo),
      ],
    );
    addTearDown(container.dispose);
    container.read(authControllerProvider);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: HomeShell()),
      ),
    );
    // Bounded pumps flush the restore + the profileProvider future without
    // pumpAndSettle (the loading state shows a perpetual spinner that would
    // never settle on the fake clock).
    await tester.pump();
    await tester.pump();
    return container;
  }

  testWidgets(
      'SAC1 a null profile (404) shows the complete-your-profile prompt',
      (tester) async {
    when(() => profileRepo.getMe()).thenAnswer((_) async => null);
    await pumpShell(tester);

    expect(find.byType(ProfilePrompt), findsOneWidget);
  });

  testWidgets('SAC1 an existing profile (200) shows NO prompt', (tester) async {
    when(() => profileRepo.getMe())
        .thenAnswer((_) async => sampleProfile(userId: 'u-1'));
    await pumpShell(tester);

    expect(find.byType(ProfilePrompt), findsNothing);
  });

  testWidgets('SAC2 dismissing the prompt hides it for the session (no save)',
      (tester) async {
    when(() => profileRepo.getMe()).thenAnswer((_) async => null);
    final container = await pumpShell(tester);
    expect(find.byType(ProfilePrompt), findsOneWidget);

    // Tap the dismiss affordance; the prompt exposes a close/dismiss control.
    await tester.tap(find.byTooltip('Dismiss'));
    await tester.pump();

    expect(find.byType(ProfilePrompt), findsNothing);
    expect(container.read(onboardingDismissedProvider), isTrue);
    // Nothing was saved.
    verifyNever(() => profileRepo.putMe(any()));
  });

  testWidgets('SAC11 AppBar titled fitAI with a Logout action, no logger UI',
      (tester) async {
    when(() => profileRepo.getMe())
        .thenAnswer((_) async => sampleProfile(userId: 'u-1'));
    await pumpShell(tester);

    expect(find.widgetWithText(AppBar, 'fitAI'), findsOneWidget);
    expect(find.textContaining('Logout'), findsOneWidget);
    for (final feature in const ['Workout', 'Nutrition', 'Photo']) {
      expect(find.textContaining(feature), findsNothing);
    }
  });
}
