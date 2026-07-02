// R-0032 (slice 1) — the voice hub: a central speak button ringed by every
// primary action. Tap an option or dictate it; both roads lead to the same
// screens. STT runs through the [SpeechInput] seam; intent mapping is the
// pure [parseVoiceIntent] (LLM-backed parsing arrives with the full R-0032).

import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../nutrition/presentation/meal_quick_log_sheet.dart';
import '../workout/application/session_driver.dart';
import 'speech_input.dart';
import 'voice_intent.dart';

/// One ring option: an icon + label that fires [onTap].
class _HubOption {
  const _HubOption(this.icon, this.label, this.onTap);

  final IconData icon;
  final String label;
  final VoidCallback onTap;
}

class VoiceHubScreen extends ConsumerStatefulWidget {
  const VoiceHubScreen({super.key});

  @override
  ConsumerState<VoiceHubScreen> createState() => _VoiceHubScreenState();
}

class _VoiceHubScreenState extends ConsumerState<VoiceHubScreen> {
  bool _listening = false;
  String _transcript = '';
  String? _hint;
  bool _committed = false;

  Future<void> _toggleListening() async {
    final speech = ref.read(speechInputProvider);
    if (_listening) {
      await speech.stop();
      if (!mounted) return;
      setState(() => _listening = false);
      _commitTranscript(_transcript);
      return;
    }
    final ready = await speech.initialize();
    if (!mounted) return;
    if (!ready) {
      setState(() => _hint = 'Voice input is not available here — '
          'tap an option instead.');
      return;
    }
    setState(() {
      _listening = true;
      _transcript = '';
      _hint = null;
      _committed = false;
    });
    await speech.listen((transcript, isFinal) {
      if (!mounted) return;
      setState(() => _transcript = transcript);
      if (isFinal) {
        setState(() => _listening = false);
        _commitTranscript(transcript);
      }
    });
  }

  void _commitTranscript(String transcript) {
    if (_committed) return;
    _committed = true;
    _act(parseVoiceIntent(transcript));
  }

  void _act(VoiceIntent intent) {
    switch (intent) {
      case LogWorkoutIntent():
        _startWorkout();
      case LogMealIntent(:final proteinG, :final carbsG, :final fatG):
        showMealQuickLogSheet(context,
            proteinG: proteinG, carbsG: carbsG, fatG: fatG);
      case ShowProgramIntent():
        context.go('/programs/current');
      case BodyMatchIntent():
        context.go('/programs/get');
      case ShowHistoryIntent():
        context.go('/home');
      case ShowProfileIntent():
        context.go('/onboarding');
      case UnknownIntent(:final transcript):
        setState(() => _hint = transcript.isEmpty
            ? 'Didn\'t catch that — try "log a meal" or "start a workout".'
            : 'Didn\'t understand "$transcript" — try "log a meal" or '
                '"start a workout".');
    }
  }

  void _startWorkout() {
    ref.read(sessionDriverProvider.notifier).start();
    context.go('/session');
  }

  String _statusText(String? matchedLabel) {
    if (_listening) {
      if (_transcript.isEmpty) {
        return 'Listening… say an option name or tap one below.';
      }
      if (matchedLabel != null) {
        return '“$_transcript” → $matchedLabel. Tap stop when done.';
      }
      return '“$_transcript”';
    }
    return _hint ?? 'Tap the mic and say what you want — or tap an option.';
  }

