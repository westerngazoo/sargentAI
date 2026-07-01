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

  Future<void> _toggleListening() async {
    final speech = ref.read(speechInputProvider);
    if (_listening) {
      await speech.stop();
      if (mounted) setState(() => _listening = false);
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
    });
    await speech.listen((transcript, isFinal) {
      if (!mounted) return;
      setState(() => _transcript = transcript);
      if (isFinal) {
        setState(() => _listening = false);
        _act(parseVoiceIntent(transcript));
      }
    });
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

    return Scaffold(
      appBar: AppBar(
        title: const Text('fitAI'),
        leading: BackButton(onPressed: () => context.go('/home')),
      ),
      body: SafeArea(
        child: Column(
          children: [
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
                          _positioned(options[i], i, options.length, radius),
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
              child: Text(
                _listening
                    ? (_transcript.isEmpty ? 'Listening…' : '“$_transcript”')
                    : (_hint ??
                        'Tap the mic and say what you want to do — '
                            'or tap an option.'),
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodyMedium,
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// Places option [i] of [n] on the ring, starting at 12 o'clock.
  Widget _positioned(_HubOption option, int i, int n, double radius) {
    final angle = -math.pi / 2 + 2 * math.pi * i / n;
    return Transform.translate(
      offset: Offset(radius * math.cos(angle), radius * math.sin(angle)),
      child: _OptionButton(option: option),
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
            color: cs.onPrimary,
          ),
        ),
      ),
    );
  }
}

class _OptionButton extends StatelessWidget {
  const _OptionButton({required this.option});

  final _HubOption option;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return InkWell(
      borderRadius: BorderRadius.circular(12),
      onTap: option.onTap,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Material(
            shape: const CircleBorder(),
            color: cs.secondaryContainer,
            elevation: 2,
            child: SizedBox(
              width: 64,
              height: 64,
              child: Icon(option.icon, color: cs.onSecondaryContainer),
            ),
          ),
          const SizedBox(height: 4),
          Text(option.label, style: Theme.of(context).textTheme.labelSmall),
        ],
      ),
    );
  }
}
