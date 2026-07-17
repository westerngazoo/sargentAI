import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_tts/flutter_tts.dart';
import 'package:audio_service/audio_service.dart';
import 'session_driver.dart';
import '../../audio/tts_scripts.dart';
import 'audio_service_handler.dart';
import '../domain/set_draft.dart';
import 'voice_coach.dart';

final earbudModeProvider = StateProvider<bool>((ref) => false);

final earbudCoachProvider = Provider<EarbudCoach>((ref) {
  final coach = EarbudCoach(ref);
  ref.onDispose(() => coach.dispose());
  return coach;
});

class EarbudCoach {
  final Ref _ref;
  final FlutterTts _tts = FlutterTts();
  AudioServiceHandler? _audioHandler;
  ProviderSubscription? _driverSubscription;

  int _lastExerciseIndex = -1;
  int _lastSetCount = 0;
  bool _isActive = false;

  EarbudCoach(this._ref) {
    _initTts();
    _ref.listen<bool>(earbudModeProvider, (previous, current) {
      if (current && !_isActive) {
        _activate();
      } else if (!current && _isActive) {
        _deactivate();
      }
    }, fireImmediately: true);
  }

  Future<void> _initTts() async {
    try {
      await _tts.setSharedInstance(true);
      await _tts.setIosAudioCategory(IosTextToSpeechAudioCategory.playback, [
        IosTextToSpeechAudioCategoryOptions.allowBluetooth,
        IosTextToSpeechAudioCategoryOptions.allowBluetoothA2DP,
        IosTextToSpeechAudioCategoryOptions.mixWithOthers
      ]);
    } catch (_) {}
  }

  Future<void> _activate() async {
    _isActive = true;

    // Disable voice coach to ensure mutual exclusivity
    await _ref.read(voiceCoachProvider.notifier).disable();

    try {
      _audioHandler = await AudioService.init(
        builder: () =>
            AudioServiceHandler(onMediaButtonPress: _handleMediaButton),
        config: const AudioServiceConfig(
          androidNotificationChannelId: 'com.example.fitai.channel.audio',
          androidNotificationChannelName: 'FitAI Workout',
          androidNotificationOngoing: true,
        ),
      );
      await _audioHandler?.startSilentLoop();
    } catch (_) {}

    final currentState = _ref.read(sessionDriverProvider);
    _lastExerciseIndex = currentState.currentExercise;
    final draft = currentState.draft;
    _lastSetCount =
        (draft != null && currentState.currentExercise < draft.exercises.length)
            ? draft.exercises[currentState.currentExercise].sets.length
            : 0;

    _driverSubscription = _ref.listen<SessionDriverState>(
      sessionDriverProvider,
      _onSessionStateChanged,
      fireImmediately: true, // Announce current state immediately
    );
  }

  Future<void> _deactivate() async {
    _isActive = false;
    _driverSubscription?.close();
    _driverSubscription = null;
    try {
      await _tts.stop();
      await _audioHandler?.stop();
    } catch (_) {}
    _audioHandler = null;
  }

  void _handleMediaButton() {
    final state = _ref.read(sessionDriverProvider);
    final draft = state.draft;
    if (draft == null || state.currentExercise >= draft.exercises.length) {
      return;
    }

    final exercise = draft.exercises[state.currentExercise];
    if (exercise.sets.isNotEmpty) {
      // Repeat the last set
      _ref.read(sessionDriverProvider.notifier).logSet(exercise.sets.last);
    } else {
      _ref
          .read(sessionDriverProvider.notifier)
          .logSet(const SetDraft(reps: null, weightKg: null, rpe: null));
    }
  }

  void _onSessionStateChanged(
      SessionDriverState? previous, SessionDriverState current) async {
    if (current.done) {
      await _speak(TtsScripts.workoutDone);
      return;
    }

    final draft = current.draft;
    if (draft == null) return;
    if (current.currentExercise >= draft.exercises.length) return;

    final exercise = draft.exercises[current.currentExercise];
    final currentSetCount = exercise.sets.length;

    if (current.currentExercise != _lastExerciseIndex) {
      // Transition to a new exercise
      _lastExerciseIndex = current.currentExercise;
      _lastSetCount = currentSetCount;

      await _speak(TtsScripts.exerciseStart(exercise.name, 3, 10, null));
    } else if (currentSetCount > _lastSetCount) {
      // A set was logged
      _lastSetCount = currentSetCount;
      await _speak(TtsScripts.rest);
    }
  }

  Future<void> _speak(String text) async {
    try {
      await _tts.stop(); // Interrupt any ongoing speech
      await _tts.speak(text);
    } catch (e) {
      // Silently continue on TTS failure as per spec
    }
  }

  void dispose() {
    _deactivate();
  }
}
