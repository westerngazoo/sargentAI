// The voice coach (R-0032 slice, feeding R-0027) — a notifier gluing
// [SpeechInput] (dictation in), [VoiceOutput] (sergeant voice out), and the
// widget-independent [SessionDriver]. When enabled on an empty session it
// preloads the active program's highlight exercises, so "what's next" always
// comes from the user's plan.

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../hub/speech_input.dart';
import '../../hub/voice_protocol.dart';
import '../../hub/voice_output.dart';
import '../../program/application/program_providers.dart';
import '../domain/set_draft.dart';
import 'session_driver.dart';
import 'session_voice_intent.dart';

@immutable
class VoiceCoachState {
  const VoiceCoachState({
    this.enabled = false,
    this.handsFree = false,
    this.listening = false,
    this.transcript = '',
    this.coachLine = '',
  });

  final bool enabled;

  /// Hands-free: the coach re-listens automatically after every response.
  final bool handsFree;
  final bool listening;

  /// The user's last (or in-flight) dictation.
  final String transcript;

  /// The last thing the coach said — also rendered for screen-on use.
  final String coachLine;

  VoiceCoachState copyWith({
    bool? enabled,
    bool? handsFree,
    bool? listening,
    String? transcript,
    String? coachLine,
  }) =>
      VoiceCoachState(
        enabled: enabled ?? this.enabled,
        handsFree: handsFree ?? this.handsFree,
        listening: listening ?? this.listening,
        transcript: transcript ?? this.transcript,
        coachLine: coachLine ?? this.coachLine,
      );
}

final voiceCoachProvider =
    NotifierProvider<VoiceCoach, VoiceCoachState>(VoiceCoach.new);

class VoiceCoach extends Notifier<VoiceCoachState> {
  /// Consecutive silent listens tolerated before standing by (hands-free).
  static const _maxSilences = 3;
  int _silences = 0;

  @override
  VoiceCoachState build() => const VoiceCoachState();

  SessionDriver get _driver => ref.read(sessionDriverProvider.notifier);
  SessionDriverState get _session => ref.read(sessionDriverProvider);

  /// Turns the coach on: initializes TTS, preloads the active program's
  /// exercises into an empty session, and announces what's first. With
  /// [handsFree] the coach re-listens automatically after every response —
  /// the whole session runs by voice.
  Future<void> enable({bool handsFree = false}) async {
    await ref.read(voiceOutputProvider).initialize();
    state = state.copyWith(enabled: true, handsFree: handsFree);

    final exercises = _session.draft?.exercises ?? const [];
    final prompt =
        handsFree ? 'Tell me your set.' : 'Tap the mic and tell me your set.';
    if (exercises.isEmpty) {
      final planned = await _loadPlan();
      if (planned.isNotEmpty) {
        for (final name in planned) {
          _driver.addExercise(name);
        }
        _driver.selectExercise(0);
        await _say('Plan loaded — ${planned.length} exercises. First up: '
            '${planned.first}. $prompt');
      } else {
        await _say('Voice coach on. No plan found — add an exercise, '
            'then dictate your sets.');
      }
    } else {
      await _say(
          'Voice coach on. Current exercise: ${_currentName()}. $prompt');
    }
    if (handsFree) await _listenLoop();
  }

  /// Turns the coach off, stopping any speech in either direction.
  Future<void> disable() async {
    await ref.read(speechInputProvider).stop();
    await ref.read(voiceOutputProvider).stop();
    state = const VoiceCoachState();
  }

  /// One dictation round: listen → parse → apply to the driver → speak the
  /// outcome. Driver rejection strings are spoken verbatim (they are the
  /// user-safe reasons by contract). In hands-free mode the loop re-arms
  /// itself after every response until paused, finished, or silent too long.
  Future<void> dictate() async {
    final speech = ref.read(speechInputProvider);
    if (state.listening) {
      await speech.stop();
      state = state.copyWith(listening: false);
      return;
    }
    _silences = 0;
    await _listenOnce();
  }

  Future<void> _listenLoop() async {
    _silences = 0;
    await _listenOnce();
  }

  Future<void> _listenOnce() async {
    final speech = ref.read(speechInputProvider);
    if (!await speech.initialize()) {
      await _say('Voice input is not available here.');
      return;
    }
    state = state.copyWith(listening: true, transcript: '');
    var handled = false;
    await speech.listen((transcript, isFinal) async {
      if (handled) return;
      state = state.copyWith(transcript: transcript);
      // "over" terminates the command instantly — no silence timeout.
      final over = endsWithOver(transcript);
      if (!isFinal && !over) return;
      handled = true;
      if (over && !isFinal) await speech.stop();
      state = state.copyWith(listening: false);
      final command = stripOver(transcript);
      if (command.isEmpty) {
        // Silence: in hands-free, quietly re-arm a few times, then stand by
        // without speaking (mid-rest chatter would be worse than silence).
        _silences += 1;
        if (state.handsFree && state.enabled && _silences < _maxSilences) {
          await _listenOnce();
        }
        return;
      }
      _silences = 0;
      final keepListening = await _apply(parseSessionVoiceIntent(command));
      if (keepListening && state.handsFree && state.enabled) {
        await _listenOnce();
      }
    });
  }

  /// Applies one intent; returns whether the hands-free loop should re-arm.
  Future<bool> _apply(SessionVoiceIntent intent) async {
    switch (intent) {
      case LogSetIntent(:final reps, :final weightKg, :final rpe):
        final rejection =
            _driver.logSet(SetDraft(reps: reps, weightKg: weightKg, rpe: rpe));
        if (rejection != null) {
          await _say(rejection);
          return true;
        }
        final sets = _session.lastSet;
        final logged = [
          if (sets?.reps != null) '${sets!.reps} reps',
          if (sets?.weightKg != null) 'at ${_trim(sets!.weightKg!)} kilos',
        ].join(' ');
        await _say('Logged $logged. Rest up.');
        return true;
      case NextExerciseIntent():
        final i = _session.currentExercise;
        final exercises = _session.draft?.exercises ?? const [];
        if (i + 1 < exercises.length) {
          _driver.selectExercise(i + 1);
          await _say('Next up: ${exercises[i + 1].name}.');
        } else {
          await _say('That was the last exercise. '
              'Say finish workout to save.');
        }
        return true;
      case FinishSessionIntent():
        await _say('Saving your workout.');
        await _driver.finish();
        final after = _session;
        if (after.done) {
          await _say('Workout saved. Dismissed!');
        } else if (after.error != null) {
          await _say(after.error!);
          return true;
        }
        return false;
      case PauseSessionIntent():
        await _say('Standing by. Tap the mic when you are ready.');
        return false;
      case UnknownSessionIntent():
        await _say('Did not catch that. Say for example: '
            'ten reps at sixty kilos.');
        return true;
    }
  }

  Future<List<String>> _loadPlan() async {
    try {
      final program = await ref.read(currentProgramProvider.future);
      return program?.program.highlightExercises ?? const [];
    } catch (_) {
      return const [];
    }
  }

  String _currentName() {
    final exercises = _session.draft?.exercises ?? const [];
    final i = _session.currentExercise;
    return (i >= 0 && i < exercises.length) ? exercises[i].name : 'none';
  }

  Future<void> _say(String line) async {
    state = state.copyWith(coachLine: line);
    await ref.read(voiceOutputProvider).speak(line);
  }
}

String _trim(double v) => v == v.roundToDouble() ? v.toStringAsFixed(0) : '$v';
