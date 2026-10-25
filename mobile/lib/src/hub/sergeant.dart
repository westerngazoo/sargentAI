// The hands-free wizard (R-0032): a conversation loop over the hub. The
// sergeant prompts, listens, acts, and re-listens — meals log by voice alone,
// "start workout" hands off to the [VoiceCoach]'s own hands-free loop, and
// navigation is surfaced as a state effect the screen consumes (notifiers
// hold no BuildContext).

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import '../l10n/app_localizations_provider.dart';
import '../nutrition/domain/preset_meals.dart';
import '../nutrition/models/food_info.dart';
import '../nutrition/services/nutrition_service.dart';
import '../program/application/program_providers.dart';
import '../workout/application/session_driver.dart';
import '../workout/application/voice_coach.dart';
import 'speech_input.dart';
import 'voice_intent.dart';
import 'voice_intent_service.dart';
import 'voice_protocol.dart';
import 'voice_output.dart';

/// One bubble in the hub's chat thread.
typedef ChatTurn = ({bool fromUser, String text});

@immutable
class SergeantState {
  const SergeantState({
    this.conversing = false,
    this.listening = false,
    this.transcript = '',
    this.line = '',
    this.history = const [],
    this.awaitingMacros = false,
    this.navigateTo,
  });

  final bool conversing;
  final bool listening;

  /// The user's last (or in-flight) dictation.
  final String transcript;

  /// The sergeant's last spoken line — also rendered in the hub pill.
  final String line;

  /// The conversation so far, oldest first (capped), for the chat thread.
  final List<ChatTurn> history;

  /// A meal was requested without macros; the next dictation answers them.
  final bool awaitingMacros;

  /// Navigation effect for the screen to consume (notifiers have no context).
  final String? navigateTo;

  SergeantState copyWith({
    bool? conversing,
    bool? listening,
    String? transcript,
    String? line,
    List<ChatTurn>? history,
    bool? awaitingMacros,
    String? navigateTo,
    bool clearNavigation = false,
  }) =>
      SergeantState(
        conversing: conversing ?? this.conversing,
        listening: listening ?? this.listening,
        transcript: transcript ?? this.transcript,
        line: line ?? this.line,
        history: history ?? this.history,
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

  /// The lines are spoken from here, outside the widget tree (R-0061 AC11).
  AppLocalizations get _l10n => ref.read(appLocalizationsProvider);

  /// Opens the conversation: prompt → listen → act → re-listen.
  Future<void> start() async {
    final speech = ref.read(speechInputProvider);
    await ref.read(voiceOutputProvider).initialize();
    if (!await speech.initialize()) {
      state = state.copyWith(line: _l10n.sergeantVoiceUnavailable);
      return;
    }
    _idleRounds = 0;
    state = state.copyWith(conversing: true, awaitingMacros: false);
    await _say(_l10n.sergeantGreeting);
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
      state = state.copyWith(history: _appended(fromUser: true, text: command));
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
      await _say(_l10n.sergeantRogerOut);
      state = state.copyWith(conversing: false);
      return false;
    }
    if (state.awaitingMacros) return _handleMacros(transcript);

    // Backend LLM/keyword parser (R-0032 slice 2) — falls back to local on error.
    try {
      final result =
          await ref.read(voiceIntentServiceProvider).parse(transcript);
      return _handleBackendResult(result);
    } catch (_) {
      // Offline / upstream failure — local keyword parser keeps the hub usable.
    }

    return _handleLocal(transcript);
  }

  Future<bool> _handleBackendResult(VoiceIntentResult result) async {
    _idleRounds = 0;
    if (result.isLoggedNutrition || result.isLoggedWorkout) {
      await _say(result.message ?? _l10n.sergeantLogged);
      return true;
    }
    if (result.isClarify) {
      state = state.copyWith(awaitingMacros: true);
      await _say(result.prompt ?? _l10n.sergeantTellMeMore);
      return true;
    }
    if (result.isNavigate && result.route != null) {
      await _say(result.message ?? _l10n.sergeantRoger);
      state = state.copyWith(conversing: false, navigateTo: result.route);
      if (result.route == '/session') {
        ref.read(sessionDriverProvider.notifier).start();
        ref.read(voiceCoachProvider.notifier).enable();
      }
      return false;
    }
    await _say(result.message ?? _l10n.sergeantDidNotCatch);
    return true;
  }

