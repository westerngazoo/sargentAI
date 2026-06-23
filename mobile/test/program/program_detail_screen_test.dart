// R-0014 Flutter widget tests — ProgramDetailScreen and home shortcut
// (SPEC-0014 §3.3 / §2.5.4).
//
// AC-coverage:
// - AC9  (detail screen: full program + diet displayed; reached from proposals
//         and from the named route /programs/current)
// - AC8  (home shortcut: CurrentProgramCard — active program → detail,
//         no program → "Get your program" CTA → match flow)
// - AC10 (Flutter widget tests all gates green)
//
// Targeted production symbols:
//   program/presentation/program_detail_screen.dart -> ProgramDetailScreen
//   program/application/program_providers.dart      -> currentProgramProvider
//   program/services/program_service.dart           -> ProgramService,
//                                                     programServiceProvider
//   shell/home_shell.dart (extended)                -> CurrentProgramCard
//   program/models/user_program.dart                -> UserProgram
//
// RED until step-5 creates the above. These tests must compile and fail at
// runtime because the production widgets do not exist.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/profile/application/profile_providers.dart';
import 'package:fitai/src/program/models/user_program.dart';
import 'package:fitai/src/program/presentation/program_detail_screen.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:fitai/src/shell/home_shell.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/profile_fakes.dart';
import '../support/program_fakes.dart';
import '../support/workout_fakes.dart';

