// Custom-painted trend visuals — no chart dependency. A full line chart with
// an area fill and min/max labels, and a tiny inline sparkline. Each series is
// auto-scaled to its own range so shape (the trend) reads clearly.

import 'dart:math' as math;

import 'package:flutter/material.dart';

/// One line on a chart: a colour and its (x-ordered) y-values.
class ChartSeries {
  const ChartSeries({
    required this.label,
    required this.color,
    required this.values,
    this.unit = '',
  });

  final String label;
  final Color color;
  final List<double> values;
  final String unit;
}

/// A line chart auto-scaled to the combined min/max of all series. Best when
/// series share a unit; for mixed units prefer separate charts or [Sparkline].
class TrendChart extends StatelessWidget {
  const TrendChart({
    super.key,
    required this.series,
    this.height = 180,
  });

  final List<ChartSeries> series;
  final double height;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return SizedBox(
      height: height,
      child: CustomPaint(
        size: Size.infinite,
        painter: _ChartPainter(
          series: series,
          grid: cs.outlineVariant.withValues(alpha: 0.5),
          labelColor: cs.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _ChartPainter extends CustomPainter {
  _ChartPainter({
    required this.series,
    required this.grid,
    required this.labelColor,
  });

  final List<ChartSeries> series;
  final Color grid;
  final Color labelColor;

  static const _padL = 8.0;
  static const _padR = 8.0;
  static const _padT = 10.0;
  static const _padB = 6.0;

  @override
  void paint(Canvas canvas, Size size) {
    final plot = Rect.fromLTRB(
      _padL,
      _padT,
      size.width - _padR,
      size.height - _padB,
    );

    // Baseline grid.
    final gridPaint = Paint()
      ..color = grid
      ..strokeWidth = 0.7;
    for (var i = 0; i <= 3; i++) {
      final y = plot.top + plot.height * i / 3;
      canvas.drawLine(Offset(plot.left, y), Offset(plot.right, y), gridPaint);
    }

    for (final s in series) {
      if (s.values.length < 2) continue;
      final lo = s.values.reduce(math.min);
      final hi = s.values.reduce(math.max);
      final span = (hi - lo).abs() < 1e-9 ? 1.0 : hi - lo;

      Offset pt(int i) {
        final x = plot.left + plot.width * i / (s.values.length - 1);
        // Pad 8% top/bottom so the line never hugs the edge.
        final norm = (s.values[i] - lo) / span;
        final y = plot.bottom - (0.08 + norm * 0.84) * plot.height;
        return Offset(x, y);
      }

      final path = Path()..moveTo(pt(0).dx, pt(0).dy);
      for (var i = 1; i < s.values.length; i++) {
        path.lineTo(pt(i).dx, pt(i).dy);
      }

      // Area fill under the line.
      final fill = Path.from(path)
        ..lineTo(plot.right, plot.bottom)
        ..lineTo(plot.left, plot.bottom)
        ..close();
      canvas.drawPath(
        fill,
        Paint()..color = s.color.withValues(alpha: 0.12),
      );
      // The line.
      canvas.drawPath(
        path,
        Paint()
          ..color = s.color
          ..style = PaintingStyle.stroke
          ..strokeWidth = 2.4
          ..strokeCap = StrokeCap.round
          ..strokeJoin = StrokeJoin.round,
      );
      // End dot.
      canvas.drawCircle(pt(s.values.length - 1), 3.5, Paint()..color = s.color);
    }
  }

  @override
  bool shouldRepaint(_ChartPainter old) => old.series != series;
}

/// A tiny inline trend line for a metric tile.
class Sparkline extends StatelessWidget {
  const Sparkline({
    super.key,
    required this.values,
    required this.color,
    this.width = 72,
    this.height = 28,
  });

  final List<double> values;
  final Color color;
  final double width;
  final double height;

  @override
  Widget build(BuildContext context) => SizedBox(
        width: width,
        height: height,
        child: CustomPaint(painter: _SparkPainter(values, color)),
      );
}

class _SparkPainter extends CustomPainter {
  _SparkPainter(this.values, this.color);
  final List<double> values;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    if (values.length < 2) return;
    final lo = values.reduce(math.min);
    final hi = values.reduce(math.max);
    final span = (hi - lo).abs() < 1e-9 ? 1.0 : hi - lo;
    Offset pt(int i) {
      final x = size.width * i / (values.length - 1);
      final y = size.height - ((values[i] - lo) / span) * size.height;
      return Offset(x, y.clamp(1.5, size.height - 1.5));
    }

    final path = Path()..moveTo(pt(0).dx, pt(0).dy);
    for (var i = 1; i < values.length; i++) {
      path.lineTo(pt(i).dx, pt(i).dy);
    }
    canvas.drawPath(
      path,
      Paint()
        ..color = color
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round,
    );
    canvas.drawCircle(pt(values.length - 1), 2.5, Paint()..color = color);
  }

  @override
  bool shouldRepaint(_SparkPainter old) => old.values != values;
}
