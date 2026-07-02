// R-0032 — the voice hub: a central speak button ringed by every primary
// action. Tapping the mic opens a hands-free conversation with the
// [Sergeant] (prompt → listen → act → re-listen); tapping an option does the
// same thing by hand. Navigation requested by the sergeant is consumed here
// (notifiers hold no BuildContext).

import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../core/brand.dart';
import '../core/theme/app_theme.dart';
import '../nutrition/presentation/meal_quick_log_sheet.dart';
import '../workout/application/session_driver.dart';
import 'sergeant.dart';

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

class _VoiceHubScreenState extends ConsumerState<VoiceHubScreen>
    with SingleTickerProviderStateMixin {
  late final AnimationController _pulse;

  @override
  void initState() {
    super.initState();
    _pulse = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1400),
    );
  }

  @override
  void dispose() {
    _pulse.dispose();
    super.dispose();
  }

  void _startWorkout() {
    ref.read(sessionDriverProvider.notifier).start();
    context.go('/session');
  }

  @override
  Widget build(BuildContext context) {
    final sergeant = ref.watch(sergeantProvider);

    // Consume the sergeant's navigation effect.
    ref.listen(sergeantProvider, (prev, next) {
      if (next.navigateTo != null) {
        final target = next.navigateTo!;
        ref.read(sergeantProvider.notifier).consumeNavigation();
        context.go(target);
      }
      if ((prev?.listening ?? false) != next.listening) {
        next.listening ? _pulse.repeat() : _pulse.stop();
      }
    });

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
        title: const Text(Brand.appName),
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
                        if (sergeant.listening) _PulseRings(animation: _pulse),
                        for (var i = 0; i < options.length; i++)
                          _positioned(options[i], i, options.length, radius),
                        _SpeakButton(
                          listening: sergeant.listening,
                          conversing: sergeant.conversing,
                          onTap: () {
                            final notifier =
                                ref.read(sergeantProvider.notifier);
                            sergeant.conversing
                                ? notifier.stop()
                                : notifier.start();
                          },
                        ),
                      ],
                    ),
                  );
                },
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 0, 24, 24),
              child: AnimatedContainer(
                duration: const Duration(milliseconds: 200),
                padding:
                    const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                decoration: BoxDecoration(
                  color: sergeant.listening
                      ? Theme.of(context).colorScheme.primaryContainer
                      : Theme.of(context).colorScheme.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(999),
                ),
                child: Text(
                  sergeant.listening && sergeant.transcript.isNotEmpty
                      ? '“${sergeant.transcript}”'
                      : (sergeant.line.isEmpty
                          ? 'Tap the mic and speak — finish every '
                              'command with "over".'
                          : sergeant.line),
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
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

/// Expanding, fading rings behind the mic while listening.
class _PulseRings extends StatelessWidget {
  const _PulseRings({required this.animation});

  final Animation<double> animation;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return AnimatedBuilder(
      animation: animation,
      builder: (context, _) {
        final t = animation.value;
        return Stack(
          alignment: Alignment.center,
          children: [
            for (final phase in const [0.0, 0.5])
              _ring(cs, ((t + phase) % 1.0)),
          ],
        );
      },
    );
  }

  Widget _ring(ColorScheme cs, double t) => Container(
        width: 112 + 90 * t,
        height: 112 + 90 * t,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          border: Border.all(
            color: cs.primary.withValues(alpha: (1 - t) * 0.45),
            width: 3,
          ),
        ),
      );
}

class _SpeakButton extends StatelessWidget {
  const _SpeakButton({
    required this.listening,
    required this.conversing,
    required this.onTap,
  });

  final bool listening;
  final bool conversing;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final active = listening || conversing;
    return AnimatedContainer(
      duration: const Duration(milliseconds: 200),
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        boxShadow: [
          BoxShadow(
            color: (active ? cs.error : cs.primary).withValues(alpha: 0.35),
            blurRadius: 24,
            spreadRadius: 2,
            offset: const Offset(0, 8),
          ),
        ],
      ),
      child: Material(
        shape: const CircleBorder(),
        color: Colors.transparent,
        child: Ink(
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            gradient: active ? null : brandGradient(),
            color: active ? cs.error : null,
          ),
          child: InkWell(
            customBorder: const CircleBorder(),
            onTap: onTap,
            child: SizedBox(
              width: 112,
              height: 112,
              child: Icon(
                active ? Icons.stop : Icons.mic,
                size: 48,
                color: Colors.white,
              ),
            ),
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
          Container(
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              boxShadow: [
                BoxShadow(
                  color: cs.shadow.withValues(alpha: 0.10),
                  blurRadius: 12,
                  offset: const Offset(0, 4),
                ),
              ],
            ),
            child: Material(
              shape: const CircleBorder(),
              color: cs.secondaryContainer,
              child: SizedBox(
                width: 64,
                height: 64,
                child: Icon(option.icon, color: cs.onSecondaryContainer),
              ),
            ),
          ),
          const SizedBox(height: 6),
          Text(
            option.label,
            style: Theme.of(context)
                .textTheme
                .labelMedium
                ?.copyWith(fontWeight: FontWeight.w600),
          ),
        ],
      ),
    );
  }
}
