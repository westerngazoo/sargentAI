// SPEC-0059 §2.4 (slice 59-c1) — the squat check: its inputs, the figure
// with playback, and how the range ended. Readouts, moment bars, the live
// graph and the caveats are slice 59-c2's.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../../../l10n/app_localizations.dart';
import '../../core/network/api_error_text.dart';
import '../../core/network/api_exception.dart';
import '../../core/theme/tokens.dart';
import '../../l10n/locale_rule.dart';
import '../application/squat_check_controller.dart';
import '../domain/playback.dart';
import '../domain/range_end_text.dart';
import '../domain/squat_range.dart';
import 'figure_painter.dart';

FbfTokens _tokens(BuildContext context) =>
    Theme.of(context).extension<FbfTokens>() ?? FbfTokens.light;

/// The lifter's squat as the server computed it: asks for the dorsiflexion
/// (no default, SPEC-0045 §2.6.1.1) and the bar, then draws the range. A
/// rejected range shows its reason and no figure (R-0059 AC3).
class SquatCheck extends ConsumerWidget {
  const SquatCheck({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final asked = ref.watch(squatDorsiflexionProvider) != null;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(
              child: Text(l10n.squatTitle,
                  style: Theme.of(context).textTheme.headlineSmall),
            ),
            const _Stamp(),
          ],
        ),
        const SizedBox(height: 12),
        const _Inputs(),
        const SizedBox(height: 12),
        ref.watch(squatCheckControllerProvider).when(
              loading: () => const Center(child: CircularProgressIndicator()),
              error: (e, _) => Text(e is ApiException
                  ? apiErrorText(e, l10n)
                  : l10n.apiErrorUnknown),
              data: (result) => switch (result) {
                null when asked => Text(l10n.squatNeedsProfile),
                null => const SizedBox.shrink(),
                SquatRangeResult(end: Rejected(:final reason)) =>
                  Text(rejectedText(reason, l10n)),
                final r => _Playback(key: ObjectKey(r), result: r),
              },
            ),
      ],
    );
  }
}

/// The stamp: COMPROBABLE, checkable, never COMPROBADO (SPEC-0061 §2.10).
class _Stamp extends StatelessWidget {
  const _Stamp();

  @override
  Widget build(BuildContext context) {
    final t = _tokens(context);
    return Transform.rotate(
      angle: -0.12,
      child: Container(
        padding: const EdgeInsets.fromLTRB(7, 4, 7, 3),
        decoration: BoxDecoration(
          color: t.panel,
          border: Border.all(color: t.red, width: 2.5),
        ),
        child: Text(
          AppLocalizations.of(context).squatStamp,
          style: TextStyle(
              fontFamily: AppFonts.headline,
              fontSize: 14,
              letterSpacing: 1.2,
              color: t.red),
        ),
      ),
    );
  }
}

/// The dorsiflexion and the bar, each applied when the lifter submits it.
class _Inputs extends ConsumerWidget {
  const _Inputs();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final format = NumberFormat.decimalPattern(numberLocale(l10n.localeName));
    String text(double? v) => v == null ? '' : format.format(v);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _NumberField(
          label: l10n.squatDorsiflexionLabel,
          help: l10n.squatDorsiflexionHelp,
          invalid: l10n.squatDorsiflexionInvalid,
          initial: text(ref.read(squatDorsiflexionProvider)),
          accepts: (v) =>
              v != null && v >= dorsiflexionMinDeg && v <= dorsiflexionMaxDeg,
          onValue: (v) =>
              ref.read(squatDorsiflexionProvider.notifier).state = v,
        ),
        const SizedBox(height: 8),
        _NumberField(
          label: l10n.squatLoadLabel,
          help: l10n.squatLoadHelp,
          invalid: l10n.squatLoadInvalid,
          initial: text(ref.read(squatLoadKgProvider)),
          accepts: (v) => v == null || (v > barKgAbove && v <= barKgMax),
          onValue: (v) => ref.read(squatLoadKgProvider.notifier).state = v,
        ),
      ],
    );
  }
}

