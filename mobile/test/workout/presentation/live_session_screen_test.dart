// SAC1-SAC7 -> AC1-AC7 (the live in-gym screen): LiveSessionScreen is a THIN
// renderer over the session driver — it shows driver state, calls the typed
// API, ref.listens `done` to navigate, and holds no try/catch (SAC10: the
// business logic is all driver-side, proven widget-free in
// session_driver_test.dart).
//
// Pinned user-facing affordances (the contract the implementation renders):
//   startWorkout (home), addExercise, add (sheet confirm), a repsLabel text
//   field, logSet, repeatLastSet, finish, and the leave dialog's
//   leaveWithoutSaving / cancel / discard — named by catalogue key (R-0061
//   AC11).
//
// R-0061 AC10 -> SPEC-0061 §2.7 (slice 5): focus mode is pushed over the day
// and pops itself when the driver reaches `done`, whichever way the session
// was finished (button, voice, earbud); a seed pre-fills the exercises.
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
import 'package:fitai/src/workout/application/earbud_coach.dart';
import 'package:fitai/src/workout/application/session_driver.dart';
import 'package:fitai/src/workout/application/voice_coach.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:fitai/src/workout/domain/draft_rejection.dart';
import 'package:fitai/src/workout/domain/plan_seed.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:fitai/src/workout/application/draft_rejection_text.dart';
import 'package:fitai/src/workout/domain/workout_session.dart';
import 'package:fitai/src/workout/presentation/live_session_screen.dart';
import 'package:fitai/src/workout/domain/preset_exercises.dart';
import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/home/main_screen.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/fakes.dart';
import '../../support/l10n.dart';
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

  /// The screen on top: a push keeps the base uri, so the last match says
  /// where the user is.
  String locationOf(GoRouter r) =>
      r.routerDelegate.currentConfiguration.last.matchedLocation;

  SessionDriver driverOf(ProviderContainer c) =>
      c.read(sessionDriverProvider.notifier);
  SessionDriverState stateOf(ProviderContainer c) =>
      c.read(sessionDriverProvider);

  // Hosts the LiveSessionScreen pushed over a /home sentinel, as the app
  // does (SPEC-0061 §2.5), so a pop is observable. `started` controls whether
  // a session is in progress before the first pump; [seed] starts a plan
  // session; [transcripts] script the voice coach's dictations.
  Future<(ProviderContainer, GoRouter)> pumpSession(
    WidgetTester tester, {
    bool started = true,
    PlanSeed? seed,
    List<String> transcripts = const [],
  }) async {
    final programService = MockProgramService();
    when(() => programService.getCurrent()).thenAnswer((_) async => null);
    final container = ProviderContainer(
      overrides: [
        workoutRepositoryProvider.overrideWithValue(repo),
        programServiceProvider.overrideWithValue(programService),
        speechInputProvider.overrideWithValue(ScriptedSpeechInput(transcripts)),
        voiceOutputProvider.overrideWithValue(RecordingVoiceOutput()),
        l10nOverride,
      ],
    );
    addTearDown(container.dispose);
    if (started) driverOf(container).start(seed: seed);

    final router = GoRouter(
      initialLocation: '/home',
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
        child: localizedRouterApp(router),
      ),
    );
    router.push('/session');
    await tester.pumpAndSettle(); // the push transition
    return (container, router);
  }

  group('SAC1 start + today-session', () {
    testWidgets('a started session renders the live screen with NO date input',
        (tester) async {
      await pumpSession(tester);

      expect(find.text(l10n.addExercise), findsWidgets);
      expect(find.text(l10n.finish), findsWidgets);
      // AC1: the user never types a date — no calendar affordance exists.
      expect(find.byIcon(Icons.calendar_today), findsNothing);
      expect(find.byIcon(Icons.edit_calendar), findsNothing);
    });

    testWidgets(
        '/session with no draft renders nothing and throws nothing (the '
        'router redirects a cold start, SPEC-0061 §2.5)', (tester) async {
      await pumpSession(tester, started: false);
      await tester.pump();

      expect(tester.takeException(), isNull);
      expect(find.text(l10n.finish), findsNothing);
    });

    testWidgets('a PlanSeed pre-fills the plan day\'s exercises (AC10)',
        (tester) async {
      await pumpSession(
        tester,
        seed: const PlanSeed(
          userProgramId: 'prog-1',
          index: 1,
          exercises: ['Back Squat', 'Leg Press'],
        ),
      );

      expect(find.textContaining('Back Squat'), findsWidgets);
      expect(find.textContaining('Leg Press'), findsWidgets);
    });

    testWidgets(
        'Empezar sesión on the main screen opens /session with a session '
        'in progress', (tester) async {
      final profileRepo = MockProfileRepository();
      final programService = MockProgramService();
      final tokenStore = MockTokenStore();
      final authRepo = MockAuthRepository();
      when(() => tokenStore.read()).thenAnswer((_) async => sampleToken());
      when(() => tokenStore.clear()).thenAnswer((_) async {});
      when(() => authRepo.clear()).thenAnswer((_) async {});
      when(() => authRepo.me()).thenAnswer((_) async => 'u-1');
      when(() => profileRepo.getMe()).thenAnswer((_) async => sampleProfile());
      when(() => programService.getCurrent()).thenAnswer((_) async => null);
      final container = ProviderContainer(
        overrides: [
          workoutRepositoryProvider.overrideWithValue(repo),
          profileRepositoryProvider.overrideWithValue(profileRepo),
          programServiceProvider.overrideWithValue(programService),
          tokenStoreProvider.overrideWithValue(tokenStore),
          authRepositoryProvider.overrideWithValue(authRepo),
        ],
      );
      container.read(authControllerProvider);
      addTearDown(container.dispose);
      final router = GoRouter(
        initialLocation: '/home',
        routes: [
          GoRoute(path: '/home', builder: (_, __) => const MainScreen()),
          GoRoute(
            path: '/session',
            builder: (_, __) => const LiveSessionScreen(),
          ),
        ],
      );
      // The shell hosts the program card, which reads the catalogue.
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: localizedRouterApp(router),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const Key('primary-action')));
      await tester.pumpAndSettle();

      // The session is pushed over the main screen, so the base uri stays
      // /home; the live screen itself is the proof.
      expect(find.byType(LiveSessionScreen), findsOneWidget);
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

      await tester.tap(find.text(l10n.addExercise).first);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400)); // sheet animation

      await tester.tap(find.text(preset.name).first);
      await tester.pump();
      expect(find.widgetWithText(TextField, preset.name), findsOneWidget,
          reason: 'the preset pre-fills the free-text name field');

      await tester.tap(find.text(l10n.add));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));

      expect(stateOf(container).draft!.exercises.single.name, preset.name);
    });

    testWidgets('typed free text adds an (untagged) exercise', (tester) async {
      final (container, _) = await pumpSession(tester);

      await tester.tap(find.text(l10n.addExercise).first);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));

      await tester.enterText(find.byType(TextField).first, 'Cable crossover');
      await tester.tap(find.text(l10n.add));
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

      await tester.enterText(
          find.widgetWithText(TextField, l10n.repsLabel), '5');
      await tester.tap(find.text(l10n.logSet));
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

      await tester.enterText(
          find.widgetWithText(TextField, l10n.repsLabel), '0');
      await tester.tap(find.text(l10n.logSet));
      await tester.pump();

      final message = draftRejectionText(DraftRejection.repsOutOfRange, l10n);
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

      await tester.tap(find.text(l10n.repeatLastSet));
      await tester.pump();
      await tester.tap(find.text(l10n.logSet));
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

      await tester.tap(find.text(l10n.finish).first, warnIfMissed: false);
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

      await tester.tap(find.text(l10n.finish).first);
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

      await tester.tap(find.text(l10n.finish).first);
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
        const ApiException(ApiErrorKind.badRequest,
            statusCode: 400, field: 'rpe'),
      );
      final (container, router) = await pumpSession(tester);
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8, rpe: 8.5));
      await tester.pump();

      await tester.tap(find.text(l10n.finish).first);
      await tester.pump();
      await tester.pump();

      expect(locationOf(router), '/session', reason: 'no navigation on error');
      expect(find.textContaining(l10n.apiErrorBadRequest), findsWidgets);
      expect(find.textContaining('Bench press'), findsWidgets,
          reason: 'the session is still rendered — nothing was lost');
      expect(stateOf(container).draft, isNotNull);
    });

    testWidgets(
        'a transport error shows a retryable message; retrying '
        're-submits and succeeds', (tester) async {
      when(() => repo.create(any()))
          .thenThrow(const ApiException(ApiErrorKind.transport));
      final (container, router) = await pumpSession(tester);
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8));
      await tester.pump();

      await tester.tap(find.text(l10n.finish).first);
      await tester.pump();
      await tester.pump();

      expect(locationOf(router), '/session');
      expect(find.textContaining(l10n.apiErrorTransport), findsWidgets);

      when(() => repo.create(any())).thenAnswer((_) async => sampleSession());
      when(() => repo.list()).thenAnswer((_) async => [sampleSession()]);
      await tester.tap(find.text(l10n.finish).first);
      await tester.pump();
      await tester.pump();
      await tester.pump();

      verify(() => repo.create(any())).called(2);
      expect(locationOf(router), '/home');
    });
  });

  group('SAC7 / AC10 leaving with unsaved sets asks for confirmation', () {
    testWidgets(
        'back with logged sets asks "Salir sin guardar"; Cancel keeps the '
        'draft and stays', (tester) async {
      final (container, router) = await pumpSession(tester);
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8));
      await tester.pump();

      await tester.binding.handlePopRoute();
      await tester.pump();
      expect(find.text(l10n.leaveWithoutSaving), findsOneWidget,
          reason: 'a confirm dialog intercepts the pop');
      expect(find.text(l10n.cancel), findsOneWidget);

      await tester.tap(find.text(l10n.cancel));
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
      await tester.tap(find.text(l10n.discard));
      await tester.pump();
      await tester.pump();

      expect(stateOf(container).draft, isNull);
      expect(locationOf(router), '/home');
      verifyNever(() => repo.create(any()));
    });

    testWidgets('back with no logged set leaves at once, nothing to lose',
        (tester) async {
      final (container, router) = await pumpSession(tester);
      driverOf(container).addExercise('Bench press'); // no sets
      await tester.pump();

      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();

      expect(find.text(l10n.leaveWithoutSaving), findsNothing);
      expect(locationOf(router), '/home');
      expect(stateOf(container).draft, isNotNull,
          reason: 'nothing dropped: the draft survives in-app navigation');
      verifyNever(() => repo.create(any()));
    });
  });

  group('AC10 every way to finish goes through done and pops', () {
    testWidgets('the Finish button pops back to the day', (tester) async {
      when(() => repo.create(any())).thenAnswer((_) async => sampleSession());
      when(() => repo.list()).thenAnswer((_) async => [sampleSession()]);
      final (container, router) = await pumpSession(tester);
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8));
      await tester.pump();

      await tester.tap(find.text(l10n.finish).first);
      await tester.pumpAndSettle();

      expect(stateOf(container).done, isTrue);
      expect(locationOf(router), '/home');
      expect(find.byType(LiveSessionScreen), findsNothing);
    });

    testWidgets('"finish workout" by voice pops back to the day',
        (tester) async {
      when(() => repo.create(any())).thenAnswer((_) async => sampleSession());
      when(() => repo.list()).thenAnswer((_) async => [sampleSession()]);
      final (container, router) =
          await pumpSession(tester, transcripts: ['finish workout']);
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8));
      await tester.pump();

      final coach = container.read(voiceCoachProvider.notifier);
      await coach.enable();
      await tester.pump();
      await coach.dictate();
      await tester.pumpAndSettle();

      expect(stateOf(container).done, isTrue);
      verify(() => repo.create(any())).called(1);
      expect(locationOf(router), '/home');
      expect(find.byType(LiveSessionScreen), findsNothing);
    });

    testWidgets('the earbud\'s finish() pops back to the day (R-0035)',
        (tester) async {
      when(() => repo.create(any())).thenAnswer((_) async => sampleSession());
      when(() => repo.list()).thenAnswer((_) async => [sampleSession()]);
      final (container, router) = await pumpSession(tester);
      // Three sets on the last exercise: the next media press finishes.
      driverOf(container)
        ..addExercise('Bench press')
        ..logSet(const SetDraft(reps: 8))
        ..logSet(const SetDraft(reps: 8))
        ..logSet(const SetDraft(reps: 8));
      await tester.pump();

      container.read(earbudCoachProvider).handleMediaButton();
      await tester.pumpAndSettle();

      expect(stateOf(container).done, isTrue);
      verify(() => repo.create(any())).called(1);
      expect(locationOf(router), '/home');
      expect(find.byType(LiveSessionScreen), findsNothing);
    });
  });

  group('SAC8 coach toggles and media button parity', () {
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

    testWidgets('Earbud toggle is mutually exclusive with Voice Dictation',
        (tester) async {
      final (container, _) = await pumpSession(tester);
      await tester.pump();

      // Enable earbud mode
      await tester.tap(find.byIcon(Icons.headphones_outlined));
      await tester.pump();
      await tester.pump();
      expect(find.byIcon(Icons.headphones), findsOneWidget);

      // Now enable voice dictation — this must turn OFF the earbud mode.
      await tester.tap(find.byIcon(Icons.headset_off_outlined));
      await tester.pump();
      await tester.pump();

      expect(container.read(voiceCoachProvider).enabled, isTrue);
      // Earbud toggle should revert to outlined (off)
      expect(find.byIcon(Icons.headphones_outlined), findsOneWidget);
      expect(find.byIcon(Icons.headphones), findsNothing);
      expect(container.read(earbudModeProvider), isFalse);
    });

    testWidgets('Earbud coach advances the session identically to Log set',
        (tester) async {
      final (container, _) = await pumpSession(tester);
      driverOf(container).addExercise('Bench press');
      await tester.pump();
      await tester.pumpAndSettle();

      // Enable earbud mode
      await tester.tap(find.byIcon(Icons.headphones_outlined));
      await tester.pump();
      await tester.pump();

      // First, log an initial set on-screen (button requires one set to exist now)
      await tester.enterText(
          find.widgetWithText(TextField, l10n.repsLabel), '5');
      await tester.tap(find.text(l10n.logSet));
      await tester.pump();

      final exerciseBefore = stateOf(container).draft!.exercises.single;
      expect(exerciseBefore.sets, hasLength(1));
      expect(exerciseBefore.sets[0].reps, 5);

      // Simulate media button press via the provider's initialized coach
      // This should identically repeat the last set (reps: 5)
      container.read(earbudCoachProvider).handleMediaButton();
      await tester.pump();

      final exerciseAfter = stateOf(container).draft!.exercises.single;
      expect(exerciseAfter.sets, hasLength(2));

      // Simulate on-screen 'Repeat last set' and then 'Log set' for parity
      await tester.tap(find.text(l10n.repeatLastSet));
      await tester.pump();
      await tester.tap(find.text(l10n.logSet));
      await tester.pump();

      final exerciseAfterScreen = stateOf(container).draft!.exercises.single;
      expect(exerciseAfterScreen.sets, hasLength(3));

      // Parity check: both advanced the set identically (repeating the 5-rep set)
      expect(exerciseAfter.sets[1].reps, 5);
      expect(exerciseAfterScreen.sets[2].reps, 5);
    });
  });
}
