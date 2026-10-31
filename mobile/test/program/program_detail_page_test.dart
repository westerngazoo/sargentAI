// R-0014 Flutter widget tests — ProgramDetailPage (SPEC-0014 §3.3 / §2.5.4),
// the first page of the Programa panel's flow since R-0061 slice 3a
// (SPEC-0061 §2.6).
//
// AC-coverage:
// - AC9  (detail page: full program + diet displayed)
// - AC8  (no program → "Find your program" opens the match flow; with one,
//         "Change program" does, R-0061 AC8)
// - AC10 (Flutter widget tests all gates green)

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/program/models/user_program.dart';
import 'package:fitai/src/program/presentation/body_type_picker_page.dart';
import 'package:fitai/src/program/presentation/program_detail_page.dart';
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
    // The weekly card's target now reads the schedule (SPEC-0061 N6).
    whenGetSchedule(programService).thenAnswer((_) async => sampleSchedule());
    when(() => tokenStore.read())
        .thenAnswer((_) async => sampleToken(userId: 'u-1'));
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
  });

  // -------------------------------------------------------------------------
  // AC9 — ProgramDetailPage renders program + diet
  // -------------------------------------------------------------------------

  /// Pumps [ProgramDetailPage] backed by a fake [currentProgramProvider].
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
        child: localizedApp(home: const Scaffold(body: ProgramDetailPage())),
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
        child: localizedApp(home: const Scaffold(body: ProgramDetailPage())),
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
        child: localizedApp(home: const Scaffold(body: ProgramDetailPage())),
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
        child: localizedApp(home: const Scaffold(body: ProgramDetailPage())),
      ),
    );
    await tester.pumpAndSettle();

    // The program must have been fetched exactly once (the provider called
    // getCurrent).
    verify(() => programService.getCurrent()).called(1);
    // The screen is present (not errored).
    expect(find.byType(ProgramDetailPage), findsOneWidget);
  });

  // R-0061 AC8: the panel lets the user choose a different program.
  testWidgets('with a program, Cambiar programa opens the body-type picker',
      (tester) async {
    await pumpDetail(tester);

    await tester.tap(find.byKey(const Key('change-program')));
    await tester.pumpAndSettle();

    expect(find.byType(BodyTypePickerPage), findsOneWidget);
    expect(find.text(l10n.chooseYourBodyType), findsOneWidget);
  });

  // R-0061 AC8: with no program the empty state opens the same flow.
  testWidgets('with no program, Encuentra tu programa opens the picker',
      (tester) async {
    when(() => programService.getCurrent()).thenAnswer((_) async => null);
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
        child: localizedApp(home: const Scaffold(body: ProgramDetailPage())),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text(l10n.noProgramYet), findsOneWidget);
    expect(find.byKey(const Key('change-program')), findsNothing);

    await tester.tap(find.byKey(const Key('get-program')));
    await tester.pumpAndSettle();
    expect(find.byType(BodyTypePickerPage), findsOneWidget);
  });
}
