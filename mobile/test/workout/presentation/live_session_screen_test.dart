// SAC1-SAC7 -> AC1-AC7 (the live in-gym screen): LiveSessionScreen is a THIN
// renderer over the session driver — it shows driver state, calls the typed
// API, ref.listens `done` to navigate, and holds no try/catch (SAC10: the
// business logic is all driver-side, proven widget-free in
// session_driver_test.dart).
//
// Pinned user-facing affordances (the contract the implementation renders):
//   'Start workout' (home), 'Add exercise', 'Add' (sheet confirm), a 'Reps'
//   text field, 'Log set', 'Repeat last set', 'Finish', and the abandon
//   dialog's 'Cancel' / 'Discard'.
//
// Harness rules (the R-0007 lessons):
//   * never `await Future.delayed` before a pump — timers run on the fake
//     clock;
//   * never `pumpAndSettle` while a spinner may be up — bounded pump()s only;
//     the in-flight submit is observed with a Completer;
//   * Dio-level errors are driven as ApiException states via the mocked
//     repository (the screen never sees a DioException).
//
// RED until package:fitai/src/workout/** exists (screen, driver, providers,
// preset list) and the home shell gains the Start action.

import 'dart:async';

import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/profile/application/profile_providers.dart';
import 'package:fitai/src/hub/speech_input.dart';
import 'package:fitai/src/hub/voice_output.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:fitai/src/shell/home_shell.dart';
import 'package:fitai/src/workout/application/session_driver.dart';
import 'package:fitai/src/workout/application/voice_coach.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:fitai/src/workout/domain/workout_session.dart';
import 'package:fitai/src/workout/presentation/live_session_screen.dart';
import 'package:fitai/src/workout/presentation/preset_exercises.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/profile_fakes.dart';
import '../../support/program_fakes.dart';
import '../../support/voice_fakes.dart';
import '../../support/workout_fakes.dart';

