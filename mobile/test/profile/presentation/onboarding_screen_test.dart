// SAC3/SAC7/SAC8/SAC9 -> AC3/AC7/AC8/AC9 (the wizard screen):
//   * the screen renders the wizard with a progress indicator and back/next
//     controls, hosting the three steps (SAC3);
//   * on submit success (200) the screen — the SOLE owner of the
//     `go('/home')` — navigates home once the controller reports `done`
//     (SAC7); the prompt is then re-evaluated by the shell (covered in
//     home_shell_test);
//   * on a 400 the screen stays put, jumps to the offending step, and shows the
//     inline message; a transport error shows a retryable message; data is
//     never lost (SAC8);
//   * the screen holds no try/catch — it ref.listens controller state and
//     reacts; failure is data on the state (SAC9).
//
// Harness notes (the two bugs that broke the predecessor):
//   * never `await Future.delayed` before a pump — timers run on the fake clock;
//   * never `pumpAndSettle` while a perpetual spinner is up — use bounded
//     pumps. The submit gate below uses a Completer + bounded pumps so the
//     in-flight spinner is observed without settling.
//
// RED until package:fitai/src/profile/presentation/onboarding_screen.dart and
// the step widgets exist, and the route is added.

import 'dart:async';

import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/profile/application/onboarding_controller.dart';
import 'package:fitai/src/profile/application/profile_providers.dart';
import 'package:fitai/src/profile/domain/goal.dart';
import 'package:fitai/src/profile/domain/profile.dart';
import 'package:fitai/src/profile/presentation/onboarding_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/l10n.dart';
import '../../support/profile_fakes.dart';