void main() {
  setUpAll(registerFallbacks);
  setUpAll(registerProfileFallbacks);
  setUpAll(registerProgramFallbacks);

  late MockTokenStore tokenStore;
  late MockAuthRepository repo;
  late MockProgramService programService;
  late MockProfileRepository profileRepo;
  late MockWorkoutRepository workoutRepo;

  setUp(() {
    tokenStore = MockTokenStore();
    repo = MockAuthRepository();
    programService = MockProgramService();
    profileRepo = MockProfileRepository();
    workoutRepo = MockWorkoutRepository();
    when(() => tokenStore.read())
        .thenAnswer((_) async => sampleToken(userId: 'u-1'));
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
    when(() => workoutRepo.list()).thenAnswer((_) async => []);
  });

  // -------------------------------------------------------------------------
  // AC9 — ProgramDetailScreen renders program + diet
  // -------------------------------------------------------------------------

  /// Pumps [ProgramDetailScreen] backed by a fake [currentProgramProvider].
  Future<void> pumpDetail(WidgetTester tester) async {
    when(() => programService.getCurrent())
        .thenAnswer((_) async => sampleUserProgram());

    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
        programServiceProvider.overrideWithValue(programService),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: ProgramDetailScreen()),
      ),
    );
    await tester.pump();
    await tester.pumpAndSettle();
  }

  // AC9: program split label appears in the detail screen.
  testWidgets('AC9 program_detail_screen_renders_program_and_diet',
      (tester) async {
    await pumpDetail(tester);

    // Split from generatedProgramJson.
    expect(
      find.textContaining('4-day split'),
      findsAtLeastNWidgets(1),
    );
    // Macro values from generatedDietJson.
    expect(find.textContaining('176'), findsAtLeastNWidgets(1)); // protein_g
    expect(
        find.textContaining('3200'), findsAtLeastNWidgets(1)); // estimated_kcal
  });

  // AC9: detail screen shows days_per_week training overview.
  testWidgets('AC9 program_detail_screen_shows_days_per_week', (tester) async {
    await pumpDetail(tester);

    expect(find.textContaining('days/week'), findsAtLeastNWidgets(1));
  });

  // AC9: detail screen shows intensity guidance from the program template.
  testWidgets('AC9 program_detail_screen_shows_intensity_guidance',
      (tester) async {
    await pumpDetail(tester);

    expect(
      find.textContaining('1 all-out working set to failure'),
      findsAtLeastNWidgets(1),
    );
  });

  // AC9: detail screen shows rest guidance.
  testWidgets('AC9 program_detail_screen_shows_rest_guidance', (tester) async {
    await pumpDetail(tester);

    expect(
      find.textContaining('2-3 min between sets'),
      findsAtLeastNWidgets(1),
    );
  });

  // AC9: detail screen shows diet approach text.
  testWidgets('AC9 program_detail_screen_shows_diet_approach', (tester) async {
    await pumpDetail(tester);

    expect(
      find.textContaining('high-protein structured clean bulk'),
      findsAtLeastNWidgets(1),
    );
  });

  // AC9: detail screen shows archetype display name or id.
  testWidgets('AC9 program_detail_screen_shows_archetype_identifier',
      (tester) async {
    when(() => programService.getCurrent()).thenAnswer(
      (_) async => UserProgram.fromJson(userProgramJson(
        archetypeId: 'heavy-duty-mass',
      )),
    );
    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
        programServiceProvider.overrideWithValue(programService),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: ProgramDetailScreen()),
      ),
    );
    await tester.pumpAndSettle();

    // Either the archetype_id slug or the display_name must appear in the UI.
    final hasIdentifier = find.byWidgetPredicate(
      (w) =>
          w is Text &&
          ((w.data ?? '').contains('heavy-duty-mass') ||
              (w.data ?? '').contains('Low-Volume') ||
              (w.data ?? '').contains('Mass Builder')),
    );
    expect(hasIdentifier, findsAtLeastNWidgets(1));
  });

  // AC9: detail screen is also reachable via named route /programs/current.
  testWidgets('AC9 detail_screen_reachable_via_named_route', (tester) async {
    when(() => programService.getCurrent())
        .thenAnswer((_) async => sampleUserProgram());

    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
        programServiceProvider.overrideWithValue(programService),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          initialRoute: '/',
          routes: {
            '/': (_) => const Scaffold(body: Text('home')),
            '/programs/current': (_) => const ProgramDetailScreen(),
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Navigate to /programs/current.
    final context = tester.element(find.text('home'));
    Navigator.of(context).pushNamed('/programs/current');
    await tester.pumpAndSettle();

    expect(find.byType(ProgramDetailScreen), findsOneWidget);
  });

  // AC9: 404 (no current program) on the detail screen shows a CTA or message.
  testWidgets('AC9 detail_screen_no_program_shows_cta', (tester) async {
    when(() => programService.getCurrent())
        .thenThrow(const ApiException('not_found', statusCode: 404));

    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
        programServiceProvider.overrideWithValue(programService),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: ProgramDetailScreen()),
      ),
    );
    await tester.pumpAndSettle();

    // Some form of "get your program" or "no program" message must appear.
    final noProgram = find.byWidgetPredicate(
      (w) =>
          w is Text &&
          ((w.data ?? '').toLowerCase().contains('program') ||
              (w.data ?? '').toLowerCase().contains('match') ||
              (w.data ?? '').toLowerCase().contains('get')),
    );
    expect(noProgram, findsAtLeastNWidgets(1));
  });

  // -------------------------------------------------------------------------
  // AC8 — HomeShell home shortcut: CurrentProgramCard
  // -------------------------------------------------------------------------

  /// Pumps [HomeShell] with the program + profile + workout overrides.
  Future<void> pumpHome(WidgetTester tester) async {
    when(() => profileRepo.getMe())
        .thenAnswer((_) async => sampleProfile(userId: 'u-1'));
    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
        profileRepositoryProvider.overrideWithValue(profileRepo),
        workoutRepositoryProvider.overrideWithValue(workoutRepo),
        programServiceProvider.overrideWithValue(programService),
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
    await tester.pump();
    await tester.pump();
  }

  // AC8: when a current program exists the home screen shows the
  // CurrentProgramCard with the program split summary and kcal.
  testWidgets('AC8 home_current_program_card_shows_split_and_kcal',
      (tester) async {
    when(() => programService.getCurrent())
        .thenAnswer((_) async => sampleUserProgram());

    await pumpHome(tester);
    await tester.pumpAndSettle();

    // Split or kcal text from the active program appears on the home screen.
    final splitOrKcal = find.byWidgetPredicate(
      (w) =>
          w is Text &&
          ((w.data ?? '').contains('4-day') ||
              (w.data ?? '').contains('3200') ||
              (w.data ?? '').contains('split')),
    );
    expect(splitOrKcal, findsAtLeastNWidgets(1));
  });

  // AC8: tapping the CurrentProgramCard navigates to ProgramDetailScreen.
  testWidgets('AC8 home_current_program_card_navigates_to_detail',
      (tester) async {
    when(() => programService.getCurrent())
        .thenAnswer((_) async => sampleUserProgram());

    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
        profileRepositoryProvider.overrideWithValue(profileRepo),
        workoutRepositoryProvider.overrideWithValue(workoutRepo),
        programServiceProvider.overrideWithValue(programService),
      ],
    );
    addTearDown(container.dispose);
    container.read(authControllerProvider);

    // GoRouter is required because CurrentProgramCard uses context.go().
    final router = GoRouter(
      initialLocation: '/home',
      routes: [
        GoRoute(path: '/home', builder: (_, __) => const HomeShell()),
        GoRoute(
          path: '/programs/current',
          builder: (_, __) => const ProgramDetailScreen(),
        ),
      ],
    );
    addTearDown(router.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp.router(routerConfig: router),
      ),
    );
    await tester.pump();
    await tester.pump();
    await tester.pumpAndSettle();

    // Tap the CurrentProgramCard / program shortcut widget.
    await tester.tap(
      find
          .byWidgetPredicate(
            (w) =>
                w is Text &&
                ((w.data ?? '').contains('4-day') ||
                    (w.data ?? '').contains('split')),
          )
          .first,
    );
    await tester.pumpAndSettle();

    expect(find.byType(ProgramDetailScreen), findsOneWidget);
  });

  // AC8: when no program exists the home screen shows a "Get your program" CTA.
  testWidgets('AC8 home_no_program_cta_shown_when_no_current_program',
      (tester) async {
    when(() => programService.getCurrent())
        .thenThrow(const ApiException('not_found', statusCode: 404));

    await pumpHome(tester);
    await tester.pumpAndSettle();

    // The CTA text must appear — "Get your program" or equivalent.
    final ctaFinder = find.byWidgetPredicate(
      (w) =>
          w is Text &&
          ((w.data ?? '').toLowerCase().contains('get your program') ||
              (w.data ?? '').toLowerCase().contains('get a program')),
    );
    expect(ctaFinder, findsAtLeastNWidgets(1));
  });

  // AC8: tapping the "Get your program" CTA navigates toward the match flow.
  testWidgets('AC8 home_no_program_cta_navigates_to_match_flow',
      (tester) async {
    when(() => programService.getCurrent())
        .thenThrow(const ApiException('not_found', statusCode: 404));

    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
        profileRepositoryProvider.overrideWithValue(profileRepo),
        workoutRepositoryProvider.overrideWithValue(workoutRepo),
        programServiceProvider.overrideWithValue(programService),
      ],
    );
    addTearDown(container.dispose);
    container.read(authControllerProvider);

    // GoRouter is required because CurrentProgramCard uses context.go().
    // /programs/get is the CTA destination (R-0030).
    final router = GoRouter(
      initialLocation: '/home',
      routes: [
        GoRoute(path: '/home', builder: (_, __) => const HomeShell()),
        GoRoute(
          path: '/programs/get',
          builder: (_, __) => const Scaffold(body: Text('onboarding-sentinel')),
        ),
      ],
    );
    addTearDown(router.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp.router(routerConfig: router),
      ),
    );
    await tester.pump();
    await tester.pump();
    await tester.pumpAndSettle();

    // Tap the CTA.
    final ctaFinder = find.byWidgetPredicate(
      (w) =>
          w is Text &&
          ((w.data ?? '').toLowerCase().contains('get your program') ||
              (w.data ?? '').toLowerCase().contains('get a program')),
    );
    expect(ctaFinder, findsAtLeastNWidgets(1));
    await tester.tap(ctaFinder.first);
    await tester.pumpAndSettle();

    // Navigation toward the match/onboarding flow must have occurred.
    expect(find.text('onboarding-sentinel'), findsOneWidget,
        reason: '"Get your program" CTA must navigate toward the match flow');
  });

  // -------------------------------------------------------------------------
  // AC9 — detail screen loaded standalone shows current program
  // -------------------------------------------------------------------------

  testWidgets('AC9 detail_screen_loaded_standalone_shows_current_program',
      (tester) async {
    when(() => programService.getCurrent()).thenAnswer(
      (_) async => sampleUserProgram(archetypeId: 'classic-aesthetic-taper'),
    );

    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
        programServiceProvider.overrideWithValue(programService),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: ProgramDetailScreen()),
      ),
    );
    await tester.pumpAndSettle();

    // The program must have been fetched exactly once (the provider called
    // getCurrent).
    verify(() => programService.getCurrent()).called(1);
    // The screen is present (not errored).
    expect(find.byType(ProgramDetailScreen), findsOneWidget);
  });
}
