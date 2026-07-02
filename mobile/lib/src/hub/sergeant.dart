// The hands-free wizard (R-0032): a conversation loop over the hub. The
// sergeant prompts, listens, acts, and re-listens — meals log by voice alone,
// "start workout" hands off to the [VoiceCoach]'s own hands-free loop, and
// navigation is surfaced as a state effect the screen consumes (notifiers
// hold no BuildContext).

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../nutrition/services/nutrition_service.dart';
import '../program/application/program_providers.dart';
import '../workout/application/session_driver.dart';
import '../workout/application/voice_coach.dart';
import '../nutrition/models/food_info.dart';
import 'speech_input.dart';
import 'voice_intent.dart';
import 'voice_protocol.dart';
import 'voice_output.dart';

@immutable
class SergeantState {
  const SergeantState({
    this.conversing = false,
    this.listening = false,
    this.transcript = '',
    this.line = '',
    this.awaitingMacros = false,
    this.navigateTo,
  });

  final bool conversing;
  final bool listening;

  /// The user's last (or in-flight) dictation.
  final String transcript;

  /// The sergeant's last spoken line — also rendered in the hub pill.
  final String line;

  /// A meal was requested without macros; the next dictation answers them.
  final bool awaitingMacros;

  /// Navigation effect for the screen to consume (notifiers have no context).
  final String? navigateTo;

  SergeantState copyWith({
    bool? conversing,
    bool? listening,
    String? transcript,
    String? line,
    bool? awaitingMacros,
    String? navigateTo,
    bool clearNavigation = false,
  }) =>
      SergeantState(
        conversing: conversing ?? this.conversing,
        listening: listening ?? this.listening,
        transcript: transcript ?? this.transcript,
        line: line ?? this.line,
        awaitingMacros: awaitingMacros ?? this.awaitingMacros,
        navigateTo: clearNavigation ? null : (navigateTo ?? this.navigateTo),
      );
}

final sergeantProvider =
    NotifierProvider<Sergeant, SergeantState>(Sergeant.new);

class Sergeant extends Notifier<SergeantState> {
  /// Fruitless rounds (silence or unknown) tolerated before standing by.
  static const _maxIdleRounds = 3;
  int _idleRounds = 0;

  @override
  SergeantState build() => const SergeantState();

  /// Opens the conversation: prompt → listen → act → re-listen.
  Future<void> start() async {
    final speech = ref.read(speechInputProvider);
    await ref.read(voiceOutputProvider).initialize();
    if (!await speech.initialize()) {
      state = state.copyWith(
          line: 'Voice input is not available here — tap an option instead.');
      return;
    }
    _idleRounds = 0;
    state = state.copyWith(conversing: true, awaitingMacros: false);
    await _say('Sergeant here. Say: start workout, plan workout, '
        'log a meal. Finish every command with over.');
    await _listenOnce();
  }

  /// Ends the conversation and stops both audio directions.
  Future<void> stop() async {
    await ref.read(speechInputProvider).stop();
    await ref.read(voiceOutputProvider).stop();
    state = const SergeantState();
  }

  /// The screen calls this after performing [SergeantState.navigateTo].
  void consumeNavigation() => state = state.copyWith(clearNavigation: true);

