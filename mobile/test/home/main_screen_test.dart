// R-0061 AC1, AC4, AC5, AC7, AC13 -> SPEC-0061 §2.5 (slices 1f and 2d): the
// one screen, its calendar from the schedule (and from the logs alone while
// the schedule loads or fails), the selected day's one action and its
// secondary one, the few controls, the liveness probe and the onboarding
// prompt that moved here from HomeShell, and the two sizes.
//
// R-0061 AC10 -> SPEC-0061 §2.7 (slice 5): the screen builds the PlanSeed
// from the schedule it holds and pushes focus mode; a finished session
// selects its day, re-reads the schedule, and a session saved as free says
// so.

import 'dart:async';

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/home/calendar.dart';
import 'package:fitai/src/home/main_screen.dart';
import 'package:fitai/src/home/selected_day.dart';
import 'package:fitai/src/profile/application/profile_providers.dart';
import 'package:fitai/src/profile/presentation/profile_prompt.dart';
import 'package:fitai/src/program/models/schedule.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:fitai/src/workout/application/session_driver.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:fitai/src/workout/domain/plan_seed.dart';
import 'package:fitai/src/workout/domain/session_draft.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:fitai/src/workout/domain/workout_session.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/l10n.dart';
import '../support/profile_fakes.dart';
import '../support/program_fakes.dart';
import '../support/workout_fakes.dart';

const phone = Size(390, 844);
const desktop = Size(1440, 900);

