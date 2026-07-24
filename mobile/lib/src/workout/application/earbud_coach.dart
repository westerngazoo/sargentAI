import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_tts/flutter_tts.dart';
import 'package:audio_service/audio_service.dart';
import 'package:audio_session/audio_session.dart';
import 'dart:async';
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
  StreamSubscription<AudioDevicesChangedEvent>? _deviceChangeSubscription;

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
    _listenForEarbudDisconnects();

    // Disable voice coach to ensure mutual exclusivity
    await _ref.read(voiceCoachProvider.notifier).disable();

    try {
      _audioHandler = await AudioService.init(
        builder: () =>
            AudioServiceHandler(onMediaButtonPress: handleMediaButton),
        config: const AudioServiceConfig(
          androidNotificationChannelId: 'com.fitai.channel.audio',
          androidNotificationChannelName: 'FitAI Workout',
          androidNotificationOngoing: true,
        ),
      );
      _audioHandler?.startSilentLoop().catchError((_) {});
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

  Future<void> _listenForEarbudDisconnects() async {
    try {
      final session = await AudioSession.instance;
      _deviceChangeSubscription =
          session.devicesChangedEventStream.listen((event) async {
        if (event.devicesRemoved.isNotEmpty) {
          final devices = await session.getDevices();
          final hasBluetooth = devices.any((d) =>
              d.type == AudioDeviceType.bluetoothA2dp ||
              d.type == AudioDeviceType.bluetoothSco);
          if (!hasBluetooth && _isActive) {
            // Turn off earbud coach if no bluetooth audio devices remain
            _ref.read(earbudModeProvider.notifier).state = false;
          }
        }
      });
    } catch (_) {}
  }

  Future<void> _deactivate() async {
    _isActive = false;
    _driverSubscription?.close();
    _driverSubscription = null;
    _deviceChangeSubscription?.cancel();
    _deviceChangeSubscription = null;
    try {
      await _tts.stop();
      await _audioHandler?.stop();
    } catch (_) {}
    _audioHandler = null;
  }

  // Visible for testing
  void handleMediaButton() {
    final state = _ref.read(sessionDriverProvider);
    final draft = state.draft;
    if (draft == null || state.currentExercise >= draft.exercises.length) {
      return;
    }

    final exercise = draft.exercises[state.currentExercise];
    final isLastSet = exercise.sets.length >=
        3; // Use a default of 3 sets for now (free-form logic)

    if (isLastSet) {
      // All sets logged for this exercise, advance to next exercise or finish
      if (state.currentExercise + 1 < draft.exercises.length) {
        _ref
            .read(sessionDriverProvider.notifier)
            .selectExercise(state.currentExercise + 1);
      } else {
        _ref.read(sessionDriverProvider.notifier).finish();
      }
    } else {
      // Log a set
      if (exercise.sets.isNotEmpty) {
        // Repeat the last set
        _ref.read(sessionDriverProvider.notifier).logSet(exercise.sets.last);
      } else {
        // Read text fields from the driver's UI if possible, else log empty
        final error = _ref
            .read(sessionDriverProvider.notifier)
            .logSet(const SetDraft(reps: null, weightKg: null, rpe: null));
        if (error != null) {
          // Could not log empty set (validation), so default to 1 rep
          _ref
              .read(sessionDriverProvider.notifier)
              .logSet(const SetDraft(reps: 1, weightKg: null, rpe: null));
        }
      }
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

      // Use null for sets/reps targets for now (free-form logic)
      await _speak(TtsScripts.exerciseStart(exercise.name, null, null, null));
      await _tts.awaitSpeakCompletion(true);
      if (_isActive) {
        await _speak(TtsScripts.setStart(1, null, null, null));
      }
    } else if (currentSetCount > _lastSetCount) {
      // A set was logged
      _lastSetCount = currentSetCount;
      await _speak(TtsScripts.rest);
      await _tts.awaitSpeakCompletion(true);
      if (_isActive && currentSetCount < 3) {
        await _speak(
            TtsScripts.setStart(currentSetCount + 1, null, null, null));
      }
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
