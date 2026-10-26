// R-0032 -> R-0061 AC2, SPEC-0061 §2.8 (slice 3b): the Dictado sheet over the
// hands-free VoiceLoop. The ring is gone; the sheet holds the mic and the
// chat thread, and resolves with the typed VoiceEffect the loop emitted.
//
// Covers: dictation runs the conversation (a meal logged by voice alone, the
// macro follow-up, the program summary, the workout handoff left to the
// screen); every local intent closes the sheet with its effect; the server's
// prose is never shown or spoken; speech unavailable and unknown transcripts
// degrade gracefully; closing by hand stops the loop.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/hub/dictation_sheet.dart';
import 'package:fitai/src/hub/speech_input.dart';
import 'package:fitai/src/hub/voice_effect.dart';
import 'package:fitai/src/hub/voice_intent_service.dart';
import 'package:fitai/src/hub/voice_loop.dart';
import 'package:fitai/src/hub/voice_output.dart';
import 'package:fitai/src/nutrition/services/nutrition_service.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:fitai/src/workout/application/session_driver.dart';
import 'package:fitai/src/workout/application/voice_coach.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/l10n.dart';
import '../support/profile_fakes.dart';
import '../support/program_fakes.dart';
import '../support/voice_fakes.dart';
import '../support/workout_fakes.dart';

/// A page with one button that opens the sheet and records what it resolved
/// with: the loop's effect, or null when closed by hand.
class _Host extends StatelessWidget {
  const _Host(this.results);

  final List<VoiceEffect?> results;

  @override
  Widget build(BuildContext context) => Scaffold(
        body: Center(
          child: FilledButton(
            key: const Key('open'),
            onPressed: () async =>
                results.add(await showDictationSheet(context)),
            child: const Text('open'),
          ),
        ),
      );
}

/// Listens forever: the conversation stays open until something closes it.
class _SilentSpeechInput implements SpeechInput {
  bool stopped = false;

  @override
  Future<bool> initialize() async => true;

  @override
  Future<void> listen(OnTranscript onTranscript) async {}

  @override
  Future<void> stop() async => stopped = true;
}