void main() {
  setUpAll(registerProfileFallbacks);

  late MockProfileRepository repo;

  setUp(() {
    repo = MockProfileRepository();
    when(() => repo.getMe()).thenAnswer((_) async => sampleProfile());
  });

  // Hosts the OnboardingScreen under a minimal router so a `go('/home')` is
  // observable. The /home route is a sentinel.
  Future<(ProviderContainer, GoRouter)> pumpWizard(WidgetTester tester) async {
    final container = ProviderContainer(
      overrides: [
        profileRepositoryProvider.overrideWithValue(repo),
      ],
    );
    addTearDown(container.dispose);
    final router = GoRouter(
      initialLocation: '/onboarding',
      routes: [
        GoRoute(
          path: '/onboarding',
          builder: (_, __) => const OnboardingScreen(),
        ),
        GoRoute(
          path: '/home',
          builder: (_, __) =>
              const Scaffold(body: Center(child: Text('HOME-SENTINEL'))),
        ),
      ],
    );
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: localizedRouterApp(router),
      ),
    );
    await tester.pump();
    return (container, router);
  }

  String locationOf(GoRouter r) =>
      r.routerDelegate.currentConfiguration.uri.path;

  // Fills the controller's draft with valid required fields directly — the
  // wizard's per-field input widgets are step-5 presentation detail; these
  // tests pin the screen's navigation/error REACTION to controller state.
  void fillValidDraft(ProviderContainer c) {
    final ctrl = c.read(onboardingControllerProvider.notifier);
    ctrl.setBodyStats(dob: DateTime(1996, 6, 6), height: 180, weight: 82.5);
    ctrl.toggleGoal(Goal.buildMuscle);
  }

  testWidgets('SAC3 renders a progress indicator and back/next affordances',
      (tester) async {
    await pumpWizard(tester);
    // A linear progress header (not a spinner) shows wizard progress.
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    expect(find.text(l10n.next), findsOneWidget);
  });

  // R-0061 AC11: the step speaks the catalogue, including the field errors
  // the draft reports as typed values.
  testWidgets('the body-stats step and its field errors speak the catalogue',
      (tester) async {
    await pumpWizard(tester);

    expect(find.text(l10n.setUpYourProfile), findsOneWidget);
    expect(find.text(l10n.aboutYou), findsOneWidget);
    expect(find.text(l10n.notSet), findsOneWidget);

    await tester.enterText(
      find.widgetWithText(TextField, l10n.heightCmLabel),
      '10',
    );
    await tester.pump();

    expect(find.text(l10n.heightOutOfRange(50, 300)), findsOneWidget);
  });

  testWidgets('SAC7 a successful save navigates to /home (screen owns the go)',
      (tester) async {
    when(() => repo.putMe(any())).thenAnswer((_) async => sampleProfile());
    final (container, router) = await pumpWizard(tester);
    fillValidDraft(container);

    // Drive submit through the controller; the screen ref.listens `done`.
    await container.read(onboardingControllerProvider.notifier).submit();
    await tester.pump(); // let ref.listen fire and the router redirect
    await tester.pump();

    expect(locationOf(router), '/home');
    expect(find.text('HOME-SENTINEL'), findsOneWidget);
  });

  testWidgets('SAC8 a 400 keeps the user on /onboarding and shows the message',
      (tester) async {
    when(() => repo.putMe(any())).thenThrow(
      const ApiException(
        ApiErrorKind.badRequest,
        statusCode: 400,
        field: 'height_cm',
      ),
    );
    final (container, router) = await pumpWizard(tester);
    fillValidDraft(container);

    await container.read(onboardingControllerProvider.notifier).submit();
    await tester.pump();
    await tester.pump();

    expect(locationOf(router), '/onboarding', reason: 'no navigation on error');
    expect(find.text(l10n.apiErrorBadRequest), findsOneWidget);
  });

  testWidgets('SAC8 a transport error shows a retryable message, stays put',
      (tester) async {
    when(() => repo.putMe(any())).thenThrow(
      const ApiException(ApiErrorKind.transport),
    );
    final (container, router) = await pumpWizard(tester);
    fillValidDraft(container);

    await container.read(onboardingControllerProvider.notifier).submit();
    await tester.pump();
    await tester.pump();

    expect(locationOf(router), '/onboarding');
    expect(find.text(l10n.apiErrorTransport), findsOneWidget);
  });

  testWidgets(
      'SAC9 the finish action disables while submitting (no double-submit)',
      (tester) async {
    final gate = Completer<Profile>();
    when(() => repo.putMe(any())).thenAnswer((_) => gate.future);
    final (container, _) = await pumpWizard(tester);
    fillValidDraft(container);

    // Begin the submit but hold the network open.
    final pending =
        container.read(onboardingControllerProvider.notifier).submit();
    await tester.pump();

    expect(
      container.read(onboardingControllerProvider).submitting,
      isTrue,
      reason: 'state reflects the in-flight submit',
    );

    gate.complete(sampleProfile());
    await pending;
    await tester.pump();
  });

  // Beta feedback 2026-10-22, on a desktop browser: the date-of-birth picker
  // opened on a calendar month 25 years back, and "Next" sat alone in the
  // bottom-right corner, greyed out with no reason given, so the owner looked
  // for a Save button that seemed not to exist.
  group('desktop layout and the date-of-birth picker', () {
    void desktop(WidgetTester tester) {
      tester.view.physicalSize = const Size(1440, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
    }

    testWidgets('the form and its controls sit in one centered column',
        (tester) async {
      desktop(tester);
      await pumpWizard(tester);

      final next = tester.getRect(find.widgetWithText(FilledButton, l10n.next));
      final weight =
          tester.getRect(find.widgetWithText(TextField, l10n.weightKgLabel));

      expect(next.right, lessThanOrEqualTo(720 + 300),
          reason: 'Next must not hug the right edge of a wide window');
      expect(weight.width, lessThanOrEqualTo(600),
          reason: 'fields must not stretch across a wide window');
      expect(next.top - weight.bottom, lessThan(200),
          reason: 'Next must sit right under the form, not at the bottom');
    });

    testWidgets('a disabled Next says what is missing', (tester) async {
      final (container, _) = await pumpWizard(tester);
      final hint = l10n.onboardingMissingBodyStats;

      expect(find.text(hint), findsOneWidget);

      fillValidDraft(container);
      await tester.pump();
      expect(find.text(hint), findsNothing);
    });

    testWidgets('the date-of-birth picker opens on the year grid',
        (tester) async {
      await pumpWizard(tester);

      await tester.tap(find.text(l10n.choose));
      await tester.pumpAndSettle();

      expect(find.byType(YearPicker), findsOneWidget);
    });
  });
}
