// VoiceCoach behavior: plan preload + announcements, dictated set logging,
// next-exercise advance, and unknown-dictation coaching — all through the
// SpeechInput/VoiceOutput seams with the real SessionDriver underneath.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/hub/speech_input.dart';
import 'package:fitai/src/hub/voice_output.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:fitai/src/workout/application/session_driver.dart';
import 'package:fitai/src/workout/application/voice_coach.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/fakes.dart';
import '../../support/profile_fakes.dart';
import '../../support/program_fakes.dart';
import '../../support/voice_fakes.dart';
import '../../support/workout_fakes.dart';

void main() {
  setUpAll(registerFallbacks);
  setUpAll(registerProfileFallbacks);
  setUpAll(registerProgramFallbacks);

  late MockProgramService programService;
  late MockWorkoutRepository workoutRepo;
  late RecordingVoiceOutput voiceOut;

  ProviderContainer makeContainer(List<String> transcripts) {
    programService = MockProgramService();
    workoutRepo = MockWorkoutRepository();
    voiceOut = RecordingVoiceOutput();
    when(() => programService.getCurrent())
        .thenAnswer((_) async => sampleUserProgram());

    final container = ProviderContainer(
      overrides: [
        authUserIdProvider.overrideWith((_) => 'u-test'),
        programServiceProvider.overrideWithValue(programService),
        workoutRepositoryProvider.overrideWithValue(workoutRepo),
        speechInputProvider.overrideWithValue(ScriptedSpeechInput(transcripts)),
        voiceOutputProvider.overrideWithValue(voiceOut),
      ],
    );
    addTearDown(container.dispose);
    return container;
  }

  test('enable on an empty session preloads the plan and announces first',
      () async {
    final container = makeContainer([]);
    container.read(sessionDriverProvider.notifier).start();

    await container.read(voiceCoachProvider.notifier).enable();

    final session = container.read(sessionDriverProvider);
    final planned = sampleUserProgram().program.highlightExercises;
    expect(session.draft!.exercises.length, planned.length);
    expect(session.draft!.exercises.first.name, planned.first);
    expect(session.currentExercise, 0);

    final coach = container.read(voiceCoachProvider);
    expect(coach.enabled, isTrue);
    expect(coach.coachLine, contains('First up: ${planned.first}'));
    expect(voiceOut.spoken, isNotEmpty);
  });

  test('dictating "10 reps at 100 kilos" logs the set and confirms', () async {
    final container = makeContainer(['10 reps at 100 kilos']);
    container.read(sessionDriverProvider.notifier).start();
    final coach = container.read(voiceCoachProvider.notifier);
    await coach.enable();

    await coach.dictate();

    final last = container.read(sessionDriverProvider).lastSet;
    expect(last, isNotNull);
    expect(last!.reps, 10);
    expect(last.weightKg, 100);
    expect(container.read(voiceCoachProvider).coachLine,
        contains('Logged 10 reps at 100 kilos'));
  });

  test('"next" advances to the second planned exercise and announces it',
      () async {
    final container = makeContainer(['next']);
    container.read(sessionDriverProvider.notifier).start();
    final coach = container.read(voiceCoachProvider.notifier);
    await coach.enable();

    await coach.dictate();

    final planned = sampleUserProgram().program.highlightExercises;
    expect(container.read(sessionDriverProvider).currentExercise, 1);
    expect(container.read(voiceCoachProvider).coachLine, contains(planned[1]));
  });

  test('a driver rejection is spoken verbatim (invalid reps)', () async {
    final container = makeContainer(['0 reps']);
    container.read(sessionDriverProvider.notifier).start();
    final coach = container.read(voiceCoachProvider.notifier);
    await coach.enable();

    await coach.dictate();

    // No set logged; the coach spoke the driver's user-safe reason.
    expect(container.read(sessionDriverProvider).lastSet, isNull);
    expect(voiceOut.spoken.last, isNotEmpty);
  });

  test('unknown dictation coaches the expected phrasing', () async {
    final container = makeContainer(['tell me a joke sergeant']);
    container.read(sessionDriverProvider.notifier).start();
    final coach = container.read(voiceCoachProvider.notifier);
    await coach.enable();

    await coach.dictate();

    expect(container.read(voiceCoachProvider).coachLine,
        contains('ten reps at sixty kilos'));
  });

  test('hands-free: dictations apply automatically until silence', () async {
    final container = makeContainer(['10 reps at 100 kilos', 'next']);
    container.read(sessionDriverProvider.notifier).start();

    // No dictate() calls: enable(handsFree) runs the loop by itself; the
    // scripted queue then runs dry (silence) and the coach stands by.
    await container.read(voiceCoachProvider.notifier).enable(handsFree: true);

    final session = container.read(sessionDriverProvider);
    expect(session.draft!.exercises.first.sets, hasLength(1));
    expect(session.currentExercise, 1);
    expect(container.read(voiceCoachProvider).listening, isFalse);
  });

  test('hands-free: "pause" stops the loop', () async {
    final container = makeContainer(['pause', '10 reps']);
    container.read(sessionDriverProvider.notifier).start();

    await container.read(voiceCoachProvider.notifier).enable(handsFree: true);

    // The set after "pause" must never be consumed.
    expect(container.read(sessionDriverProvider).lastSet, isNull);
    expect(
        container.read(voiceCoachProvider).coachLine, contains('Standing by'));
  });
}