  @override
  Widget build(BuildContext context) {
    final options = [
      _HubOption(Icons.fitness_center, 'Workout', _startWorkout),
      _HubOption(
          Icons.restaurant, 'Meal', () => showMealQuickLogSheet(context)),
      _HubOption(
          Icons.assignment, 'Program', () => context.go('/programs/current')),
      _HubOption(Icons.accessibility_new, 'Body match',
          () => context.go('/programs/get')),
      _HubOption(Icons.history, 'History', () => context.go('/home')),
      _HubOption(Icons.person, 'Profile', () => context.go('/onboarding')),
    ];
    final matchedLabel =
        _listening ? matchedHubOptionLabel(_transcript) : null;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Voice hub'),
        leading: BackButton(onPressed: () => context.go('/home')),
      ),
      body: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 8, 24, 0),
              child: Text(
                'What do you want to do?',
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
            Expanded(
              child: LayoutBuilder(
                builder: (context, constraints) {
                  final size =
                      math.min(constraints.maxWidth, constraints.maxHeight);
                  final radius = size * 0.36;
                  // SizedBox.expand keeps the Stack full-size: a loose Stack
                  // shrinks to its largest child and then the translated ring
                  // falls outside its hit-test bounds.
                  return SizedBox.expand(
                    child: Stack(
                      alignment: Alignment.center,
                      children: [
                        for (var i = 0; i < options.length; i++)
                          _positioned(
                            options[i],
                            i,
                            options.length,
                            radius,
                            listening: _listening,
                            matched: options[i].label == matchedLabel,
                          ),
                        _SpeakButton(
                          listening: _listening,
                          onTap: _toggleListening,
                        ),
                      ],
                    ),
                  );
                },
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 0, 24, 24),
              child: AnimatedSwitcher(
                duration: const Duration(milliseconds: 200),
                child: Text(
                  _statusText(matchedLabel),
                  key: ValueKey(_statusText(matchedLabel)),
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        color: matchedLabel != null
                            ? Theme.of(context).colorScheme.primary
                            : null,
                        fontWeight:
                            matchedLabel != null ? FontWeight.w600 : null,
                      ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// Places option [i] of [n] on the ring, starting at 12 o'clock.
  Widget _positioned(
    _HubOption option,
    int i,
    int n,
    double radius, {
    required bool listening,
    required bool matched,
  }) {
    final angle = -math.pi / 2 + 2 * math.pi * i / n;
    return Transform.translate(
      offset: Offset(radius * math.cos(angle), radius * math.sin(angle)),
      child: _OptionButton(
        option: option,
        listening: listening,
        matched: matched,
      ),
    );
  }
}

class _SpeakButton extends StatelessWidget {
  const _SpeakButton({required this.listening, required this.onTap});

  final bool listening;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Material(
      shape: const CircleBorder(),
      color: listening ? cs.error : cs.primary,
      elevation: 6,
      child: InkWell(
        customBorder: const CircleBorder(),
        onTap: onTap,
        child: SizedBox(
          width: 112,
          height: 112,
          child: Icon(
            listening ? Icons.stop : Icons.mic,
            size: 48,
            color: listening ? cs.onError : cs.onPrimary,
          ),
        ),
      ),
    );
  }
}

class _OptionButton extends StatelessWidget {
  const _OptionButton({
    required this.option,
    required this.listening,
    required this.matched,
  });

  final _HubOption option;
  final bool listening;
  final bool matched;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final bg = matched
        ? cs.primaryContainer
        : listening
            ? cs.surfaceContainerHighest
            : cs.secondaryContainer;
    final fg = matched
        ? cs.onPrimaryContainer
        : listening
            ? cs.onSurface
            : cs.onSecondaryContainer;
    final scale = matched ? 1.12 : listening ? 1.04 : 1.0;

    return AnimatedScale(
      scale: scale,
      duration: const Duration(milliseconds: 200),
      curve: Curves.easeOut,
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: option.onTap,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            AnimatedContainer(
              duration: const Duration(milliseconds: 200),
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                border: matched
                    ? Border.all(color: cs.primary, width: 3)
                    : listening
                        ? Border.all(
                            color: cs.outline.withValues(alpha: 0.5), width: 1)
                        : null,
                boxShadow: matched
                    ? [
                        BoxShadow(
                          color: cs.primary.withValues(alpha: 0.35),
                          blurRadius: 12,
                          spreadRadius: 2,
                        ),
                      ]
                    : null,
              ),
              child: Material(
                shape: const CircleBorder(),
                color: bg,
                elevation: matched ? 4 : 2,
                child: SizedBox(
                  width: 64,
                  height: 64,
                  child: Icon(option.icon, color: fg),
                ),
              ),
            ),
            const SizedBox(height: 4),
            Text(
              option.label,
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                    fontWeight: matched ? FontWeight.w700 : FontWeight.w500,
                    color: matched ? cs.primary : null,
                  ),
            ),
          ],
        ),
      ),
    );
  }
}