/// A numeric field that hands [onValue] the submitted number (null when
/// empty) if [accepts] it, and shows [invalid] otherwise.
class _NumberField extends StatefulWidget {
  const _NumberField({
    required this.label,
    required this.help,
    required this.invalid,
    required this.initial,
    required this.accepts,
    required this.onValue,
  });

  final String label;
  final String help;
  final String invalid;
  final String initial;
  final bool Function(double?) accepts;
  final void Function(double?) onValue;

  @override
  State<_NumberField> createState() => _NumberFieldState();
}

class _NumberFieldState extends State<_NumberField> {
  late final _controller = TextEditingController(text: widget.initial);
  String? _error;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit(String raw) {
    final text = raw.trim();
    // A decimal comma is how many lifters type it; both are accepted.
    final value =
        text.isEmpty ? null : double.tryParse(text.replaceAll(',', '.'));
    final ok = (text.isEmpty || value != null) && widget.accepts(value);
    setState(() => _error = ok ? null : widget.invalid);
    if (ok) widget.onValue(value);
  }

  @override
  Widget build(BuildContext context) => TextField(
        controller: _controller,
        keyboardType: const TextInputType.numberWithOptions(decimal: true),
        textInputAction: TextInputAction.done,
        onSubmitted: _submit,
        decoration: InputDecoration(
          labelText: widget.label,
          helperText: widget.help,
          helperMaxLines: 2,
          errorText: _error,
        ),
      );
}

/// The figure over the returned frames, with play/pause and a scrubber.
/// Opens paused on the last frame: the bottom, or where the lifter stops.
class _Playback extends StatefulWidget {
  const _Playback({super.key, required this.result});

  final SquatRangeResult result;

  @override
  State<_Playback> createState() => _PlaybackState();
}

class _PlaybackState extends State<_Playback>
    with SingleTickerProviderStateMixin {
  // 0 upright to 1 at the last returned frame; down and back up on play.
  late final _ticker = AnimationController(
    vsync: this,
    duration: const Duration(seconds: 3),
    value: 1,
  )..addListener(() => setState(() {}));

  @override
  void dispose() {
    _ticker.dispose();
    super.dispose();
  }

  void _toggle() {
    _ticker.isAnimating ? _ticker.stop() : _ticker.repeat(reverse: true);
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final frames = widget.result.frames;
    final (first, last) = (frames.first.progress, frames.last.progress);
    final progress = first + (last - first) * _ticker.value;
    // The painter's range and the frame it draws come from one list.
    final current = playbackAt(frames, progress);
    if (current == null) return const SizedBox.shrink();
    final t = _tokens(context);
    final end = widget.result.end;
    final playing = _ticker.isAnimating;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 280),
            child: AspectRatio(
              aspectRatio: 0.75,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: t.panel,
                  border: Border.all(color: t.line, width: 2),
                ),
                child: Semantics(
                  label: l10n.squatFigureLabel,
                  child: CustomPaint(
                    painter: SquatFigurePainter(
                        range: frames, current: current, tokens: t),
                  ),
                ),
              ),
            ),
          ),
        ),
        if (end is Stopped) ...[
          const SizedBox(height: 8),
          Text(stoppedText(end, frames.first.posture.load?.kg, l10n)),
        ],
        if (frames.length > 1)
          Row(
            children: [
              IconButton.outlined(
                tooltip: playing ? l10n.squatPause : l10n.squatPlay,
                icon: Icon(playing ? Icons.pause : Icons.play_arrow),
                onPressed: _toggle,
              ),
              Expanded(
                child: Semantics(
                  label: l10n.squatScrub,
                  child: Slider(
                    value: _ticker.value,
                    onChanged: (v) {
                      _ticker.stop();
                      _ticker.value = v;
                    },
                  ),
                ),
              ),
            ],
          ),
      ],
    );
  }
}
