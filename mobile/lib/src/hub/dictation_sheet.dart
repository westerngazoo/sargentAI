import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/theme/app_theme.dart';
import 'sergeant.dart';

/// Opens the voice dictation sheet.
Future<void> showDictationSheet(BuildContext context) {
  return showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    builder: (_) => const DictationSheet(),
  );
}

class DictationSheet extends ConsumerStatefulWidget {
  const DictationSheet({super.key});

  @override
  ConsumerState<DictationSheet> createState() => _DictationSheetState();
}

class _DictationSheetState extends ConsumerState<DictationSheet>
    with SingleTickerProviderStateMixin {
  late final AnimationController _pulse;

  @override
  void initState() {
    super.initState();
    _pulse = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 2000),
    );
  }

  @override
  void dispose() {
    _pulse.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final sergeant = ref.watch(sergeantProvider);

    // The screen (HomeShell) handles navigation out of the hub via state observation.
    ref.listen(sergeantProvider.select((s) => s.navigateTo), (_, route) {
      if (route != null && mounted) {
        ref.read(sergeantProvider.notifier).consumeNavigation();
        Navigator.of(context).pop();
        // HomeShell or router handles the actual navigation.
      }
    });

    ref.listen(sergeantProvider.select((s) => s.conversing), (was, isNow) {
      if (was == true && isNow == false && mounted) {
        Navigator.of(context).pop();
      }
    });

    ref.listen(sergeantProvider.select((s) => s.listening), (_, isNow) {
      if (isNow) {
        _pulse.repeat();
      } else {
        _pulse.stop();
      }
    });

    return SafeArea(
      child: Container(
        color: Theme.of(context).colorScheme.surface,
        padding: EdgeInsets.only(
          bottom: MediaQuery.of(context).viewInsets.bottom,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Padding(
              padding: const EdgeInsets.all(16.0),
              child: Text(
                'Voice Dictation',
                style: Theme.of(context).textTheme.titleLarge,
              ),
            ),
            const SizedBox(height: 16),
            SizedBox(
              height: 250,
              child: Stack(
                alignment: Alignment.center,
                children: [
                  if (sergeant.listening) _PulseRings(animation: _pulse),
                  _SpeakButton(
                    listening: sergeant.listening,
                    conversing: sergeant.conversing,
                    onTap: () {
                      final notifier = ref.read(sergeantProvider.notifier);
                      sergeant.conversing ? notifier.stop() : notifier.start();
                    },
                  ),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 0, 24, 24),
              child: sergeant.history.isEmpty
                  ? _HintPill(sergeant: sergeant)
                  : _ChatThread(sergeant: sergeant),
            ),
          ],
        ),
      ),
    );
  }
}

/// The idle hint (shown before any conversation).
class _HintPill extends StatelessWidget {
  const _HintPill({required this.sergeant});

  final SergeantState sergeant;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
      decoration: BoxDecoration(
        color:
            sergeant.listening ? cs.primaryContainer : cs.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        sergeant.listening && sergeant.transcript.isNotEmpty
            ? '“${sergeant.transcript}”'
            : (sergeant.line.isEmpty
                ? 'Tap the mic and speak — finish every command with "over".'
                : sergeant.line),
        textAlign: TextAlign.center,
        style: Theme.of(context).textTheme.bodyMedium,
      ),
    );
  }
}

/// The agent chat thread: the sergeant conversation as bubbles (last few
/// turns), with the live transcript while listening.
class _ChatThread extends StatelessWidget {
  const _ChatThread({required this.sergeant});

  final SergeantState sergeant;

  @override
  Widget build(BuildContext context) {
    final turns = sergeant.history.length > 4
        ? sergeant.history.sublist(sergeant.history.length - 4)
        : sergeant.history;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        for (final turn in turns)
          _ChatBubble(fromUser: turn.fromUser, text: turn.text),
        if (sergeant.listening)
          _ChatBubble(
            fromUser: true,
            text: sergeant.transcript.isEmpty
                ? 'Listening…'
                : sergeant.transcript,
            live: true,
          ),
      ],
    );
  }
}

class _ChatBubble extends StatelessWidget {
  const _ChatBubble({
    required this.fromUser,
    required this.text,
    this.live = false,
  });

  final bool fromUser;
  final String text;
  final bool live;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Align(
      alignment: fromUser ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.only(top: 6),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        constraints: const BoxConstraints(maxWidth: 420),
        decoration: BoxDecoration(
          color: fromUser
              ? (live ? cs.primaryContainer : cs.primary)
              : cs.surfaceContainerHigh,
          borderRadius: BorderRadius.only(
            topLeft: const Radius.circular(16),
            topRight: const Radius.circular(16),
            bottomLeft: Radius.circular(fromUser ? 16 : 4),
            bottomRight: Radius.circular(fromUser ? 4 : 16),
          ),
        ),
        child: Text(
          text,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: fromUser
                    ? (live ? cs.onPrimaryContainer : cs.onPrimary)
                    : cs.onSurface,
                fontStyle: live ? FontStyle.italic : FontStyle.normal,
              ),
        ),
      ),
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
