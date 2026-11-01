// SPEC-0059 §2.4 (R-0059 AC4) — the live graph: the server's hip and knee
// moments against bar height, standing at the left and the bottom at the
// right (design v2), with the frame shown marked. Display only (AC7): it
// places the server's numbers, it derives none.

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../../core/theme/tokens.dart';
import '../domain/moment_scale.dart';
import '../domain/playback.dart';
import '../domain/squat_range.dart';

/// The joints the graph plots.
const _plotted = [Joint.hip, Joint.knee];

/// Maps a frame's moment to a point in [plot]: x is the bar's height
/// (`posture.load.at.y`, the shoulder's for a bodyweight frame, where the bar
/// would sit), highest at the left; y spans the hip and knee moments with
/// zero always inside. A span of zero puts the point in the middle instead
/// of dividing by it.
class GraphScale {
  GraphScale(List<RangeFrame> frames, this.plot)
      : _hMax = frames.map(_height).fold(double.negativeInfinity, math.max),
        _hMin = frames.map(_height).fold(double.infinity, math.min),
        _mMax = frames.expand(_moments).fold(0.0, math.max),
        _mMin = frames.expand(_moments).fold(0.0, math.min);

  final Rect plot;
  final double _hMax;
  final double _hMin;
  final double _mMax;
  final double _mMin;

  static double _height(RangeFrame f) =>
      f.posture.load?.at.y ?? f.posture.shoulder.y;

  static Iterable<double> _moments(RangeFrame f) =>
      _plotted.map(f.mechanics.moment);

  double _y(double moment) => _mMax == _mMin
      ? plot.center.dy
      : plot.bottom - (moment - _mMin) / (_mMax - _mMin) * plot.height;

  /// The point for [joint]'s moment in [frame].
  Offset at(RangeFrame frame, Joint joint) => Offset(
        _hMax == _hMin
            ? plot.center.dx
            : plot.left +
                (_hMax - _height(frame)) / (_hMax - _hMin) * plot.width,
        _y(frame.mechanics.moment(joint)),
      );

  /// The zero line's height.
  double get zeroY => _y(0);
}

/// Paints the graph over [frames], marking [current]'s returned frame. The
/// hip's curve is heated segment by segment by its moment, as the figure's
/// bar arrow is; the knee's is a thin neutral line.
class LiveGraphPainter extends CustomPainter {
  LiveGraphPainter({
    required this.frames,
    required this.current,
    required this.tokens,
  });

  final List<RangeFrame> frames;
  final PlaybackFrame current;
  final FbfTokens tokens;

  static const _inset = EdgeInsets.fromLTRB(10, 10, 10, 10);

  @override
  void paint(Canvas canvas, Size size) {
    final plot = _inset.deflateRect(Offset.zero & size);
    final scale = GraphScale(frames, plot);
    final max = maxAbsMoment(frames);
    final stroke = Paint()..style = PaintingStyle.stroke;
    canvas.drawLine(
        Offset(plot.left, scale.zeroY),
        Offset(plot.right, scale.zeroY),
        stroke
          ..color = tokens.line
          ..strokeWidth = 1);
    for (var i = 1; i < frames.length; i++) {
      final (a, b) = (frames[i - 1], frames[i]);
      canvas.drawLine(
          scale.at(a, Joint.knee),
          scale.at(b, Joint.knee),
          stroke
            ..color = tokens.muted
            ..strokeWidth = 1.6);
      final m =
          (a.mechanics.moment(Joint.hip) + b.mechanics.moment(Joint.hip)) / 2;
      canvas.drawLine(
        scale.at(a, Joint.hip),
        scale.at(b, Joint.hip),
        stroke
          ..color = tokens.heat(max > 0 ? math.max(0, m) / max : 0)
          ..strokeWidth = 4
          ..strokeCap = StrokeCap.round,
      );
    }
    final knee = scale.at(current.frame, Joint.knee);
    final hip = scale.at(current.frame, Joint.hip);
    canvas
      ..drawCircle(knee, 4, Paint()..color = tokens.panel)
      ..drawCircle(
          knee,
          4,
          stroke
            ..color = tokens.text
            ..strokeWidth = 1.5)
      ..drawCircle(hip, 6, Paint()..color = tokens.red)
      ..drawCircle(
          hip,
          6,
          stroke
            ..color = tokens.text
            ..strokeWidth = 1.8);
  }

  @override
  bool shouldRepaint(LiveGraphPainter old) =>
      old.current != current || old.frames != frames || old.tokens != tokens;
}

/// The graph with its title, legend and axis caption.
class LiveGraph extends StatelessWidget {
  const LiveGraph({super.key, required this.frames, required this.current});

  final List<RangeFrame> frames;
  final PlaybackFrame current;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final t = Theme.of(context).extension<FbfTokens>() ?? FbfTokens.light;
    final small = Theme.of(context)
        .textTheme
        .bodySmall
        ?.copyWith(fontFamily: AppFonts.mono, color: t.muted);
    Widget swatch(Decoration d) =>
        Container(width: 22, height: 5, decoration: d);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(l10n.squatGraphTitle,
            style: Theme.of(context)
                .textTheme
                .labelSmall
                ?.copyWith(color: t.muted)),
        const SizedBox(height: 6),
        DecoratedBox(
          decoration: BoxDecoration(
            color: t.panel,
            border: Border.all(color: t.line, width: 2),
          ),
          child: SizedBox(
            height: 150,
            child: Semantics(
              label: l10n.squatGraphLabel,
              child: CustomPaint(
                painter: LiveGraphPainter(
                    frames: frames, current: current, tokens: t),
              ),
            ),
          ),
        ),
        const SizedBox(height: 6),
        Wrap(
          spacing: 16,
          runSpacing: 4,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            Row(mainAxisSize: MainAxisSize.min, children: [
              swatch(BoxDecoration(
                  gradient:
                      LinearGradient(colors: [t.heat0, t.heat1, t.heat2]))),
              const SizedBox(width: 6),
              Text(l10n.squatReadoutHip, style: small),
            ]),
            Row(mainAxisSize: MainAxisSize.min, children: [
              swatch(BoxDecoration(color: t.muted)),
              const SizedBox(width: 6),
              Text(l10n.squatReadoutKnee, style: small),
            ]),
          ],
        ),
        Text(l10n.squatGraphAxis, style: small),
      ],
    );
  }
}