void main() {
  setUpAll(registerWorkoutFallbacks);
  setUpAll(registerProgramFallbacks);

  late MockWorkoutRepository repo;

  setUp(() {
    repo = MockWorkoutRepository();
    when(() => repo.list()).thenAnswer((_) async => []);
  });

  String locationOf(GoRouter r) =>
      r.routerDelegate.currentConfiguration.uri.path;

  SessionDriver driverOf(ProviderContainer c) =>
      c.read(sessionDriverProvider.notifier);
  SessionDriverState stateOf(ProviderContainer c) =>
      c.read(sessionDriverProvider);

  // Hosts the LiveSessionScreen under a minimal router so navigation is
  // observable; /home is a sentinel. `started` controls whether a session is
  // in progress before the first pump (the null-draft case is OQ-F5).
  Future<(ProviderContainer, GoRouter)> pumpSession(
    WidgetTester tester, {
    bool started = true,
  }) async {
    final programService = MockProgramService();
    when(() => programService.getCurrent()).thenAnswer((_) async => null);
    final container = ProviderContainer(
      overrides: [
        workoutRepositoryProvider.overrideWithValue(repo),
        programServiceProvider.overrideWithValue(programService),
        speechInputProvider.overrideWithValue(ScriptedSpeechInput([])),
        voiceOutputProvider.overrideWithValue(RecordingVoiceOutput()),
      ],
    );
    addTearDown(container.dispose);
    if (started) driverOf(container).start();

    final router = GoRouter(
      initialLocation: '/session',
      routes: [
        GoRoute(
          path: '/session',
          builder: (_, __) => const LiveSessionScreen(),
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
        child: MaterialApp.router(routerConfig: router),
      ),
    );
    await tester.pump();
    return (container, router);
  }

  group('SAC1 start + today-session', () {
    testWidgets('a started session renders the live screen with NO date input',
        (tester) async {
      await pumpSession(tester);

      expect(find.text('Add exercise'), findsWidgets);
      expect(find.text('Finish'), findsWidgets);
      // AC1: the user never types a date — no calendar affordance exists.
      expect(find.byIcon(Icons.calendar_today), findsNothing);
      expect(find.byIcon(Icons.edit_calendar), findsNothing);
    });

    testWidgets('/session with no draft immediately redirects home (OQ-F5)',
        (tester) async {
      final (_, router) = await pumpSession(tester, started: false);
      await tester.pump();

      expect(locationOf(router), '/home');
    });

    testWidgets(
        'Start workout on the home shell opens /session with a session '
        'in progress', (tester) async {
      final profileRepo = MockProfileRepository();
      final programService = MockProgramService();
      when(() => profileRepo.getMe()).thenAnswer((_) async => sampleProfile());
      when(() => programService.getCurrent()).thenAnswer((_) async => null);
      final container = ProviderContainer(
        overrides: [
          workoutRepositoryProvider.overrideWithValue(repo),
          profileRepositoryProvider.overrideWithValue(profileRepo),
          programServiceProvider.overrideWithValue(programService),
        ],
      );
      addTearDown(container.dispose);
      final router = GoRouter(
        initialLocation: '/home',
        routes: [
          GoRoute(path: '/home', builder: (_, __) => const HomeShell()),
          GoRoute(
            path: '/session',
            builder: (_, __) => const LiveSessionScreen(),
          ),
        ],
      );
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp.router(routerConfig: router),
        ),
      );
      await tester.pump();
      await tester.pump();

      await tester.tap(find.text('Start workout'));
      await tester.pump();
      await tester.pump();

      expect(locationOf(router), '/session');
      expect(stateOf(container).draft, isNotNull,
          reason: 'starting opens an empty in-progress session');
      expect(find.byIcon(Icons.calendar_today), findsNothing);
    });
  });

  group('SAC2 add exercise (preset pre-fill + free text)', () {
    testWidgets('a preset chip pre-fills the SAME free-text field',
        (tester) async {
      final (container, _) = await pumpSession(tester);
      final preset = presetExercises.first;

      await tester.tap(find.text('Add exercise').first);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400)); // sheet animation

      await tester.tap(find.text(preset.name).first);
      await tester.pump();
      expect(find.widgetWithText(TextField, preset.name), findsOneWidget,
          reason: 'the preset pre-fills the free-text name field');

      await tester.tap(find.text('Add'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));

      expect(stateOf(container).draft!.exercises.single.name, preset.name);
    });

    testWidgets('typed free text adds an (untagged) exercise', (tester) async {
      final (container, _) = await pumpSession(tester);

      await tester.tap(find.text('Add exercise').first);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));

      await tester.enterText(find.byType(TextField).first, 'Cable crossover');
      await tester.tap(find.text('Add'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));

      final exercise = stateOf(container).draft!.exercises.single;
      expect(exercise.name, 'Cable crossover');
      expect(exercise.muscleGroup, isNull, reason: 'the tag is optional');
    });
  });

  group('SAC3 set entry', () {
    testWidgets('a valid reps-only set is logged (optionals stay absent)',
        (tester) async {
      final (container, _) = await pumpSession(tester);
      driverOf(container).addExercise('Bench press');
      await tester.pump();

      await tester.enterText(find.widgetWithText(TextField, 'Reps'), '5');
      await tester.tap(find.text('Log set'));
      await tester.pump();

      final sets = stateOf(container).draft!.exercises.single.sets;
      expect(sets.single.reps, 5);
      expect(sets.single.weightKg, isNull);
      expect(sets.single.rpe, isNull);
    });

    testWidgets('invalid reps block the add with the validator message',
        (tester) async {
      final (container, _) = await pumpSession(tester);
      driverOf(container).addExercise('Bench press');
      await tester.pump();

      await tester.enterText(find.widgetWithText(TextField, 'Reps'), '0');
      await tester.tap(find.text('Log set'));
      await tester.pump();

      final message = const SetDraft(reps: 0).repsError()!;
      expect(find.textContaining(message), findsWidgets,
          reason: 'the driver-returned reason is shown inline');
      expect(stateOf(container).draft!.exercises.single.sets, isEmpty,
          reason: 'the invalid set was never appended');
    });

    testWidgets(
        'Repeat last set pre-fills the previous set — logging it '
        'reproduces the values', (tester) async {
      final (container, _) = await pumpSession(tester);
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8, weightKg: 80, rpe: 8.5));
      await tester.pump();

      await tester.tap(find.text('Repeat last set'));
      await tester.pump();
      await tester.tap(find.text('Log set'));
      await tester.pump();

      final sets = stateOf(container).draft!.exercises.single.sets;
      expect(sets, hasLength(2));
      expect(sets[1].reps, 8);
      expect(sets[1].weightKg, 80.0);
      expect(sets[1].rpe, 8.5);
    });
  });

  group('SAC4 the draft survives in-app navigation', () {
    testWidgets('leaving /session and returning re-renders the same draft',
        (tester) async {
      final (container, router) = await pumpSession(tester);
      driverOf(container)
        ..addExercise('Deadlift')
        ..logSet(const SetDraft(reps: 5));
      await tester.pump();
      expect(find.textContaining('Deadlift'), findsWidgets);

      router.go('/home');
      await tester.pump();
      await tester.pump();
      expect(find.text('HOME-SENTINEL'), findsOneWidget);

      router.go('/session');
      await tester.pump();
      await tester.pump();

      expect(find.textContaining('Deadlift'), findsWidgets);
      expect(stateOf(container).draft!.exercises.single.sets, hasLength(1),
          reason: 'in-memory draft, untouched by navigation');
    });
  });

  group('SAC5 finish', () {
    testWidgets('Finish does nothing while the session is not finishable',
        (tester) async {
      final (container, router) = await pumpSession(tester);
      driverOf(container).addExercise('Bench press'); // no sets yet
      await tester.pump();

      await tester.tap(find.text('Finish').first, warnIfMissed: false);
      await tester.pump();
      await tester.pump();

      verifyNever(() => repo.create(any()));
      expect(locationOf(router), '/session');
    });

    testWidgets('a successful finish lands on home with the draft cleared',
        (tester) async {
      when(() => repo.create(any())).thenAnswer((_) async => sampleSession());
      when(() => repo.list()).thenAnswer((_) async => [sampleSession()]);
      final (container, router) = await pumpSession(tester);
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8));
      await tester.pump();

      await tester.tap(find.text('Finish').first);
      await tester.pump();
      await tester.pump();
      await tester.pump();

      expect(locationOf(router), '/home');
      expect(stateOf(container).draft, isNull);
      verify(() => repo.create(any())).called(1);
    });

    testWidgets(
        'the finish submit disables while in flight (Completer, '
        'bounded pumps)', (tester) async {
      final gate = Completer<WorkoutSession>();
      when(() => repo.create(any())).thenAnswer((_) => gate.future);
      when(() => repo.list()).thenAnswer((_) async => [sampleSession()]);
      final (container, router) = await pumpSession(tester);
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8));
      await tester.pump();

      await tester.tap(find.text('Finish').first);
      await tester.pump();

      expect(stateOf(container).submitting, isTrue,
          reason: 'state reflects the in-flight POST');
      expect(locationOf(router), '/session', reason: 'no early navigation');

      gate.complete(sampleSession());
      await tester.pump();
      await tester.pump();
      await tester.pump();
      expect(locationOf(router), '/home');
    });
  });

  group('SAC6 finish failure (each branch, no data loss)', () {
    testWidgets('a 400{field} stays put, shows the message, keeps the draft',
        (tester) async {
      when(() => repo.create(any())).thenThrow(
        const ApiException('please check your details',
            statusCode: 400, field: 'rpe'),
      );
      final (container, router) = await pumpSession(tester);
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8, rpe: 8.5));
      await tester.pump();

      await tester.tap(find.text('Finish').first);
      await tester.pump();
      await tester.pump();

      expect(locationOf(router), '/session', reason: 'no navigation on error');
      expect(find.textContaining('check your details'), findsWidgets);
      expect(find.textContaining('Bench press'), findsWidgets,
          reason: 'the session is still rendered — nothing was lost');
      expect(stateOf(container).draft, isNotNull);
    });

    testWidgets(
        'a transport error shows a retryable message; retrying '
        're-submits and succeeds', (tester) async {
      when(() => repo.create(any()))
          .thenThrow(const ApiException("can't reach the server — retry"));
      final (container, router) = await pumpSession(tester);
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8));
      await tester.pump();

      await tester.tap(find.text('Finish').first);
      await tester.pump();
      await tester.pump();

      expect(locationOf(router), '/session');
      expect(find.textContaining('retry'), findsWidgets);

      when(() => repo.create(any())).thenAnswer((_) async => sampleSession());
      when(() => repo.list()).thenAnswer((_) async => [sampleSession()]);
      await tester.tap(find.text('Finish').first);
      await tester.pump();
      await tester.pump();
      await tester.pump();

      verify(() => repo.create(any())).called(2);
      expect(locationOf(router), '/home');
    });
  });

  group('SAC7 abandon asks for confirmation', () {
    testWidgets('back with a draft asks; Cancel keeps the draft and stays',
        (tester) async {
      final (container, router) = await pumpSession(tester);
      driverOf(container).addExercise('Bench press');
      await tester.pump();

      await tester.binding.handlePopRoute();
      await tester.pump();
      expect(find.text('Cancel'), findsOneWidget,
          reason: 'a confirm dialog intercepts the pop');

      await tester.tap(find.text('Cancel'));
      await tester.pump();
      await tester.pump();

      expect(locationOf(router), '/session');
      expect(stateOf(container).draft, isNotNull, reason: 'nothing discarded');
    });

    testWidgets(
        'confirming Discard clears the draft, leaves, persists '
        'NOTHING', (tester) async {
      final (container, router) = await pumpSession(tester);
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8));
      await tester.pump();

      await tester.binding.handlePopRoute();
      await tester.pump();
      await tester.tap(find.text('Discard'));
      await tester.pump();
      await tester.pump();

      expect(stateOf(container).draft, isNull);
      expect(locationOf(router), '/home');
      verifyNever(() => repo.create(any()));
    });
  });

  group('SAC8 voice coach toggle', () {
    testWidgets('AppBar headset button toggles voiceCoachProvider',
        (tester) async {
      final (container, _) = await pumpSession(tester);
      await tester.pump();

      expect(find.byIcon(Icons.headset_off_outlined), findsOneWidget);
      expect(find.byIcon(Icons.headset_mic), findsNothing);
      expect(container.read(voiceCoachProvider).enabled, isFalse);

      await tester.tap(find.byIcon(Icons.headset_off_outlined));
      await tester.pump();
      await tester.pump();

      expect(find.byIcon(Icons.headset_mic), findsOneWidget);
      expect(find.byIcon(Icons.headset_off_outlined), findsNothing);
      expect(container.read(voiceCoachProvider).enabled, isTrue);

      await tester.tap(find.byIcon(Icons.headset_mic));
      await tester.pump();
      await tester.pump();

      expect(find.byIcon(Icons.headset_off_outlined), findsOneWidget);
      expect(find.byIcon(Icons.headset_mic), findsNothing);
      expect(container.read(voiceCoachProvider).enabled, isFalse);
    });
  });
}
