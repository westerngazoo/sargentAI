// R-0061 AC1, AC4, AC5, AC7, AC13 -> SPEC-0061 §2.5 (slice 1f): the one
// screen, its calendar from the logs, the selected day's one action, the
// few controls, the liveness probe and the onboarding prompt that moved here
// from HomeShell, and the two sizes.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/home/calendar.dart';
import 'package:fitai/src/home/main_screen.dart';
import 'package:fitai/src/home/selected_day.dart';
import 'package:fitai/src/profile/application/profile_providers.dart';
import 'package:fitai/src/profile/presentation/profile_prompt.dart';
import 'package:fitai/src/workout/application/session_driver.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:fitai/src/workout/domain/workout_session.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/l10n.dart';
import '../support/profile_fakes.dart';
import '../support/workout_fakes.dart';

const phone = Size(390, 844);
const desktop = Size(1440, 900);

void main() {
  setUpAll(() {
    registerFallbacks();
    registerProfileFallbacks();
    registerWorkoutFallbacks();
  });

  late MockTokenStore tokenStore;
  late MockAuthRepository repo;
  late MockProfileRepository profileRepo;
  late MockWorkoutRepository workoutRepo;

  final today =
      DateTime(DateTime.now().year, DateTime.now().month, DateTime.now().day);
  final yesterday = today.subtract(const Duration(days: 1));

  WorkoutSession session(DateTime on) => WorkoutSession(
        id: 's-${on.day}',
        userId: 'u-1',
        performedOn: on,
        exercises: const [],
        createdAt: on,
        updatedAt: on,
      );

  setUp(() {
    tokenStore = MockTokenStore();
    repo = MockAuthRepository();
    profileRepo = MockProfileRepository();
    workoutRepo = MockWorkoutRepository();
    when(() => tokenStore.read()).thenAnswer((_) async => sampleToken());
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
    when(() => repo.me()).thenAnswer((_) async => 'u-1');
    when(() => profileRepo.getMe()).thenAnswer((_) async => sampleProfile());
    when(() => workoutRepo.list())
        .thenAnswer((_) async => [session(yesterday)]);
  });

  Future<(ProviderContainer, GoRouter)> pumpMain(WidgetTester tester,
      {Size size = phone}) async {
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final container = ProviderContainer(overrides: [
      tokenStoreProvider.overrideWithValue(tokenStore),
      authRepositoryProvider.overrideWithValue(repo),
      profileRepositoryProvider.overrideWithValue(profileRepo),
      workoutRepositoryProvider.overrideWithValue(workoutRepo),
    ]);
    addTearDown(container.dispose);
    // A stored token restores the session, as a cold start does.
    container.read(authControllerProvider);
    final router = GoRouter(
      initialLocation: '/home',
      routes: [
        GoRoute(path: '/home', builder: (_, __) => const MainScreen()),
        GoRoute(
            path: '/session',
            builder: (_, __) => const Scaffold(body: Text('SESSION-SENTINEL'))),
        GoRoute(
            path: '/hub',
            builder: (_, __) => const Scaffold(body: Text('HUB-SENTINEL'))),
        GoRoute(
            path: '/onboarding',
            builder: (_, __) =>
                const Scaffold(body: Text('ONBOARDING-SENTINEL'))),
        GoRoute(
            path: '/programs/current',
            builder: (_, __) => const Scaffold(body: Text('PROGRAM-SENTINEL'))),
        GoRoute(
            path: '/progress',
            builder: (_, __) =>
                const Scaffold(body: Text('PROGRESS-SENTINEL'))),
      ],
    );
    await tester.pumpWidget(UncontrolledProviderScope(
      container: container,
      child: localizedRouterApp(router),
    ));
    await tester.pumpAndSettle();
    return (container, router);
  }

  group('AC7 few controls', () {
    testWidgets('the main screen has the profile menu, + Comida and the mic',
        (tester) async {
      await pumpMain(tester);
      expect(find.byKey(const Key('profile-menu')), findsOneWidget);
      expect(find.byKey(const Key('add-meal')), findsOneWidget);
      expect(find.byKey(const Key('mic')), findsOneWidget);
      expect(find.byKey(const Key('primary-action')), findsOneWidget);
      // Nothing the old shell had: no Start-workout FAB label, no logout.
      expect(find.text('Start workout'), findsNothing);
      expect(find.text('Logout'), findsNothing);
    });

    testWidgets(
        'the profile menu lists program, measurements, progress and sign out',
        (tester) async {
      await pumpMain(tester);
      await tester.tap(find.byKey(const Key('profile-menu')));
      await tester.pumpAndSettle();
      for (final s in [
        l10n.menuProgram,
        l10n.menuMeasurements,
        l10n.menuProgress,
        l10n.signOut,
      ]) {
        expect(find.text(s), findsOneWidget);
      }
    });
  });

  group('AC2 nothing is lost: the menu reaches the old screens', () {
    for (final (item, sentinel) in [
      (l10n.menuProgram, 'PROGRAM-SENTINEL'),
      (l10n.menuProgress, 'PROGRESS-SENTINEL'),
    ]) {
      testWidgets('$item pushes its screen with no exception', (tester) async {
        await pumpMain(tester);
        await tester.tap(find.byKey(const Key('profile-menu')));
        await tester.pumpAndSettle();
        await tester.tap(find.text(item));
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        expect(find.text(sentinel), findsOneWidget);
      });
    }
  });

  group('AC4 the calendar from the logs', () {
    testWidgets('a day with a session is marked done, today is selected',
        (tester) async {
      await pumpMain(tester);
      expect(find.byKey(const Key('mark-done')), findsOneWidget);
      final cell = tester.widget<DayCell>(
          find.byKey(ValueKey('day-${today.toIso8601String()}')));
      expect(cell.isToday, isTrue);
      expect(cell.isSelected, isTrue);
    });

    testWidgets('a phone shows the week and can open the month',
        (tester) async {
      await pumpMain(tester);
      expect(find.byType(DayCell), findsNWidgets(7));
      await tester.tap(find.byKey(const Key('calendar-toggle')));
      await tester.pumpAndSettle();
      expect(find.byType(DayCell).evaluate().length, greaterThanOrEqualTo(28));
    });

    testWidgets('back to the week strip, the selected day is in view',
        (tester) async {
      final (container, _) = await pumpMain(tester);
      await tester.tap(find.byKey(const Key('calendar-toggle')));
      await tester.pumpAndSettle();
      container.read(selectedDayProvider.notifier).state = yesterday;
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('calendar-toggle')));
      await tester.pumpAndSettle();
      expect(find.byKey(ValueKey('day-${yesterday.toIso8601String()}')),
          findsOneWidget);
    });

    testWidgets('a wide window shows the month with no toggle', (tester) async {
      await pumpMain(tester, size: desktop);
      expect(find.byType(DayCell).evaluate().length, greaterThanOrEqualTo(28));
      expect(find.byKey(const Key('calendar-toggle')), findsNothing);
    });

    testWidgets('‹ and › move the calendar', (tester) async {
      await pumpMain(tester);
      final before =
          tester.widget<Text>(find.byKey(const Key('calendar-title'))).data;
      await tester.tap(find.byKey(const Key('calendar-prev')));
      await tester.pumpAndSettle();
      final after =
          tester.widget<Text>(find.byKey(const Key('calendar-title'))).data;
      expect(after, isNot(before));
    });
  });

  group('AC5 the selected day', () {
    testWidgets('today offers Empezar sesión and it opens the session',
        (tester) async {
      final (container, _) = await pumpMain(tester);
      expect(find.text(l10n.startSession), findsOneWidget);
      await tester.tap(find.byKey(const Key('primary-action')));
      await tester.pumpAndSettle();
      // A push keeps the base uri; the pushed match is the last one.
      expect(find.text('SESSION-SENTINEL'), findsOneWidget);
      expect(container.read(sessionDriverProvider).draft, isNotNull);
    });

    testWidgets('a done day offers Ver lo registrado', (tester) async {
      await pumpMain(tester);
      await tester
          .tap(find.byKey(ValueKey('day-${yesterday.toIso8601String()}')));
      await tester.pumpAndSettle();
      expect(find.text(l10n.viewLogged), findsOneWidget);
    });

    testWidgets('a past empty day offers Registrar tarde, dated that day',
        (tester) async {
      final (container, _) = await pumpMain(tester);
      final twoDaysAgo = DateTime(today.year, today.month, today.day - 2);
      container.read(selectedDayProvider.notifier).state = twoDaysAgo;
      await tester.pumpAndSettle();
      expect(find.text(l10n.logLate), findsOneWidget);

      await tester.tap(find.byKey(const Key('primary-action')));
      await tester.pumpAndSettle();
      expect(container.read(sessionDriverProvider).performedOn, twoDaysAgo);
    });

    testWidgets('today\'s session carries no date: finish logs today',
        (tester) async {
      final (container, _) = await pumpMain(tester);
      await tester.tap(find.byKey(const Key('primary-action')));
      await tester.pumpAndSettle();
      expect(container.read(sessionDriverProvider).performedOn, isNull);
    });

    testWidgets('a future day offers no action', (tester) async {
      final (container, _) = await pumpMain(tester);
      container.read(selectedDayProvider.notifier).state =
          today.add(const Duration(days: 1));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('primary-action')), findsNothing);
    });
  });

  group('AC1 panels keep the selected day', () {
    testWidgets('opening and closing a panel leaves the day selected',
        (tester) async {
      final (container, _) = await pumpMain(tester);
      container.read(selectedDayProvider.notifier).state = yesterday;
      await tester.pumpAndSettle();

      // Ver lo registrado opens the day's sessions in a panel.
      await tester.tap(find.byKey(const Key('primary-action')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('panel-close')), findsOneWidget);

      await tester.tap(find.byKey(const Key('panel-close')));
      await tester.pumpAndSettle();
      expect(container.read(selectedDayProvider), yesterday);
      expect(find.byType(MainScreen), findsOneWidget);
    });
  });

  group('what moved here from HomeShell', () {
    testWidgets('no profile shows the onboarding prompt, dismiss hides it',
        (tester) async {
      when(() => profileRepo.getMe()).thenAnswer((_) async => null);
      final (container, _) = await pumpMain(tester);
      expect(find.byType(ProfilePrompt), findsOneWidget);

      container.read(onboardingDismissedProvider.notifier).state = true;
      await tester.pumpAndSettle();
      expect(find.byType(ProfilePrompt), findsNothing);
    });

    testWidgets('the screen reads the profile (the liveness probe)',
        (tester) async {
      await pumpMain(tester);
      verify(() => profileRepo.getMe()).called(1);
    });
  });

  group('AC13 sizes', () {
    for (final (name, size) in [('phone', phone), ('desktop', desktop)]) {
      testWidgets('$name: no overflow and the action is on screen',
          (tester) async {
        await pumpMain(tester, size: size);
        expect(tester.takeException(), isNull);
        final rect = tester.getRect(find.byKey(const Key('primary-action')));
        expect(rect.top, greaterThanOrEqualTo(0));
        expect(rect.left, greaterThanOrEqualTo(0));
        expect(rect.bottom, lessThanOrEqualTo(size.height));
        expect(rect.right, lessThanOrEqualTo(size.width));
        // No horizontal scroll wider than the screen.
        for (final e in find.byType(Scrollable).evaluate()) {
          final s = e.widget as Scrollable;
          if (s.axisDirection == AxisDirection.left ||
              s.axisDirection == AxisDirection.right) {
            final r = tester.getRect(find.byWidget(s));
            expect(r.width, lessThanOrEqualTo(size.width));
          }
        }
      });
    }
  });
}
