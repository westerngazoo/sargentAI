// R-0032 — widget tests for the voice hub with the hands-free Sergeant.
//
// Covers: ring renders all six options + mic; taps navigate; dictation runs
// the conversation (meal logged by voice alone, macro follow-up, program
// summary + navigation, workout handoff to the hands-free coach); speech
// unavailable and unknown transcripts degrade gracefully.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/hub/speech_input.dart';
import 'package:fitai/src/hub/voice_hub_screen.dart';
import 'package:fitai/src/hub/voice_output.dart';
import 'package:fitai/src/nutrition/services/nutrition_service.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:fitai/src/workout/application/session_driver.dart';
import 'package:fitai/src/workout/application/voice_coach.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/profile_fakes.dart';
import '../support/program_fakes.dart';
import '../support/voice_fakes.dart';
import '../support/workout_fakes.dart';

void main() {
  setUpAll(registerFallbacks);
  setUpAll(registerProfileFallbacks);
  setUpAll(registerProgramFallbacks);

  late MockProgramService programService;
  late MockNutritionService nutritionService;
  late MockWorkoutRepository workoutRepo;
  late RecordingVoiceOutput voiceOut;
  late ProviderContainer container;

  Widget app(SpeechInput speech) {
    programService = MockProgramService();
    nutritionService = MockNutritionService();
    workoutRepo = MockWorkoutRepository();
    voiceOut = RecordingVoiceOutput();
    when(() => programService.getCurrent())
        .thenAnswer((_) async => sampleUserProgram());
    when(() => nutritionService.create(
          performedOn: any(named: 'performedOn'),
          proteinG: any(named: 'proteinG'),
          carbsG: any(named: 'carbsG'),
          fatG: any(named: 'fatG'),
        )).thenAnswer((_) async => sampleNutritionLog());

    final router = GoRouter(
      initialLocation: '/hub',
      routes: [
        GoRoute(path: '/hub', builder: (_, __) => const VoiceHubScreen()),
        GoRoute(
          path: '/home',
          builder: (_, __) => const Scaffold(body: Text('home-sentinel')),
        ),
        GoRoute(
          path: '/session',
          builder: (_, __) => const Scaffold(body: Text('session-sentinel')),
        ),
        GoRoute(
          path: '/programs/current',
          builder: (_, __) => const Scaffold(body: Text('program-sentinel')),
        ),
        GoRoute(
          path: '/programs/get',
          builder: (_, __) => const Scaffold(body: Text('picker-sentinel')),
        ),
        GoRoute(
          path: '/onboarding',
          builder: (_, __) => const Scaffold(body: Text('profile-sentinel')),
        ),
      ],
    );
    container = ProviderContainer(
      overrides: [
        authUserIdProvider.overrideWith((_) => 'u-test'),
        programServiceProvider.overrideWithValue(programService),
        nutritionServiceProvider.overrideWithValue(nutritionService),
        workoutRepositoryProvider.overrideWithValue(workoutRepo),
        speechInputProvider.overrideWithValue(speech),
        voiceOutputProvider.overrideWithValue(voiceOut),
      ],
    );
    addTearDown(container.dispose);
    return UncontrolledProviderScope(
      container: container,
      child: MaterialApp.router(routerConfig: router),
    );
  }

  testWidgets('ring renders all six options and the mic', (tester) async {
    await tester.pumpWidget(app(ScriptedSpeechInput([])));
    await tester.pumpAndSettle();

    for (final label in [
      'Workout',
      'Meal',
      'Program',
      'Body match',
      'History',
      'Profile',
    ]) {
      expect(find.text(label), findsOneWidget);
    }
    expect(find.byIcon(Icons.mic), findsOneWidget);
  });

  testWidgets('tapping Program navigates to /programs/current', (tester) async {
    await tester.pumpWidget(app(ScriptedSpeechInput([])));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Program'));
    await tester.pumpAndSettle();

    expect(find.text('program-sentinel'), findsOneWidget);
  });

  testWidgets('tapping Body match navigates to /programs/get', (tester) async {
    await tester.pumpWidget(app(ScriptedSpeechInput([])));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Body match'));
    await tester.pumpAndSettle();

    expect(find.text('picker-sentinel'), findsOneWidget);
  });

  testWidgets('tapping Meal opens the quick-log sheet', (tester) async {
    await tester.pumpWidget(app(ScriptedSpeechInput([])));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Meal'));
    await tester.pumpAndSettle();

    expect(find.text('Log a meal'), findsOneWidget);
    expect(find.text('Save meal'), findsOneWidget);
  });

  testWidgets('saying "show my program" speaks the summary and navigates',
      (tester) async {
    await tester.pumpWidget(app(ScriptedSpeechInput(['show my program'])));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.mic));
    await tester.pumpAndSettle();

    expect(find.text('program-sentinel'), findsOneWidget);
    expect(voiceOut.spoken.join(' '), contains('Your plan'));
  });

  testWidgets('a dictated meal with macros logs directly — no sheet',
      (tester) async {
    await tester.pumpWidget(app(ScriptedSpeechInput(
        ['log a meal with 40 grams of protein 60 carbs and 20 fat'])));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.mic));
    await tester.pumpAndSettle();

    verify(() => nutritionService.create(
          performedOn: any(named: 'performedOn'),
          proteinG: 40,
          carbsG: 60,
          fatG: 20,
        )).called(1);
    expect(find.text('Log a meal'), findsNothing); // no sheet
    expect(find.textContaining('Meal logged'), findsOneWidget);
  });

  testWidgets('"log a meal" without macros asks, then logs the follow-up',
      (tester) async {
    await tester.pumpWidget(
        app(ScriptedSpeechInput(['log a meal', '45 protein 70 carbs 25 fat'])));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.mic));
    await tester.pumpAndSettle();

    expect(voiceOut.spoken.join(' '), contains('grams of protein'));
    verify(() => nutritionService.create(
          performedOn: any(named: 'performedOn'),
          proteinG: 45,
          carbsG: 70,
          fatG: 25,
        )).called(1);
  });

  testWidgets('"start workout" starts the session and hands off to the coach',
      (tester) async {
    await tester.pumpWidget(app(ScriptedSpeechInput(['start workout'])));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.mic));
    await tester.pumpAndSettle();

    expect(find.text('session-sentinel'), findsOneWidget);
    final coach = container.read(voiceCoachProvider);
    expect(coach.enabled, isTrue);
    expect(coach.handsFree, isTrue);
    // The coach preloaded the plan into the session.
    final session = container.read(sessionDriverProvider);
    expect(session.draft!.exercises,
        hasLength(sampleUserProgram().program.highlightExercises.length));
  });

  testWidgets('unavailable speech shows the fallback hint', (tester) async {
    await tester.pumpWidget(app(ScriptedSpeechInput([], available: false)));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.mic));
    await tester.pumpAndSettle();

    expect(find.textContaining('not available'), findsOneWidget);
  });

  testWidgets('unknown dictation shows a hint and stands by', (tester) async {
    await tester
        .pumpWidget(app(ScriptedSpeechInput(['purple monkey dishwasher'])));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.mic));
    await tester.pumpAndSettle();

    expect(find.textContaining('purple monkey dishwasher'), findsOneWidget);
    expect(find.byIcon(Icons.mic), findsOneWidget); // conversation ended
  });
}
