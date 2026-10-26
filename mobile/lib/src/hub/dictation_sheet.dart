// SPEC-0061 §2.6, §2.8 (slice 3b): the Dictado panel — the mic and the chat
// thread of the [VoiceLoop], nothing else. The ring the hub drew is gone:
// each of its items has a home on the main screen. The sheet closes itself
// when the loop emits an effect and hands that effect to whoever opened it.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import '../core/panels/show_panel.dart';
import 'voice_effect.dart';
import 'voice_loop.dart';

/// Opens the Dictado sheet on [showPanel]. Resolves, once the sheet has
/// closed, with the [VoiceEffect] the loop emitted, or with null when the
/// sheet was closed by hand (✕, Esc, a tap outside, back). The loop is
/// stopped either way, so the opener performs the effect on a quiet loop.
Future<VoiceEffect?> showDictationSheet(BuildContext context) =>
    showPanel<VoiceEffect>(
      context,
      title: AppLocalizations.of(context).dictationTitle,
      scrolls: false,
      body: (_) => const DictationSheet(),
    );

/// The sheet's body: the thread of the conversation, or the idle hint before
/// one, over the mic that starts and stops it.
class DictationSheet extends ConsumerWidget {
  const DictationSheet({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final loop = ref.watch(voiceLoopProvider);
    final notifier = ref.read(voiceLoopProvider.notifier);
    ref.listen<VoiceLoopState>(voiceLoopProvider, (_, next) {
      final effect = next.effect;
      if (effect == null) return;
      notifier.consumeEffect();
      Navigator.of(context).pop(effect);
    });
    return PopScope(
      // Every way out — ✕, Esc, outside, back, or the loop's own effect —
      // leaves the loop stopped. A pop is never during a build, so the
      // notifier may change here; a dispose would be too late.
      onPopInvokedWithResult: (didPop, _) {
        if (didPop) notifier.stop();
      },
      child: Column(
        children: [
          Expanded(
            child: loop.history.isEmpty
                ? Center(
                    child: Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 20),
                      child: _HintPill(loop: loop),
                    ),
                  )
                : _ChatThread(loop: loop),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(0, 12, 0, 24),
            child: _SpeakButton(
              listening: loop.listening,
              conversing: loop.conversing,
              onTap: () => loop.conversing ? notifier.stop() : notifier.start(),
            ),
          ),
        ],
      ),
    );
  }
}

/// The idle hint (shown before any conversation), or the loop's one line
/// when it has nothing to converse with (speech unavailable).
class _HintPill extends StatelessWidget {
  const _HintPill({required this.loop});

  final VoiceLoopState loop;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
      decoration: BoxDecoration(
        color: loop.listening ? cs.primaryContainer : cs.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        loop.listening && loop.transcript.isNotEmpty
            ? '“${loop.transcript}”'
            : (loop.line.isEmpty
                ? AppLocalizations.of(context).hubIdleHint
                : loop.line),
        textAlign: TextAlign.center,
        style: Theme.of(context).textTheme.bodyMedium,
      ),
    );
  }
}

/// The conversation as bubbles, newest at the bottom, with the live
/// transcript while listening.
class _ChatThread extends StatelessWidget {
  const _ChatThread({required this.loop});

  final VoiceLoopState loop;

  @override
  Widget build(BuildContext context) => ListView(
        reverse: true,
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
        children: [
          if (loop.listening)
            _ChatBubble(
              fromUser: true,
              text: loop.transcript.isEmpty
                  ? AppLocalizations.of(context).listening
                  : loop.transcript,
              live: true,
            ),
          for (final turn in loop.history.reversed)
            _ChatBubble(fromUser: turn.fromUser, text: turn.text),
        ],
      );
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

/// The mic: starts the conversation, and reads as a stop while it runs.
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
            color: active ? cs.error : cs.primary,
          ),
          child: InkWell(
            key: const Key('dictation-mic'),
            customBorder: const CircleBorder(),
            onTap: onTap,
            child: SizedBox(
              width: 88,
              height: 88,
              child: Icon(
                active ? Icons.stop : Icons.mic,
                size: 40,
                color: cs.onPrimary,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
