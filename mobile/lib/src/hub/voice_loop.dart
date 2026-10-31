// The hands-free voice loop (R-0032; SPEC-0061 §2.8), the engine of the
// Dictado sheet. It prompts, listens, acts, and re-listens — meals log by
// voice alone, and anything that leaves the sheet is surfaced as a typed
// [VoiceEffect] the screen performs once the sheet has closed (notifiers
// hold no BuildContext). Every line is the client's own, in its language;
// the server's wording never reaches the user (R-0061 AC11).

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import '../l10n/app_localizations_provider.dart';
import '../nutrition/domain/preset_meals.dart';
import '../nutrition/models/food_info.dart';
import '../nutrition/services/nutrition_service.dart';
import '../program/application/program_providers.dart';
import 'effect_lines.dart';
import 'speech_input.dart';
import 'voice_effect.dart';
import 'voice_intent.dart';
import 'voice_intent_service.dart';
import 'voice_protocol.dart';
import 'voice_output.dart';

/// One bubble in the Dictado thread.
typedef ChatTurn = ({bool fromUser, String text});

/// What the Dictado sheet draws: the conversation and whether the loop is
/// talking or listening.
@immutable
class VoiceLoopState {
  const VoiceLoopState({
    this.conversing = false,
    this.listening = false,
    this.transcript = '',
    this.line = '',
    this.history = const [],
    this.awaitingMacros = false,
    this.effect,
  });

  final bool conversing;
  final bool listening;

  /// The user's last (or in-flight) dictation.
  final String transcript;

  /// The loop's last spoken line — also rendered in the idle pill.
  final String line;

  /// The conversation so far, oldest first (capped), for the chat thread.
  final List<ChatTurn> history;

  /// A meal was requested without macros; the next dictation answers them.
  final bool awaitingMacros;

  /// What the loop asks the screen to do; the sheet closes with it and the
  /// screen performs it (notifiers have no context).
  final VoiceEffect? effect;

  VoiceLoopState copyWith({
    bool? conversing,
    bool? listening,
    String? transcript,
    String? line,
    List<ChatTurn>? history,
    bool? awaitingMacros,
    VoiceEffect? effect,
    bool clearEffect = false,
  }) =>
      VoiceLoopState(
        conversing: conversing ?? this.conversing,
        listening: listening ?? this.listening,
        transcript: transcript ?? this.transcript,
        line: line ?? this.line,
        history: history ?? this.history,
        awaitingMacros: awaitingMacros ?? this.awaitingMacros,
        effect: clearEffect ? null : (effect ?? this.effect),
      );
}

final voiceLoopProvider =
    NotifierProvider<VoiceLoop, VoiceLoopState>(VoiceLoop.new);

/// The conversation: [start] opens it, [stop] ends it, and the sheet
/// consumes each [VoiceLoopState.effect] with [consumeEffect].
class VoiceLoop extends Notifier<VoiceLoopState> {
  /// Fruitless rounds (silence or unknown) tolerated before standing by.
  static const _maxIdleRounds = 3;
  int _idleRounds = 0;

  @override
  VoiceLoopState build() => const VoiceLoopState();

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

  /// Ends the conversation and clears the thread. The audio engines are
  /// stopped only while they run: after an effect the loop has already said
  /// its line and stopped listening, and a stop then could cut into what the
  /// screen says next (the coach's first line).
  Future<void> stop() async {
    if (state.conversing || state.listening) {
      await ref.read(speechInputProvider).stop();
      await ref.read(voiceOutputProvider).stop();
    }
    state = const VoiceLoopState();
  }

  /// The sheet calls this as it closes with [VoiceLoopState.effect].
  void consumeEffect() => state = state.copyWith(clearEffect: true);

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
      return _emit(const Close());
    }
    if (state.awaitingMacros) return _handleMacros(transcript);

    // Backend LLM/keyword parser (R-0032 slice 2) — falls back to local on error.
    try {
      final result =
          await ref.read(voiceIntentServiceProvider).parse(transcript);
      return _handleBackendResult(result);
    } catch (_) {
      // Offline / upstream failure — the local parser keeps the sheet usable.
    }

    return _handleLocal(transcript);
  }

  /// The server's answer, acknowledged in the client's words: its `message`
  /// and `prompt` are English prose and stay unspoken (SPEC-0061 §2.8).
  Future<bool> _handleBackendResult(VoiceIntentResult result) async {
    _idleRounds = 0;
    if (result.isLoggedNutrition || result.isLoggedWorkout) {
      await _say(_l10n.sergeantLogged);
      return true;
    }
    if (result.isClarify) {
      state = state.copyWith(awaitingMacros: true);
      await _say(_l10n.sergeantAskMacros);
      return true;
    }
    final effect = result.effect;
    if (effect != null) {
      await _say(lineForEffect(effect, _l10n));
      return _emit(effect);
    }
    await _say(_l10n.sergeantDidNotCatch);
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
        return _emit(const Close());
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
        // The screen seeds and starts the session, and hands it to the
        // coach, once the sheet has closed (SPEC-0061 §2.8).
        await _say(_l10n.sergeantStartingSession);
        return _emit(const OpenFocusMode());
      case ShowProgramIntent():
        // With no plan yet, Programa opens on its choose flow.
        await _say(await _programSummary() ?? _l10n.sergeantNoPlanYet);
        return _emit(const OpenPanel(PanelKind.programa));
      case BodyMatchIntent():
        // The body match lives inside Programa's choose flow.
        await _say(_l10n.sergeantOpeningBodyMatch);
        return _emit(const OpenPanel(PanelKind.programa));
      case ShowHistoryIntent():
        // The calendar is the screen behind the sheet.
        await _say(_l10n.sergeantRecentActivity);
        return _emit(const Close());
      case ShowProfileIntent():
        await _say(_l10n.sergeantOpeningProfile);
        return _emit(const OpenOnboarding());
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

  /// Ends the conversation with [effect] for the sheet to close with. A
  /// conversation already stopped by hand emits nothing: the sheet is gone.
  bool _emit(VoiceEffect effect) {
    if (state.conversing) {
      state = state.copyWith(conversing: false, effect: effect);
    }
    return false;
  }

  /// Speaks [line] and adds it to the thread, unless the conversation was
  /// stopped by hand meanwhile.
  Future<void> _say(String line) async {
    if (!state.conversing) return;
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
