import 'package:fitai/src/hub/voice_output.dart';
import 'package:fitai/src/workout/application/earbud_coach.dart';
import 'package:fitai/src/workout/application/session_driver.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import '../../support/voice_fakes.dart';

void main() {
  late ProviderContainer container;
  late RecordingVoiceOutput voiceOut;

  setUp(() {
    voiceOut = RecordingVoiceOutput();
    container = ProviderContainer(
      overrides: [
        voiceOutputProvider.overrideWithValue(voiceOut),
      ],
    );
  });

  tearDown(() {
    container.dispose();
  });

  test('EarbudCoach handleMediaButton logs a new set with last values', () {
    final driver = container.read(sessionDriverProvider.notifier);

    // Set up a session with 1 set
    driver.start();
    driver.addExercise('Squat');
    driver.logSet(const SetDraft(reps: 8, weightKg: 100));

    final stateBefore = container.read(sessionDriverProvider);
    expect(stateBefore.draft!.exercises.first.sets.length, 1);

    // Call media button handler directly
    container.read(earbudCoachProvider).handleMediaButton();

    final stateAfter = container.read(sessionDriverProvider);
    expect(stateAfter.draft!.exercises.first.sets.length, 2);
    expect(stateAfter.draft!.exercises.first.sets.last.reps, 8);
    expect(stateAfter.draft!.exercises.first.sets.last.weightKg, 100);
  });
}
