import 'dart:async';

import 'package:audio_service/audio_service.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_tts/flutter_tts.dart';

import '../application/session_driver.dart';
import '../domain/session_draft.dart';
import '../domain/set_draft.dart';
import 'voice_session_audio_handler.dart';

// Provides the singleton audio handler, initialized lazily.
final audioHandlerProvider = FutureProvider<VoiceSessionAudioHandler>((ref) async {
  final handler = await AudioService.init(
    builder: () => VoiceSessionAudioHandler(),
    config: const AudioServiceConfig(
      androidNotificationChannelId: 'com.example.fitai.voice',
      androidNotificationChannelName: 'FitAI Workout',
      androidNotificationOngoing: true,
    ),
  );
  return handler as VoiceSessionAudioHandler;
});

// A provider to allow mocking TTS in tests
final flutterTtsProvider = Provider<FlutterTts>((ref) {
  final tts = FlutterTts();
  tts.setIosAudioCategory(
    IosTextToSpeechAudioCategory.playback,
    [
      IosTextToSpeechAudioCategoryOptions.mixWithOthers,
      IosTextToSpeechAudioCategoryOptions.duckOthers,
    ],
  );
  return tts;
});

final voiceModeProvider = NotifierProvider<VoiceModeNotifier, bool>(VoiceModeNotifier.new);

class VoiceModeNotifier extends Notifier<bool> {
  @override
  bool build() => false;

  void toggle() {
    state = !state;
    if (state) {
      ref.read(voiceSessionAdapterProvider.notifier).activate();
    } else {
      ref.read(voiceSessionAdapterProvider.notifier).deactivate();
    }
  }
}

final voiceSessionAdapterProvider =
    NotifierProvider<VoiceSessionAdapter, void>(VoiceSessionAdapter.new);

class VoiceSessionAdapter extends Notifier<void> {
  ProviderSubscription? _driverSub;
  SessionDriverState? _prevState;

  @override
  void build() {}

  void activate() async {
    final tts = ref.read(flutterTtsProvider);
    final handler = await ref.read(audioHandlerProvider.future);

    handler.onMediaButtonPress = () {
      _advanceSet();
    };

    _prevState = null;
    _driverSub = ref.listen(sessionDriverProvider, (prev, next) {
      _handleStateChange(next);
    }, fireImmediately: true);
  }

  void deactivate() async {
    _driverSub?.close();
    _driverSub = null;

    final tts = ref.read(flutterTtsProvider);
    try {
      await tts.stop();
    } catch (_) {}

    try {
      final handler = await ref.read(audioHandlerProvider.future);
      handler.onMediaButtonPress = null;
    } catch (_) {}
  }

  void _advanceSet() {
    final state = ref.read(sessionDriverProvider);
    final lastSet = state.lastSet;
    if (lastSet != null) {
      ref.read(sessionDriverProvider.notifier).logSet(
        SetDraft(
          reps: lastSet.reps,
          weightKg: lastSet.weightKg,
          rpe: lastSet.rpe
        )
      );
    } else {
      // If there's no previous set logged, we can't repeat it.
      // A more robust implementation would read the target reps from the draft if available.
    }
  }

  String _fmt(double n) =>
      n == n.roundToDouble() ? n.toInt().toString() : n.toString();

  Future<void> _speak(String text) async {
    final tts = ref.read(flutterTtsProvider);
    try {
      await tts.stop();
      await tts.speak(text);
    } catch (_) {
      // Silently fail on TTS errors (engine unavailable)
    }
  }

  void _handleStateChange(SessionDriverState state) {
    if (state.done) {
      _speak("Workout done.");
      deactivate();
      ref.read(voiceModeProvider.notifier).state = false;
      return;
    }

    final draft = state.draft;
    if (draft == null || draft.exercises.isEmpty) return;

    final currExIdx = state.currentExercise;
    if (currExIdx < 0 || currExIdx >= draft.exercises.length) return;

    final exercise = draft.exercises[currExIdx];

    bool isNewExercise = false;
    bool isNewSet = false;

    if (_prevState == null) {
      isNewExercise = true;
    } else {
      final prevExIdx = _prevState!.currentExercise;
      if (currExIdx != prevExIdx) {
        isNewExercise = true;
      } else {
        final prevEx = _prevState!.draft?.exercises[prevExIdx];
        if (prevEx != null && prevEx.sets.length < exercise.sets.length) {
          isNewSet = true;
        }
      }
    }

    _prevState = state;

    if (isNewExercise) {
      // Find targets (using first set of the exercise as a rough target if available)
      int? targetReps;
      double? targetWeight;
      int numSets = exercise.sets.length; // Actually we want to announce planned sets, but currently driver only holds logged sets.
      // For R-0027, if we read from UserProgram we'd have target values.
      // But for now, we just say the exercise name since draft only has logged sets.
      // The spec says: Next: [name]. [N] sets of [R] reps at [W] kg.
      // Let's assume for free-form we just read what we can.
      _speak("Next: ${exercise.name}");
      // In a real implementation reading from UserProgram, we'd have the plan.
    } else if (isNewSet) {
      _speak("Rest.");
    }
  }
}