  Future<void> _listenOnce() async {
    final speech = ref.read(speechInputProvider);
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
      if (!state.conversing) return;
      final command = stripOver(transcript);
      if (command.isEmpty) {
        await _idleRound();
        return;
      }
      final keepListening = await _handle(command);
      if (keepListening && state.conversing) await _listenOnce();
    });
  }

  /// Silence/unknown loop guard: quietly re-arm a few times, then stand by
  /// without overwriting the last useful line.
  Future<void> _idleRound() async {
    _idleRounds += 1;
    if (_idleRounds < _maxIdleRounds && state.conversing) {
      await _listenOnce();
    } else {
      state = state.copyWith(conversing: false, listening: false);
    }
  }

  Future<bool> _handle(String transcript) async {
    if (isOut(transcript)) {
      await _say('Roger. Out.');
      state = state.copyWith(conversing: false);
      return false;
    }
    if (state.awaitingMacros) return _handleMacros(transcript);

    switch (parseVoiceIntent(transcript)) {
      case StopIntent():
        await _say('Standing by.');
        state = state.copyWith(conversing: false);
        return false;
      case LogMealIntent(:final proteinG, :final carbsG, :final fatG):
        _idleRounds = 0;
        if (proteinG != null && carbsG != null && fatG != null) {
          return _logMeal(proteinG, carbsG, fatG);
        }
        final portion = parseFoodQuantity(transcript);
        if (portion != null) return _logFood(portion.food, portion.grams);
        state = state.copyWith(awaitingMacros: true);
        await _say('Tell me the grams of protein, carbs, and fat — '
            'or say a portion, like 200 grams of chicken breast.');
        return true;
      case LogWorkoutIntent():
        await _say('Starting your session.');
        ref.read(sessionDriverProvider.notifier).start();
        state = state.copyWith(conversing: false, navigateTo: '/session');
        // Fire-and-forget: the coach announces the plan and runs its own
        // hands-free loop on the session screen.
        ref.read(voiceCoachProvider.notifier).enable(handsFree: true);
        return false;
      case ShowProgramIntent():
        final summary = await _programSummary();
        if (summary != null) {
          await _say(summary);
          state = state.copyWith(
              conversing: false, navigateTo: '/programs/current');
        } else {
          await _say('No plan yet — let us find your body match.');
          state =
              state.copyWith(conversing: false, navigateTo: '/programs/get');
        }
        return false;
      case BodyMatchIntent():
        await _say('Opening the body match.');
        state = state.copyWith(conversing: false, navigateTo: '/programs/get');
        return false;
      case ShowHistoryIntent():
        await _say('Your recent activity.');
        state = state.copyWith(conversing: false, navigateTo: '/home');
        return false;
      case ShowProfileIntent():
        await _say('Opening your profile.');
        state = state.copyWith(conversing: false, navigateTo: '/onboarding');
        return false;
      case UnknownIntent(:final transcript):
        _idleRounds += 1;
        if (_idleRounds >= _maxIdleRounds) {
          state = state.copyWith(conversing: false);
          return false;
        }
        await _say(transcript.isEmpty
            ? 'Say: start workout, plan workout, or log a meal.'
            : 'Did not get "$transcript" — say start workout, '
                'plan workout, or log a meal.');
        return true;
    }
  }

  Future<bool> _handleMacros(String transcript) async {
    final portion = parseFoodQuantity(transcript);
    if (portion != null) {
      state = state.copyWith(awaitingMacros: false);
      return _logFood(portion.food, portion.grams);
    }
    final macros = parseMacros(transcript);
    if (macros == null) {
      _idleRounds += 1;
      if (_idleRounds >= _maxIdleRounds) {
        state = state.copyWith(conversing: false, awaitingMacros: false);
        return false;
      }
      await _say('Grams of protein, carbs, and fat — for example: '
          '40 protein, 60 carbs, 20 fat.');
      return true;
    }
    state = state.copyWith(awaitingMacros: false);
    return _logMeal(macros.proteinG ?? 0, macros.carbsG ?? 0, macros.fatG ?? 0);
  }

  Future<bool> _logMeal(double p, double c, double f, {String? label}) async {
    try {
      final today = DateTime.now();
      final iso = '${today.year.toString().padLeft(4, '0')}-'
          '${today.month.toString().padLeft(2, '0')}-'
          '${today.day.toString().padLeft(2, '0')}';
      final log = await ref
          .read(nutritionServiceProvider)
          .create(performedOn: iso, proteinG: p, carbsG: c, fatG: f);
      final what = label == null ? 'Meal logged' : 'Logged $label';
      await _say('$what: ${_g(p)} protein, ${_g(c)} carbs, '
          '${_g(f)} fat — ${log.calories.round()} calories. What else?');
      return true;
    } catch (_) {
      await _say('Could not save the meal — try again in a moment.');
      return true;
    }
  }

  /// Nutrient lookup: "`<grams>` grams of `<food>`" → USDA macros → meal.
  Future<bool> _logFood(String food, double grams) async {
    await _say('Looking up $food.');
    List<FoodInfo> matches;
    try {
      matches = await ref.read(nutritionServiceProvider).searchFoods(food);
    } catch (_) {
      matches = const [];
    }
    final match = matches.where((f) => f.hasData).firstOrNull;
    if (match == null) {
      state = state.copyWith(awaitingMacros: true);
      await _say('Could not find $food — tell me the grams of protein, '
          'carbs, and fat instead.');
      return true;
    }
    final factor = grams / 100.0;
    return _logMeal(
      double.parse((match.proteinGPer100g * factor).toStringAsFixed(1)),
      double.parse((match.carbsGPer100g * factor).toStringAsFixed(1)),
      double.parse((match.fatGPer100g * factor).toStringAsFixed(1)),
      label: '${_g(grams)} grams of $food',
    );
  }

  Future<String?> _programSummary() async {
    try {
      final program = await ref.read(currentProgramProvider.future);
      if (program == null) return null;
      return 'Your plan: ${program.program.split}. '
          '${program.program.daysPerWeek} days a week, '
          '${program.diet.estimatedKcal} calories.';
    } catch (_) {
      return null;
    }
  }

  Future<void> _say(String line) async {
    state = state.copyWith(line: line);
    await ref.read(voiceOutputProvider).speak(line);
  }
}

String _g(double v) => v == v.roundToDouble() ? v.toStringAsFixed(0) : '$v';