  Future<bool> _handleLocal(String transcript) async {
    // A named preset meal works without any "meal" keyword ("log a protein
    // shake") — but never when digits suggest a macro/portion dictation.
    if (!RegExp(r'\d').hasMatch(transcript)) {
      final preset = matchPresetMeal(transcript);
      if (preset != null) {
        _idleRounds = 0;
        return _logMeal(preset.proteinG, preset.carbsG, preset.fatG,
            label: preset.name);
      }
    }

    switch (parseVoiceIntent(transcript)) {
      case StopIntent():
        await _say(_l10n.sergeantStandingBy);
        state = state.copyWith(conversing: false);
        return false;
      case LogMealIntent(:final proteinG, :final carbsG, :final fatG):
        _idleRounds = 0;
        if (proteinG != null && carbsG != null && fatG != null) {
          return _logMeal(proteinG, carbsG, fatG);
        }
        final portion = parseFoodQuantity(transcript);
        if (portion != null) return _logFood(portion.food, portion.grams);
        final preset = matchPresetMeal(transcript);
        if (preset != null) {
          return _logMeal(preset.proteinG, preset.carbsG, preset.fatG,
              label: preset.name);
        }
        state = state.copyWith(awaitingMacros: true);
        await _say(_l10n.sergeantAskMacros);
        return true;
      case LogWorkoutIntent():
        await _say(_l10n.sergeantStartingSession);
        ref.read(sessionDriverProvider.notifier).start();
        state = state.copyWith(conversing: false, navigateTo: '/session');
        // Fire-and-forget: the coach announces the plan; push-to-talk —
        // the user keys the mic (later: earbud/watch button) per command.
        ref.read(voiceCoachProvider.notifier).enable();
        return false;
      case ShowProgramIntent():
        final summary = await _programSummary();
        if (summary != null) {
          await _say(summary);
          state = state.copyWith(
              conversing: false, navigateTo: '/programs/current');
        } else {
          await _say(_l10n.sergeantNoPlanYet);
          state =
              state.copyWith(conversing: false, navigateTo: '/programs/get');
        }
        return false;
      case BodyMatchIntent():
        await _say(_l10n.sergeantOpeningBodyMatch);
        state = state.copyWith(conversing: false, navigateTo: '/programs/get');
        return false;
      case ShowHistoryIntent():
        await _say(_l10n.sergeantRecentActivity);
        state = state.copyWith(conversing: false, navigateTo: '/home');
        return false;
      case ShowProfileIntent():
        await _say(_l10n.sergeantOpeningProfile);
        state = state.copyWith(conversing: false, navigateTo: '/onboarding');
        return false;
      case UnknownIntent(:final transcript):
        _idleRounds += 1;
        if (_idleRounds >= _maxIdleRounds) {
          state = state.copyWith(conversing: false);
          return false;
        }
        await _say(transcript.isEmpty
            ? _l10n.sergeantSayOptions
            : _l10n.sergeantDidNotGet(transcript));
        return true;
    }
  }

  Future<bool> _handleMacros(String transcript) async {
    final portion = parseFoodQuantity(transcript);
    if (portion != null) {
      state = state.copyWith(awaitingMacros: false);
      return _logFood(portion.food, portion.grams);
    }
    final followUpPreset = matchPresetMeal(transcript);
    if (followUpPreset != null && !RegExp(r'\d').hasMatch(transcript)) {
      state = state.copyWith(awaitingMacros: false);
      return _logMeal(
          followUpPreset.proteinG, followUpPreset.carbsG, followUpPreset.fatG,
          label: followUpPreset.name);
    }
    final macros = parseMacros(transcript);
    if (macros == null) {
      _idleRounds += 1;
      if (_idleRounds >= _maxIdleRounds) {
        state = state.copyWith(conversing: false, awaitingMacros: false);
        return false;
      }
      await _say(_l10n.sergeantMacrosExample);
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
      final what = label == null
          ? _l10n.sergeantMealLogged
          : _l10n.sergeantLoggedLabel(label);
      await _say(_l10n.sergeantMealSummary(
          what, _g(p), _g(c), _g(f), log.calories.round()));
      return true;
    } catch (_) {
      await _say(_l10n.sergeantCouldNotSaveMeal);
      return true;
    }
  }

  /// Nutrient lookup: "`<grams>` grams of `<food>`" → USDA macros → meal.
  Future<bool> _logFood(String food, double grams) async {
    await _say(_l10n.sergeantLookingUp(food));
    List<FoodInfo> matches;
    try {
      matches = await ref.read(nutritionServiceProvider).searchFoods(food);
    } catch (_) {
      matches = const [];
    }
    final match = matches.where((f) => f.hasData).firstOrNull;
    if (match == null) {
      state = state.copyWith(awaitingMacros: true);
      await _say(_l10n.sergeantCouldNotFind(food));
      return true;
    }
    final factor = grams / 100.0;
    return _logMeal(
      double.parse((match.proteinGPer100g * factor).toStringAsFixed(1)),
      double.parse((match.carbsGPer100g * factor).toStringAsFixed(1)),
      double.parse((match.fatGPer100g * factor).toStringAsFixed(1)),
      label: _l10n.sergeantGramsOf(_g(grams), food),
    );
  }

  Future<String?> _programSummary() async {
    try {
      final program = await ref.read(currentProgramProvider.future);
      if (program == null) return null;
      return _l10n.sergeantPlanSummary(
        program.program.split,
        program.program.daysPerWeek,
        program.diet.estimatedKcal,
      );
    } catch (_) {
      return null;
    }
  }

  Future<void> _say(String line) async {
    state = state.copyWith(
        line: line, history: _appended(fromUser: false, text: line));
    await ref.read(voiceOutputProvider).speak(line);
  }

  /// History with one turn appended, capped to the last 12.
  List<ChatTurn> _appended({required bool fromUser, required String text}) {
    final next = [...state.history, (fromUser: fromUser, text: text)];
    return next.length > 12 ? next.sublist(next.length - 12) : next;
  }
}

String _g(double v) => v == v.roundToDouble() ? v.toStringAsFixed(0) : '$v';
