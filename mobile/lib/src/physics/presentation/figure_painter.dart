// SPEC-0059 §2.4 — the squat figure, drawn in SPEC-0061 §2.10's ink: a
// side-view silhouette in `line`, the dashed mid-foot balance line and hip
// lever in ink, and the bar and ground forces as arrows to one scale: the
// bar's in the tension ramp, the ground's in ink over a paper underlay. Where things go is `figure_geometry.dart`'s; this only
// strokes it.

import 'dart:math' as math;

import 'package:flutter/rendering.dart';

import '../../core/theme/tokens.dart';
import '../domain/playback.dart';
import '../domain/squat_range.dart';
import 'figure_geometry.dart';

/// Paints [current] over [range] in [tokens]' colours. Its inputs are public
/// so a test reads the [geometry] it draws.
class SquatFigurePainter extends CustomPainter {
  const SquatFigurePainter({
    required this.range,
    required this.current,
    required this.tokens,
  });

  /// The returned frames playback moves through; they fix the scale.
  final List<RangeFrame> range;
  final PlaybackFrame current;
  final FbfTokens tokens;

  /// What [paint] draws on a canvas of [size].
  FigureGeometry geometry(Size size) => figureGeometry(range, current, size);

  @override
  void paint(Canvas canvas, Size size) {
    final g = geometry(size);
    Paint stroke(Color color, double width) => Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = width
      ..strokeCap = StrokeCap.round;

    canvas.drawLine(Offset(0, g.groundY), Offset(size.width, g.groundY),
        stroke(tokens.line, 2));
    _dashed(canvas, g.balanceLine, stroke(tokens.muted, 1));

    // The silhouette: each segment outlined in paper so the limbs separate.
    void limb(List<Offset> points, double metres) {
      for (final (color, extra) in [(tokens.panel, 3.0), (tokens.line, 0.0)]) {
        final paint = stroke(color, metres * g.scale + extra);
        for (var i = 0; i < points.length - 1; i++) {
          canvas.drawLine(points[i], points[i + 1], paint);
        }
      }
    }

    limb([g.foot.$1, g.foot.$2], 0.034);
    limb([g.ankle, g.knee], 0.052);
    limb([g.knee, g.hip], 0.062);
    limb([g.hip, g.shoulder], 0.072);
    canvas.drawCircle(g.head, g.headRadius, Paint()..color = tokens.line);
    limb(g.arm, 0.04);

    // The plate is drawn to scale and seen side-on, so it is an outline:
    // filled, it would hide the trunk and the head behind it.
    final bar = g.bar;
    if (bar != null) {
      canvas.drawCircle(bar, g.plateRadius, stroke(tokens.line, 2));
      canvas.drawCircle(bar, 0.03 * g.scale, Paint()..color = tokens.line);
    }
    // In ink, over the body (red is for force and torque). At parallel it
    // lies along the thigh, so a paper underlay keeps the dashes apart.
    final lever = g.lever;
    if (lever != null) {
      _dashed(canvas, lever, stroke(tokens.panel, 5));
      _dashed(canvas, lever, stroke(tokens.line, 2));
    }
    // The ground's arrow is ink and rises through the shank, so it goes over
    // the body on a paper underlay, like the lever.
    _arrow(canvas, g.ground, underlay: true);
    final barForce = g.barForce;
    if (barForce != null) _arrow(canvas, barForce);

    for (final joint in [g.hip, g.knee, g.ankle]) {
      canvas.drawCircle(joint, 3, Paint()..color = tokens.panel);
      canvas.drawCircle(joint, 3, stroke(tokens.line, 1.5));
    }
  }

  void _dashed(Canvas canvas, Segment s, Paint paint) {
    const dash = 5.0, gap = 4.0;
    final (a, b) = s;
    final length = (b - a).distance;
    if (length == 0) return;
    final step = (b - a) / length;
    for (var d = 0.0; d < length; d += dash + gap) {
      canvas.drawLine(
          a + step * d, a + step * math.min(d + dash, length), paint);
    }
  }

  /// Strokes [v]'s shaft and head in its heat, or in ink when it has none;
  /// with [underlay], a wider paper stroke first so it reads over the body.
  void _arrow(Canvas canvas, ForceVector v, {bool underlay = false}) {
    if (v.length == 0) return;
    Paint pen(Color color, double width) => Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = width
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;
    final ink = switch (v.heat) {
      null => tokens.line,
      final heat => tokens.heat(heat),
    };
    final back = (v.from - v.to) / v.length * 9;
    for (final paint in [if (underlay) pen(tokens.panel, 6), pen(ink, 3)]) {
      canvas.drawLine(v.from, v.to, paint);
      for (final turn in [-0.5, 0.5]) {
        final c = math.cos(turn), s = math.sin(turn);
        canvas.drawLine(
            v.to,
            v.to + Offset(back.dx * c - back.dy * s, back.dx * s + back.dy * c),
            paint);
      }
    }
  }

  @override
  bool shouldRepaint(SquatFigurePainter old) =>
      !identical(old.current, current) ||
      !identical(old.range, range) ||
      old.tokens != tokens;
}