void main() {
  setUpAll(() {
    registerFallbacks();
    registerProfileFallbacks();
    registerWorkoutFallbacks();
    registerProgramFallbacks();
  });

  late MockTokenStore tokenStore;
  late MockAuthRepository repo;
  late MockProfileRepository profileRepo;
  late MockWorkoutRepository workoutRepo;
  late MockProgramService programService;

  final today =
      DateTime(DateTime.now().year, DateTime.now().month, DateTime.now().day);
  final tomorrow = DateTime(today.year, today.month, today.day + 1);

  /// The done day must sit in the week strip a phone shows, which starts on
  /// Monday in `es`: doneDay, unless today is Monday, then the day after
  /// tomorrow.
  final doneDay = today.weekday == DateTime.monday
      ? DateTime(today.year, today.month, today.day + 2)
      : DateTime(today.year, today.month, today.day - 1);

  /// The cycle is [chest+arms, legs, rest] (`scheduleJson`). The done day
  /// completed day 0, today is legs, tomorrow rests; every other day is
  /// absent, so the screen falls back to the logs for it.
  Schedule schedule() => sampleSchedule(days: [
        dayJson(isoDate(doneDay), state: 'done', completed: [0]),
        dayJson(isoDate(today), state: 'planned', cycleIndex: 1),
        dayJson(isoDate(tomorrow), state: 'rest', cycleIndex: 2),
      ]);

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
    programService = MockProgramService();
    when(() => tokenStore.read()).thenAnswer((_) async => sampleToken());
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
    when(() => repo.me()).thenAnswer((_) async => 'u-1');
    when(() => profileRepo.getMe()).thenAnswer((_) async => sampleProfile());
    when(() => workoutRepo.list()).thenAnswer((_) async => [session(doneDay)]);
    whenGetSchedule(programService).thenAnswer((_) async => schedule());
    when(() => programService.getCurrent()).thenAnswer((_) async => null);
  });

  /// [settle] is false when something on screen never stops animating (the
  /// loading spinner while a stubbed request never completes).
  Future<(ProviderContainer, GoRouter)> pumpMain(WidgetTester tester,
      {Size size = phone, bool settle = true}) async {
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final container = ProviderContainer(overrides: [
      tokenStoreProvider.overrideWithValue(tokenStore),
      authRepositoryProvider.overrideWithValue(repo),
      profileRepositoryProvider.overrideWithValue(profileRepo),
      workoutRepositoryProvider.overrideWithValue(workoutRepo),
      programServiceProvider.overrideWithValue(programService),
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
    if (settle) {
      await tester.pumpAndSettle();
    } else {
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
    }
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
      // The secondary action exists only for a free workout on a plan day.
      expect(find.byKey(const Key('secondary-action')), findsNothing);
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

  group('AC4 the calendar from the schedule', () {
    testWidgets('a day with a session is marked done, today is selected',
        (tester) async {
      await pumpMain(tester);
      expect(find.byKey(const Key('mark-done')), findsOneWidget);
      final cell = tester.widget<DayCell>(
          find.byKey(ValueKey('day-${today.toIso8601String()}')));
      expect(cell.isToday, isTrue);
      expect(cell.isSelected, isTrue);
    });

    testWidgets('today is planned and tomorrow rests, as the server says',
        (tester) async {
      await pumpMain(tester, size: desktop); // the month holds both days
      expect(find.byKey(const Key('mark-planned')), findsOneWidget);
      expect(find.byKey(const Key('mark-rest')), findsOneWidget);
    });

    testWidgets('it asks for a window that covers the month shown',
        (tester) async {
      await pumpMain(tester);
      final captured = verify(() => programService.getSchedule(
            from: captureAny(named: 'from'),
            to: captureAny(named: 'to'),
            today: captureAny(named: 'today'),
          )).captured;
      final from = captured[0] as DateTime;
      final to = captured[1] as DateTime;
      expect(captured[2], today);
      expect(from.isAfter(DateTime(today.year, today.month, 1 - 6)), isFalse);
      expect(to.isBefore(DateTime(today.year, today.month + 1, 6)), isFalse);
    });

    testWidgets('while the schedule loads, the logs alone draw the calendar',
        (tester) async {
      whenGetSchedule(programService)
          .thenAnswer((_) => Completer<Schedule>().future);
      await pumpMain(tester, settle: false);
      expect(tester.takeException(), isNull);
      expect(find.byKey(const Key('mark-done')), findsOneWidget);
      expect(find.byKey(const Key('mark-planned')), findsNothing);
      expect(find.text(l10n.startSession), findsOneWidget);
    });

    testWidgets('when the schedule fails, the logs alone draw the calendar',
        (tester) async {
      whenGetSchedule(programService)
          .thenThrow(const ApiException(ApiErrorKind.transport));
      await pumpMain(tester);
      expect(tester.takeException(), isNull);
      expect(find.byKey(const Key('mark-done')), findsOneWidget);
      expect(find.text(l10n.startSession), findsOneWidget);
      await tester
          .tap(find.byKey(ValueKey('day-${doneDay.toIso8601String()}')));
      await tester.pumpAndSettle();
      expect(find.text(l10n.viewLogged), findsOneWidget);
    });

    testWidgets('a planned cell is named by its focus for a screen reader',
        (tester) async {
      await pumpMain(tester);
      final cell = tester
          .getSemantics(find.byKey(ValueKey('day-${today.toIso8601String()}')));
      expect(cell.label, contains(l10n.muscleLegs));
      final other = tester.getSemantics(
          find.byKey(ValueKey('day-${doneDay.toIso8601String()}')));
      expect(other.label, isNot(contains(l10n.muscleLegs)));
    });

    testWidgets('app resume re-reads the schedule', (tester) async {
      await pumpMain(tester);
      clearInteractions(programService); // the reads that built the screen
      // The lifecycle arrives from the engine on its channel, as it would
      // when the user comes back to the app.
      for (final state in ['paused', 'resumed']) {
        await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
          'flutter/lifecycle',
          const StringCodec().encodeMessage('AppLifecycleState.$state'),
          (_) {},
        );
      }
      await tester.pumpAndSettle();
      verify(() => programService.getSchedule(
            from: any(named: 'from'),
            to: any(named: 'to'),
            today: any(named: 'today'),
          )).called(1);
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
      container.read(selectedDayProvider.notifier).state = doneDay;
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('calendar-toggle')));
      await tester.pumpAndSettle();
      expect(find.byKey(ValueKey('day-${doneDay.toIso8601String()}')),
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
          .tap(find.byKey(ValueKey('day-${doneDay.toIso8601String()}')));
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
      expect(container.read(sessionDriverProvider).seed, isNull,
          reason: 'the server offered no pending day: a free late log');
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

    testWidgets('a planned day is named by its focus and lists its exercises',
        (tester) async {
      await pumpMain(tester);
      // Today is cycle day 1: legs.
      expect(find.text(l10n.muscleLegs), findsOneWidget);
      expect(find.text('Back Squat'), findsOneWidget);
      expect(find.text('Leg Press'), findsOneWidget);
      expect(find.text(l10n.dayFreeHeadline), findsNothing);
    });

    testWidgets('a two-group focus joins the names in the catalogue',
        (tester) async {
      whenGetSchedule(programService).thenAnswer((_) async => sampleSchedule(
            days: [dayJson(isoDate(today), state: 'planned', cycleIndex: 0)],
          ));
      await pumpMain(tester);
      expect(
        find.text(
            l10n.listAnd(l10n.muscleChest, l10n.muscleArms.toLowerCase())),
        findsOneWidget,
      );
      expect(find.text('Bench Press'), findsOneWidget);
    });

    testWidgets('a rest day says so and offers nothing', (tester) async {
      final (container, _) = await pumpMain(tester);
      container.read(selectedDayProvider.notifier).state = tomorrow;
      await tester.pumpAndSettle();
      expect(find.text(l10n.dayRestHeadline), findsOneWidget);
      expect(find.byKey(const Key('primary-action')), findsNothing);
      expect(find.text(l10n.nothingLogged), findsNothing);
    });

    testWidgets(
        'a free workout today keeps the plan day startable (N1): Ver lo '
        'registrado first, Empezar sesión second', (tester) async {
      whenGetSchedule(programService).thenAnswer((_) async => sampleSchedule(
            days: [
              dayJson(isoDate(today), state: 'done', completed: [], pending: 1),
            ],
          ));
      final (container, _) = await pumpMain(tester);
      expect(find.byKey(const Key('primary-action')), findsOneWidget);
      expect(find.text(l10n.viewLogged), findsOneWidget);
      expect(find.text(l10n.pendingPlanDay(l10n.muscleLegs)), findsOneWidget);
      expect(find.byKey(const Key('secondary-action')), findsOneWidget);
      expect(find.text(l10n.startSession), findsOneWidget);

      await tester.tap(find.byKey(const Key('secondary-action')));
      await tester.pumpAndSettle();
      expect(find.text('SESSION-SENTINEL'), findsOneWidget);
      expect(container.read(sessionDriverProvider).draft, isNotNull);
      expect(container.read(sessionDriverProvider).performedOn, isNull);
    });

    testWidgets('a done day with a plan session has no secondary action',
        (tester) async {
      await pumpMain(tester);
      await tester
          .tap(find.byKey(ValueKey('day-${doneDay.toIso8601String()}')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('secondary-action')), findsNothing);
    });
  });

  group('AC1 panels keep the selected day', () {
    testWidgets('opening and closing a panel leaves the day selected',
        (tester) async {
      final (container, _) = await pumpMain(tester);
      container.read(selectedDayProvider.notifier).state = doneDay;
      await tester.pumpAndSettle();

      // Ver lo registrado opens the day's sessions in a panel.
      await tester.tap(find.byKey(const Key('primary-action')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('panel-close')), findsOneWidget);

      await tester.tap(find.byKey(const Key('panel-close')));
      await tester.pumpAndSettle();
      expect(container.read(selectedDayProvider), doneDay);
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

  group('AC10 focus mode', () {
    final twoDaysAgo = DateTime(today.year, today.month, today.day - 2);

    PlanSeed? seedOf(ProviderContainer c) => c.read(sessionDriverProvider).seed;

    testWidgets('Empezar sesión seeds today\'s plan day and pushes focus mode',
        (tester) async {
      final (container, _) = await pumpMain(tester);
      await tester.tap(find.byKey(const Key('primary-action')));
      await tester.pumpAndSettle();

      expect(find.text('SESSION-SENTINEL'), findsOneWidget);
      final seed = seedOf(container);
      expect(seed?.userProgramId, 'prog-uuid-001');
      expect(seed?.index, 1);
      expect(seed?.exercises, ['Back Squat', 'Leg Press']);
      expect(
        container
            .read(sessionDriverProvider)
            .draft!
            .exercises
            .map((e) => e.name),
        ['Back Squat', 'Leg Press'],
      );
    });

    testWidgets('the secondary Empezar sesión seeds the pending day (N1)',
        (tester) async {
      whenGetSchedule(programService).thenAnswer((_) async => sampleSchedule(
            days: [
              dayJson(isoDate(today), state: 'done', completed: [], pending: 0),
            ],
          ));
      final (container, _) = await pumpMain(tester);
      await tester.tap(find.byKey(const Key('secondary-action')));
      await tester.pumpAndSettle();

      expect(seedOf(container)?.index, 0);
      expect(seedOf(container)?.exercises, ['Bench Press', 'Barbell Curl']);
      expect(container.read(sessionDriverProvider).performedOn, isNull);
    });

    testWidgets(
        'Registrar tarde seeds that day\'s pending training day, dated '
        'that day', (tester) async {
      whenGetSchedule(programService).thenAnswer((_) async => sampleSchedule(
            days: [
              dayJson(isoDate(twoDaysAgo), state: 'nothing', pending: 0),
              dayJson(isoDate(today), state: 'planned', cycleIndex: 1),
            ],
          ));
      final (container, _) = await pumpMain(tester);
      container.read(selectedDayProvider.notifier).state = twoDaysAgo;
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const Key('primary-action')));
      await tester.pumpAndSettle();

      expect(find.text('SESSION-SENTINEL'), findsOneWidget);
      expect(container.read(sessionDriverProvider).performedOn, twoDaysAgo);
      expect(seedOf(container)?.index, 0);
      expect(seedOf(container)?.exercises, ['Bench Press', 'Barbell Curl']);
    });

    testWidgets('with no program the session is free', (tester) async {
      whenGetSchedule(programService)
          .thenAnswer((_) async => sampleSchedule(programId: null));
      final (container, _) = await pumpMain(tester);
      await tester.tap(find.byKey(const Key('primary-action')));
      await tester.pumpAndSettle();

      expect(seedOf(container), isNull);
      expect(container.read(sessionDriverProvider).draft!.exercises, isEmpty);
    });

    testWidgets(
        'a finished session selects its day, shown as done, and the '
        'schedule is re-read', (tester) async {
      when(() => workoutRepo.create(any()))
          .thenAnswer((_) async => sampleSession());
      final (container, _) = await pumpMain(tester);
      when(() => workoutRepo.list())
          .thenAnswer((_) async => [session(doneDay), session(twoDaysAgo)]);
      clearInteractions(programService);

      final driver = container.read(sessionDriverProvider.notifier);
      driver
        ..start(performedOn: twoDaysAgo)
        ..addExercise('Squat')
        ..logSet(const SetDraft(reps: 5));
      await driver.finish();
      await tester.pumpAndSettle();

      expect(container.read(selectedDayProvider), twoDaysAgo);
      expect(find.text(l10n.dayDoneHeadline), findsOneWidget);
      expect(find.text(l10n.viewLogged), findsOneWidget);
      verify(() => programService.getSchedule(
            from: any(named: 'from'),
            to: any(named: 'to'),
            today: any(named: 'today'),
          )).called(1);
    });

    for (final (name, status, reason, notice) in [
      (
        '409 program_not_current',
        409,
        'program_not_current',
        l10n.noticeProgramNotCurrent
      ),
      (
        '422 plan_unavailable',
        422,
        'plan_unavailable',
        l10n.noticePlanUnavailable
      ),
    ]) {
      testWidgets('a $name saves the session as free and shows the notice',
          (tester) async {
        when(() => workoutRepo.create(any())).thenAnswer((inv) async {
          final req = inv.positionalArguments.single as SessionRequest;
          if (req.toJson().containsKey('user_program_id')) {
            throw ApiException(ApiErrorKind.unknown,
                statusCode: status, reason: reason);
          }
          return sampleSession();
        });
        final (container, _) = await pumpMain(tester);
        await tester.tap(find.byKey(const Key('primary-action')));
        await tester.pumpAndSettle();

        final driver = container.read(sessionDriverProvider.notifier);
        driver
          ..selectExercise(0)
          ..logSet(const SetDraft(reps: 5))
          ..selectExercise(1)
          ..logSet(const SetDraft(reps: 5));
        await driver.finish();
        await tester.pump();

        expect(container.read(sessionDriverProvider).done, isTrue);
        expect(find.text(notice), findsOneWidget);
        final sent = verify(() => workoutRepo.create(captureAny()))
            .captured
            .cast<SessionRequest>();
        expect(sent, hasLength(2));
        expect(sent.last.toJson().containsKey('user_program_id'), isFalse);
      });
    }
  });
}
