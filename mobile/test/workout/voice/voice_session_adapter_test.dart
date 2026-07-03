import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:fitai/src/workout/application/session_driver.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:fitai/src/workout/voice/voice_session_adapter.dart';
import 'package:fitai/src/workout/voice/voice_session_audio_handler.dart';
import 'package:flutter_tts/flutter_tts.dart';

class MockFlutterTts extends Mock implements FlutterTts {}

class MockVoiceSessionAudioHandler extends Mock
    implements VoiceSessionAudioHandler {}

void main() {
  late MockFlutterTts mockTts;
  late MockVoiceSessionAudioHandler mockAudioHandler;
  late ProviderContainer container;

  setUpAll(() {
    registerFallbackValue(IosTextToSpeechAudioCategory.playback);
  });

  setUp(() {
    mockTts = MockFlutterTts();
    mockAudioHandler = MockVoiceSessionAudioHandler();
    when(() => mockTts.speak(any())).thenAnswer((_) async => 1);
    when(() => mockTts.stop()).thenAnswer((_) async => 1);
    when(() => mockTts.setIosAudioCategory(any(), any()))
        .thenAnswer((_) async => 1);

    container = ProviderContainer(overrides: [
      flutterTtsProvider.overrideWithValue(mockTts),
      audioHandlerProvider
          .overrideWith((ref) => Future.value(mockAudioHandler)),
    ]);
  });

  tearDown(() {
    container.dispose();
  });

  test('toggle voice mode activates adapter and hooks button press', () async {
    container.read(voiceModeProvider.notifier).toggle();

    await Future.delayed(Duration.zero);

    verify(() => mockAudioHandler.onMediaButtonPress = any()).called(1);

    container.read(voiceModeProvider.notifier).toggle();

    verify(() => mockTts.stop()).called(1);
  });

  test('reads exercise name on new exercise', () async {
    container.read(voiceModeProvider.notifier).toggle();
    await Future.delayed(Duration.zero);

    container.read(sessionDriverProvider.notifier).start();
    container.read(sessionDriverProvider.notifier).addExercise('Squat');

    await Future.delayed(Duration.zero);

    verify(() => mockTts.speak('Next: Squat')).called(1);
  });

  test('cues rest when new set is added', () async {
    container.read(voiceModeProvider.notifier).toggle();
    await Future.delayed(Duration.zero);

    container.read(sessionDriverProvider.notifier).start();
    container.read(sessionDriverProvider.notifier).addExercise('Squat');

    clearInteractions(mockTts);

    container
        .read(sessionDriverProvider.notifier)
        .logSet(const SetDraft(reps: 10, weightKg: 100));

    await Future.delayed(Duration.zero);

    verify(() => mockTts.speak('Rest.')).called(1);
  });
}
