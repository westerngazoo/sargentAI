// SPEC-0059 §2.4 (R-0059 AC4) — one bar per joint for the frame shown. A
// positive moment runs right of the zero line in the tension ramp; a
// negative one (the load assisting extension) runs left of it in neutral
// grey. Heat is tension magnitude only (SPEC-0061 §2.10). Display only: the
// numbers are the server's.

import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

import '../../../l10n/app_localizations.dart';
import '../../core/theme/tokens.dart';
import '../../l10n/locale_rule.dart';
import '../domain/squat_range.dart';

/// Where the zero line sits across a bar's track, `0..=1` from the left:
/// design v2's 16 %, room for the negative moments the model produces.
const momentZero = 0.16;

/// A bar across its track, in fractions of the track's width; [heat] in
/// `0..=1` for a positive moment, null for a negative one (no heat).
@immutable
class MomentBarSpan {
  const MomentBarSpan(this.left, this.width, this.heat);

  final double left;
  final double width;
  final double? heat;
}

/// The bar for [moment] when the range's largest magnitude is [maxAbs]
/// (`maxAbsMoment`). Both sides share one scale; a negative bar is clipped
/// at the track's left edge. A [maxAbs] of zero or less draws nothing.
MomentBarSpan momentBar(double moment, double maxAbs) {
  if (maxAbs <= 0) return const MomentBarSpan(momentZero, 0, null);
  final span = moment.abs() / maxAbs * (1 - momentZero);
  if (moment >= 0) return MomentBarSpan(momentZero, span, moment / maxAbs);
  final left = math.min(momentZero, span);
  return MomentBarSpan(momentZero - left, left, null);
}

/// The four joints' bars for [frame], scaled to [maxAbs].
class MomentBars extends StatelessWidget {
  const MomentBars({super.key, required this.frame, required this.maxAbs});

  final RangeFrame frame;
  final double maxAbs;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final t = Theme.of(context).extension<FbfTokens>() ?? FbfTokens.light;
    final text = Theme.of(context).textTheme;
    final format = NumberFormat.decimalPatternDigits(
        locale: numberLocale(l10n.localeName), decimalDigits: 0);
    final mono = text.bodySmall?.copyWith(fontFamily: AppFonts.mono);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(l10n.squatGroupsTitle,
            style: text.labelSmall?.copyWith(color: t.muted)),
        for (final joint in Joint.values)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 3),
            child: Row(
              children: [
                SizedBox(
                    width: 96,
                    child: Text(l10n.squatJoint(joint.name), style: mono)),
                Expanded(
                  child: _Track(
                    joint: joint,
                    span: momentBar(frame.mechanics.moment(joint), maxAbs),
                    tokens: t,
                  ),
                ),
                SizedBox(
                  width: 80,
                  child: Text(
                    l10n.squatMomentValue(
                        format.format(frame.mechanics.moment(joint))),
                    textAlign: TextAlign.end,
                    style: mono,
                  ),
                ),
              ],
            ),
          ),
      ],
    );
  }
}

class _Track extends StatelessWidget {
  const _Track({required this.joint, required this.span, required this.tokens});

  final Joint joint;
  final MomentBarSpan span;
  final FbfTokens tokens;

  @override
  Widget build(BuildContext context) => SizedBox(
        height: 12,
        child: LayoutBuilder(
          builder: (context, box) {
            final w = box.maxWidth;
            return Stack(
              children: [
                Positioned(
                  left: w * span.left,
                  width: w * span.width,
                  top: 0,
                  bottom: 0,
                  child: ColoredBox(
                    key: Key('moment-bar-${joint.name}'),
                    color: switch (span.heat) {
                      null => tokens.muted,
                      final heat => tokens.heat(heat),
                    },
                  ),
                ),
                Positioned(
                  left: w * momentZero,
                  width: 1.5,
                  top: 0,
                  bottom: 0,
                  child: ColoredBox(
                      key: Key('moment-zero-${joint.name}'),
                      color: tokens.text),
                ),
              ],
            );
          },
        ),
      );
}
