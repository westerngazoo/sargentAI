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
//   program/models/user_program.dart                -> UserProgram
//
// RED until step-5 creates the above. These tests must compile and fail at
// runtime because the production widgets do not exist.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/program/models/user_program.dart';
import 'package:fitai/src/program/presentation/program_detail_screen.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/l10n.dart';
import '../support/profile_fakes.dart';
import '../support/program_fakes.dart';

void main() {
  setUpAll(registerFallbacks);
  setUpAll(registerProfileFallbacks);
  setUpAll(registerProgramFallbacks);

  late MockTokenStore tokenStore;
  late MockAuthRepository repo;
  late MockProgramService programService;

  setUp(() {
    tokenStore = MockTokenStore();
    repo = MockAuthRepository();
    programService = MockProgramService();
    when(() => tokenStore.read())
        .thenAnswer((_) async => sampleToken(userId: 'u-1'));
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
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
        child: localizedApp(home: const ProgramDetailScreen()),
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
    // Macro values from generatedDietJson — in the nutrition card below the
    // fold, so scroll it into view first.
    await tester.scrollUntilVisible(
      find.textContaining('176'),
      300,
      scrollable: find.byType(Scrollable).first,
    );
    expect(find.textContaining('176'), findsAtLeastNWidgets(1)); // protein_g
    expect(
        find.textContaining('3200'), findsAtLeastNWidgets(1)); // estimated_kcal
  });

  // AC9: detail screen shows days_per_week training overview.
  testWidgets('AC9 program_detail_screen_shows_days_per_week', (tester) async {
    await pumpDetail(tester);

    expect(find.text(l10n.daysPerWeek(4)), findsOneWidget);
    expect(find.text(l10n.minutesShort(45)), findsOneWidget);
  });

  // R-0061 AC11: the section headers and row labels speak the catalogue.
  testWidgets('program_detail_screen_speaks_the_catalogue', (tester) async {
    await pumpDetail(tester);

    expect(find.text(l10n.yourProgram), findsOneWidget);
    // The section header and the days-per-week row label share the key.
    expect(find.text(l10n.training), findsNWidgets(2));
    expect(find.text(l10n.split), findsOneWidget);
    expect(find.text(l10n.targetPhysiqueCaps), findsOneWidget);

    await tester.scrollUntilVisible(
      find.text(l10n.kcalPerDay),
      300,
      scrollable: find.byType(Scrollable).first,
    );
    expect(find.text(l10n.nutrition), findsOneWidget);
    expect(find.text(l10n.protein), findsOneWidget);
    expect(find.text(l10n.grams(176)), findsOneWidget);
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

    // Nutrition card is below the fold; scroll it into view.
    await tester.scrollUntilVisible(
      find.textContaining('high-protein structured clean bulk'),
      300,
      scrollable: find.byType(Scrollable).first,
    );
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
        child: localizedApp(home: const ProgramDetailScreen()),
      ),
    );
    await tester.pumpAndSettle();

    // Either the archetype id (slug or display-cased) or the display_name
    // must appear in the UI.
    final hasIdentifier = find.byWidgetPredicate(
      (w) =>
          w is Text &&
          ((w.data ?? '').contains('heavy-duty-mass') ||
              (w.data ?? '').contains('Heavy Duty Mass') ||
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
        child: localizedApp(
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
        .thenThrow(const ApiException(ApiErrorKind.notFound, statusCode: 404));

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
        child: localizedApp(home: const ProgramDetailScreen()),
      ),
    );
    await tester.pumpAndSettle();

    // The provider surfaces the thrown 404 as an error, so the screen shows
    // the could-not-load message with its retry action.
    expect(find.text(l10n.couldNotLoadYourProgram), findsOneWidget);
    expect(find.text(l10n.retry), findsOneWidget);
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
    // Resolve the cold-start restore before the widget reads the provider —
    // mirrors production, where the auth-gate makes this screen reachable
    // only once AuthAuthenticated (so userId is set on the first fetch).
    container.read(authControllerProvider);
    await tester.pump();
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: localizedApp(home: const ProgramDetailScreen()),
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
