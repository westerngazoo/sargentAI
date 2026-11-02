// SPEC-0059 §2.4 (R-0059 AC4) — the frame's readouts in design v2's
// `panelito` boxes: τ cadera, palanca cadera, τ rodilla, β tronco, then the
// flexions and progress in the mono face. The server's numbers, rounded for
// reading; the lever is shown in cm. Only the torque boxes carry the
// tension ramp on their edge: heat is tension magnitude, never a length or
// an angle (SPEC-0061 §2.10).

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../core/theme/tokens.dart';
import '../../l10n/locale_rule.dart';
import '../domain/squat_range.dart';

/// The readouts for [frame]; [maxAbs] scales the torque boxes' heat. A
/// bodyweight frame has no lever box.
class FrameReadouts extends StatelessWidget {
  const FrameReadouts({super.key, required this.frame, required this.maxAbs});

  final RangeFrame frame;
  final double maxAbs;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final t = FbfTokens.of(context);
    final format = numberFormat(l10n.localeName, digits: 0);
    Color heat(double moment) =>
        maxAbs > 0 ? t.heat(math.max(0, moment) / maxAbs) : t.line;
    final hip = frame.mechanics.moment(Joint.hip);
    final knee = frame.mechanics.moment(Joint.knee);
    final lever = frame.hip;
    // Units are SI symbols, the same in every language.
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            _Box(
                key: const Key('readout-hip'),
                label: l10n.squatReadoutHip,
                value: format.format(hip),
                unit: 'N·m',
                edge: heat(hip),
                tokens: t),
            if (lever != null)
              _Box(
                  key: const Key('readout-lever'),
                  label: l10n.squatReadoutLever,
                  value: format.format(lever.leverM * 100),
                  unit: 'cm',
                  edge: t.line,
                  tokens: t),
            _Box(
                key: const Key('readout-knee'),
                label: l10n.squatReadoutKnee,
                value: format.format(knee),
                unit: 'N·m',
                edge: heat(knee),
                tokens: t),
            _Box(
                key: const Key('readout-trunk'),
                label: l10n.squatReadoutTrunk,
                value: format.format(frame.trunkLeanDeg),
                unit: '°',
                edge: t.line,
                tokens: t),
          ],
        ),
        const SizedBox(height: 6),
        Text(
          l10n.squatAngles(
            format.format(frame.kneeFlexionDeg),
            format.format(frame.hipFlexionDeg),
            (frame.progress * 100).round(),
          ),
          style: Theme.of(context)
              .textTheme
              .bodySmall
              ?.copyWith(fontFamily: AppFonts.mono),
        ),
      ],
    );
  }
}

class _Box extends StatelessWidget {
  const _Box({
    super.key,
    required this.label,
    required this.value,
    required this.unit,
    required this.edge,
    required this.tokens,
  });

  final String label;
  final String value;
  final String unit;
  final Color edge;
  final FbfTokens tokens;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Container(
      constraints: const BoxConstraints(minWidth: 110),
      padding: const EdgeInsets.fromLTRB(8, 6, 8, 6),
      decoration: BoxDecoration(
        color: tokens.panel,
        border: Border(
          left: BorderSide(color: edge, width: 5),
          top: BorderSide(color: tokens.line, width: 2),
          right: BorderSide(color: tokens.line, width: 2),
          bottom: BorderSide(color: tokens.line, width: 2),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: text.labelSmall?.copyWith(color: tokens.muted)),
          Row(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: [
              Text(value,
                  style: TextStyle(
                      fontFamily: AppFonts.big,
                      fontSize: 24,
                      color: tokens.text)),
              const SizedBox(width: 4),
              Text(unit, style: text.labelSmall),
            ],
          ),
        ],
      ),
    );
  }
}
