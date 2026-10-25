// R-0014 Flutter widget tests — ProgramProposalsScreen (SPEC-0014 §3.3).
//
// AC-coverage:
// - AC8  (proposals screen: three cards, score label, program + diet summary,
//         tap-to-expand, exclusive expansion, "Choose this program" button)
// - AC9  (choose button navigates to ProgramDetailScreen)
// - AC10 (Flutter widget tests)
//
// Targeted production symbols (all under package:fitai/src/program/...):
//   presentation/program_proposals_screen.dart
//     -> ProgramProposalsScreen(sessionId: String)
//   presentation/program_detail_screen.dart
//     -> ProgramDetailScreen
//   services/program_service.dart
//     -> ProgramService, programServiceProvider
//   models/program_proposal.dart
//     -> ProposalsResponse
//   models/user_program.dart
//     -> UserProgram
//   application/program_providers.dart
//     -> currentProgramProvider
//
// RED until step-5 implementation creates all of the above. No edits to this
// file should be needed to make it GREEN.

import 'dart:async';

import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/program/models/user_program.dart';
import 'package:fitai/src/program/presentation/program_detail_screen.dart';
import 'package:fitai/src/program/presentation/program_proposals_screen.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/l10n.dart';
import '../support/program_fakes.dart';

void main() {
  setUpAll(registerFallbacks);
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

  /// Pumps [ProgramProposalsScreen] with the test overrides.
  ///
  /// Uses a simple [MaterialApp] + [Navigator] wrapping instead of the full
  /// GoRouter so navigation assertions are predictable and router-agnostic.
  Future<void> pumpProposals(
    WidgetTester tester, {
    String sessionId = 'sess-001',
  }) async {
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
          home: ProgramProposalsScreen(sessionId: sessionId),
        ),
      ),
    );
    // Flush the loading state.
    await tester.pump();
  }

  // -------------------------------------------------------------------------
  // AC8: three proposal cards rendered
  // -------------------------------------------------------------------------

  // AC8: three cards are rendered, each showing the archetype display name.
  testWidgets('AC8 proposals_screen_renders_three_cards', (tester) async {
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    // Three distinct archetype names must appear.
    expect(find.text('Low-Volume Mass Builder'), findsAtLeastNWidgets(1));
    expect(find.text('Minimalist High-Intensity'), findsAtLeastNWidgets(1));
    expect(find.text('Compact Powerbuilder'), findsAtLeastNWidgets(1));
  });

  // AC8: each card shows a human-readable score label
  // (rank 1 → "Best match", rank 2 → "Close match", rank 3 → "Good option").
  testWidgets('AC8 proposals_screen_shows_score_labels_for_all_three_ranks',
      (tester) async {
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    expect(find.text(l10n.rankBestMatch), findsOneWidget);
    expect(find.text(l10n.rankCloseMatch), findsOneWidget);
    expect(find.text(l10n.rankGoodOption), findsOneWidget);
  });

  // AC8: each card (collapsed) shows days_per_week summary.
  testWidgets('AC8 proposals_screen_shows_days_per_week_in_collapsed_card',
      (tester) async {
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    // The 4 days/week · 3200 kcal/day summary line (generatedProgramJson,
    // generatedDietJson) appears on every card.
    expect(
      find.text(l10n.proposalSummaryLine(4, 3200)),
      findsAtLeastNWidgets(1),
    );
  });

  // AC8: each card (collapsed) shows estimated kcal.
  testWidgets('AC8 proposals_screen_shows_kcal_in_collapsed_card',
      (tester) async {
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    // The kcal value from generatedDietJson (3200) must appear on some card.
    expect(find.textContaining('3200'), findsAtLeastNWidgets(1));
  });

  // -------------------------------------------------------------------------
  // AC8: tap-to-expand, exclusive expansion
  // -------------------------------------------------------------------------

  // AC8: tapping card 1 expands it (shows "Choose this program" button).
  testWidgets('AC8 proposals_screen_card_expand_shows_choose_button',
      (tester) async {
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    // "Choose this program" button must NOT be visible before any tap.
    expect(find.text(l10n.chooseThisProgram), findsNothing);

    // Tap card 1 (first card by display name).
    await tester.tap(find.text('Low-Volume Mass Builder'));
    await tester.pumpAndSettle();

    // The button appears in the expanded card.
    expect(find.text(l10n.chooseThisProgram), findsOneWidget);
  });

  // AC8: tapping card 2 collapses card 1 and expands card 2 — exclusive.
  testWidgets('AC8 proposals_screen_card_expand_collapse', (tester) async {
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    // Expand card 1.
    await tester.tap(find.text('Low-Volume Mass Builder'));
    await tester.pumpAndSettle();
    expect(find.text(l10n.chooseThisProgram), findsOneWidget);

    // Tap card 2.
    await tester.tap(find.text('Minimalist High-Intensity'));
    await tester.pumpAndSettle();

    // Still exactly one "Choose this program" button — card 2 is now expanded,
    // card 1 is collapsed.
    expect(find.text(l10n.chooseThisProgram), findsOneWidget);
  });

  // AC8: expanded card shows full GeneratedProgram fields (intensity guidance).
  testWidgets('AC8 expanded_card_shows_full_program_details', (tester) async {
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    await tester.tap(find.text('Low-Volume Mass Builder'));
    await tester.pumpAndSettle();

    // Intensity guidance text from generatedProgramJson, in its labelled line.
    expect(
      find.text(l10n.intensityLine('1 all-out working set to failure')),
      findsOneWidget,
    );
  });

  // AC8: expanded card shows macro table (protein / carbs / fat).
  testWidgets('AC8 expanded_card_shows_diet_macro_table', (tester) async {
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    await tester.tap(find.text('Low-Volume Mass Builder'));
    await tester.pumpAndSettle();

    // Protein / carbs / fat from generatedDietJson.
    expect(find.text(l10n.grams(176)), findsOneWidget); // protein_g
    expect(find.text(l10n.grams(360)), findsOneWidget); // carbs_g
    expect(find.text(l10n.grams(89)), findsOneWidget); // fat_g
  });

  // -------------------------------------------------------------------------
  // AC9: "Choose this program" navigates to ProgramDetailScreen on success
  // -------------------------------------------------------------------------

  // AC9: tapping "Choose this program" calls ProgramService.chooseProgram and
  // navigates to ProgramDetailScreen on success.
  testWidgets('AC9 proposals_screen_choose_button_navigates_to_detail',
      (tester) async {
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());
    when(() => programService.chooseProgram(any(), any()))
        .thenAnswer((_) async => sampleUserProgram());
    // getCurrent is needed by ProgramDetailScreen when it loads.
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

    // Use GoRouter so context.go('/programs/current') resolves correctly.
    final router = GoRouter(
      initialLocation: '/proposals',
      routes: [
        GoRoute(
          path: '/proposals',
          builder: (_, __) =>
              const ProgramProposalsScreen(sessionId: 'sess-001'),
        ),
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
        child: localizedRouterApp(router),
      ),
    );
    await tester.pump();
    await tester.pumpAndSettle();

    // Expand card 1.
    await tester.tap(find.text('Low-Volume Mass Builder'));
    await tester.pumpAndSettle();

    // Tap "Choose this program".
    await tester.tap(find.text(l10n.chooseThisProgram));
    await tester.pumpAndSettle();

    // ProgramDetailScreen must now be on screen.
    expect(find.byType(ProgramDetailScreen), findsOneWidget);

    verify(() => programService.chooseProgram('sess-001', 'heavy-duty-mass'))
        .called(1);
  });

  // AC9: while the choose call is in-flight the button shows a loading
  // indicator and is disabled (no double-submit).
  testWidgets('AC9 choose_button_disabled_while_in_flight', (tester) async {
    final gate = Completer<UserProgram>();
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());
    when(() => programService.chooseProgram(any(), any()))
        .thenAnswer((_) => gate.future);

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    await tester.tap(find.text('Low-Volume Mass Builder'));
    await tester.pumpAndSettle();

    await tester.tap(find.text(l10n.chooseThisProgram));
    await tester.pump(); // start the async call, do NOT settle

    // A loading indicator must be visible.
    expect(find.byType(CircularProgressIndicator), findsAtLeastNWidgets(1));

    gate.completeError(
        const ApiException(ApiErrorKind.conflict, statusCode: 409));
    await tester.pumpAndSettle();
  });

  // AC8: a 409 from chooseProgram surfaces a snackbar with the expected message.
  testWidgets('AC8 choose_409_shows_snackbar_refresh_message', (tester) async {
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());
    when(() => programService.chooseProgram(any(), any())).thenThrow(
      const ApiException(ApiErrorKind.conflict, statusCode: 409),
    );

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    await tester.tap(find.text('Low-Volume Mass Builder'));
    await tester.pumpAndSettle();

    await tester.tap(find.text(l10n.chooseThisProgram));
    await tester.pumpAndSettle();

    // The spec §2.5.3 snackbar message for 409.
    expect(find.text(l10n.selectionNoLongerAvailable), findsOneWidget);
  });

  // AC8: the first card shows the first three highlight exercises as chips.
  testWidgets('AC8 proposals_screen_shows_highlight_exercise_chips',
      (tester) async {
    when(() => programService.getProposals(any()))
        .thenAnswer((_) async => sampleProposals());

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    // The first three highlight exercises from generatedProgramJson.
    expect(find.text('Bench Press'), findsAtLeastNWidgets(1));
    expect(find.text('Overhead Press'), findsAtLeastNWidgets(1));
    expect(find.text('Squat'), findsAtLeastNWidgets(1));
  });

  // AC6 / AC10: a transport/network error surfaces a human-readable error
  // rather than a raw exception or an infinite spinner.
  testWidgets('AC10 proposals_screen_error_shows_friendly_message',
      (tester) async {
    when(() => programService.getProposals(any())).thenThrow(
      const ApiException(ApiErrorKind.transport),
    );

    await pumpProposals(tester);
    await tester.pumpAndSettle();

    // Some error text must appear; no raw 'Exception' string.
    expect(find.textContaining('Exception'), findsNothing);
    expect(find.textContaining('DioException'), findsNothing);
    // The catalogue's message and its retry action appear.
    expect(find.text(l10n.couldNotLoadProposals), findsOneWidget);
    expect(find.text(l10n.retry), findsOneWidget);
  });
}