void main() {
  setUpAll(registerFallbacks);
  setUpAll(registerProfileFallbacks);
  setUpAll(registerProgramFallbacks);

  late MockProgramService programService;
  late MockNutritionService nutritionService;
  late MockWorkoutRepository workoutRepo;
  late MockVoiceIntentService voiceIntent;
  late RecordingVoiceOutput voiceOut;
  late ProviderContainer container;
  late List<VoiceEffect?> results;

  Widget app(SpeechInput speech) {
    programService = MockProgramService();
    nutritionService = MockNutritionService();
    workoutRepo = MockWorkoutRepository();
    voiceIntent = MockVoiceIntentService();
    voiceOut = RecordingVoiceOutput();
    results = [];
    when(() => programService.getCurrent())
        .thenAnswer((_) async => sampleUserProgram());
    when(() => nutritionService.create(
          performedOn: any(named: 'performedOn'),
          proteinG: any(named: 'proteinG'),
          carbsG: any(named: 'carbsG'),
          fatG: any(named: 'fatG'),
        )).thenAnswer((_) async => sampleNutritionLog());
    when(() => nutritionService.searchFoods(any()))
        .thenAnswer((_) async => const []);
    // Offline by default: the local parser carries the conversation.
    when(() => voiceIntent.parse(any())).thenThrow(Exception('offline'));

    container = ProviderContainer(
      overrides: [
        authUserIdProvider.overrideWith((_) => 'u-test'),
        programServiceProvider.overrideWithValue(programService),
        nutritionServiceProvider.overrideWithValue(nutritionService),
        workoutRepositoryProvider.overrideWithValue(workoutRepo),
        voiceIntentServiceProvider.overrideWithValue(voiceIntent),
        speechInputProvider.overrideWithValue(speech),
        voiceOutputProvider.overrideWithValue(voiceOut),
        l10nOverride,
      ],
    );
    addTearDown(container.dispose);
    return UncontrolledProviderScope(
      container: container,
      child: localizedApp(home: _Host(results)),
    );
  }

  Future<void> open(WidgetTester tester, SpeechInput speech) async {
    await tester.pumpWidget(app(speech));
    await tester.tap(find.byKey(const Key('open')));
    await tester.pumpAndSettle();
  }

  Future<void> dictate(WidgetTester tester) async {
    await tester.tap(find.byKey(const Key('dictation-mic')));
    await tester.pumpAndSettle();
  }

  final sheetOpen = find.byKey(const Key('panel-close'));

  testWidgets('the sheet opens with its title, the hint and the mic',
      (tester) async {
    await open(tester, ScriptedSpeechInput([]));

    expect(find.byType(DictationSheet), findsOneWidget);
    expect(find.text(l10n.dictationTitle), findsOneWidget);
    expect(find.text(l10n.hubIdleHint), findsOneWidget);
    expect(find.byKey(const Key('dictation-mic')), findsOneWidget);
    expect(sheetOpen, findsOneWidget);
  });

  testWidgets('"show my program" speaks the summary and opens Programa',
      (tester) async {
    await open(tester, ScriptedSpeechInput(['show my program']));
    await dictate(tester);

    final program = sampleUserProgram();
    expect(
        voiceOut.spoken.join(' '),
        contains(l10n.sergeantPlanSummary(program.program.split,
            program.program.daysPerWeek, program.diet.estimatedKcal)));
    expect(results, [const OpenPanel(PanelKind.programa)]);
    expect(sheetOpen, findsNothing);
  });

  testWidgets('with no program, "show my program" still opens Programa',
      (tester) async {
    await open(tester, ScriptedSpeechInput(['show my program']));
    when(() => programService.getCurrent()).thenAnswer((_) async => null);
    await dictate(tester);

    expect(voiceOut.spoken.join(' '), contains(l10n.sergeantNoPlanYet));
    expect(results, [const OpenPanel(PanelKind.programa)]);
  });

  testWidgets('"body match" opens Programa, where the choose flow lives',
      (tester) async {
    await open(tester, ScriptedSpeechInput(['find my body type']));
    await dictate(tester);

    expect(voiceOut.spoken.join(' '), contains(l10n.sergeantOpeningBodyMatch));
    expect(results, [const OpenPanel(PanelKind.programa)]);
  });

  testWidgets('"my profile" opens onboarding', (tester) async {
    await open(tester, ScriptedSpeechInput(['open my profile']));
    await dictate(tester);

    expect(voiceOut.spoken.join(' '), contains(l10n.sergeantOpeningProfile));
    expect(results, [const OpenOnboarding()]);
  });

  testWidgets('"history" closes: the calendar is behind the sheet',
      (tester) async {
    await open(tester, ScriptedSpeechInput(['show my history']));
    await dictate(tester);

    expect(voiceOut.spoken.join(' '), contains(l10n.sergeantRecentActivity));
    expect(results, [const Close()]);
    expect(sheetOpen, findsNothing);
  });

  testWidgets('a dictated meal with macros logs directly, the sheet stays',
      (tester) async {
    await open(
        tester,
        ScriptedSpeechInput(
            ['log a meal with 40 grams of protein 60 carbs and 20 fat']));
    await dictate(tester);

    verify(() => nutritionService.create(
          performedOn: any(named: 'performedOn'),
          proteinG: 40,
          carbsG: 60,
          fatG: 20,
        )).called(1);
    expect(find.textContaining(l10n.sergeantMealLogged), findsOneWidget);
    expect(sheetOpen, findsOneWidget);
    expect(results, isEmpty);
  });

  testWidgets('"log a meal" without macros asks, then logs the follow-up',
      (tester) async {
    await open(tester,
        ScriptedSpeechInput(['log a meal', '45 protein 70 carbs 25 fat']));
    await dictate(tester);

    expect(voiceOut.spoken.join(' '), contains(l10n.sergeantAskMacros));
    verify(() => nutritionService.create(
          performedOn: any(named: 'performedOn'),
          proteinG: 45,
          carbsG: 70,
          fatG: 25,
        )).called(1);
  });

  testWidgets(
      '"start workout" closes with focus mode and leaves the session to '
      'the screen', (tester) async {
    await open(tester, ScriptedSpeechInput(['start workout']));
    await dictate(tester);

    expect(voiceOut.spoken.join(' '), contains(l10n.sergeantStartingSession));
    expect(results, [const OpenFocusMode()]);
    expect(sheetOpen, findsNothing);
    // The screen builds today's PlanSeed and starts the driver (§2.8); the
    // loop touches neither the driver nor the coach.
    expect(container.read(sessionDriverProvider).draft, isNull);
    expect(container.read(voiceCoachProvider).enabled, isFalse);
  });

  testWidgets('unavailable speech shows the fallback hint', (tester) async {
    await open(tester, ScriptedSpeechInput([], available: false));
    await dictate(tester);

    expect(find.text(l10n.sergeantVoiceUnavailable), findsOneWidget);
    expect(sheetOpen, findsOneWidget);
  });

  testWidgets('unknown dictation shows a hint and stands by, the sheet open',
      (tester) async {
    await open(tester, ScriptedSpeechInput(['purple monkey dishwasher']));
    await dictate(tester);

    // Visible both as the user's bubble and in the loop's echo.
    expect(find.textContaining('purple monkey dishwasher'),
        findsAtLeastNWidgets(1));
    expect(container.read(voiceLoopProvider).conversing, isFalse);
    expect(sheetOpen, findsOneWidget);
    expect(results, isEmpty);
  });

  testWidgets('portion dictation looks up nutrients and logs the meal',
      (tester) async {
    await open(tester,
        ScriptedSpeechInput(['log a meal 200 grams of chicken breast over']));
    when(() => nutritionService.searchFoods('chicken breast'))
        .thenAnswer((_) async => [sampleFoodInfo()]);
    await dictate(tester);

    // 200 g of 31/0/3.5 per 100 g → 62 protein, 0 carbs, 7 fat.
    verify(() => nutritionService.create(
          performedOn: any(named: 'performedOn'),
          proteinG: 62,
          carbsG: 0,
          fatG: 7,
        )).called(1);
    expect(voiceOut.spoken.join(' '),
        contains(l10n.sergeantLookingUp('chicken breast')));
  });

  testWidgets('"over and out" signs off and closes the sheet', (tester) async {
    await open(tester, ScriptedSpeechInput(['over and out']));
    await dictate(tester);

    expect(voiceOut.spoken.join(' '), contains(l10n.sergeantRogerOut));
    expect(results, [const Close()]);
    expect(container.read(voiceLoopProvider).conversing, isFalse);
  });

  testWidgets('a preset meal by voice logs without lookup or macros',
      (tester) async {
    await open(tester, ScriptedSpeechInput(['log a protein shake over']));
    await dictate(tester);

    verify(() => nutritionService.create(
          performedOn: any(named: 'performedOn'),
          proteinG: 30,
          carbsG: 8,
          fatG: 3,
        )).called(1);
    verifyNever(() => nutritionService.searchFoods(any()));
    expect(find.textContaining('Protein shake'), findsAtLeastNWidgets(1));
  });

  testWidgets('closing the sheet by hand stops the loop', (tester) async {
    final speech = _SilentSpeechInput();
    await open(tester, speech);
    await dictate(tester);
    expect(container.read(voiceLoopProvider).listening, isTrue);

    await tester.tap(sheetOpen);
    await tester.pumpAndSettle();

    expect(sheetOpen, findsNothing);
    expect(speech.stopped, isTrue);
    expect(container.read(voiceLoopProvider), const VoiceLoopState());
    expect(results, [null]);
  });

  group('the server answers', () {
    testWidgets('its prose is never shown or spoken; the effect is acted on',
        (tester) async {
      await open(tester, ScriptedSpeechInput(['start workout']));
      const prose = 'Starting your session.';
      when(() => voiceIntent.parse(any())).thenAnswer((_) async =>
          const VoiceIntentResult(
              status: 'navigate', message: prose, effect: OpenFocusMode()));
      await dictate(tester);

      expect(voiceOut.spoken.join(' '), isNot(contains(prose)));
      expect(voiceOut.spoken.join(' '), contains(l10n.sergeantStartingSession));
      expect(find.text(prose), findsNothing);
      expect(results, [const OpenFocusMode()]);
    });

    testWidgets('a clarify asks in the client\'s words, then logs',
        (tester) async {
      await open(tester,
          ScriptedSpeechInput(['log a meal', '45 protein 70 carbs 25 fat']));
      const prose = 'How many grams of protein?';
      when(() => voiceIntent.parse(any())).thenAnswer((_) async =>
          const VoiceIntentResult(status: 'clarify', prompt: prose));
      await dictate(tester);

      expect(voiceOut.spoken.join(' '), isNot(contains(prose)));
      expect(voiceOut.spoken.join(' '), contains(l10n.sergeantAskMacros));
      verify(() => nutritionService.create(
            performedOn: any(named: 'performedOn'),
            proteinG: 45,
            carbsG: 70,
            fatG: 25,
          )).called(1);
    });

    testWidgets('a logged answer is acknowledged in the client\'s words',
        (tester) async {
      await open(tester, ScriptedSpeechInput(['bench 100 for 5']));
      const prose = 'Logged bench press.';
      when(() => voiceIntent.parse(any())).thenAnswer((_) async =>
          const VoiceIntentResult(status: 'logged_workout', message: prose));
      await dictate(tester);

      expect(voiceOut.spoken.join(' '), isNot(contains(prose)));
      expect(voiceOut.spoken.join(' '), contains(l10n.sergeantLogged));
      expect(find.text(prose), findsNothing);
    });
  });
}
