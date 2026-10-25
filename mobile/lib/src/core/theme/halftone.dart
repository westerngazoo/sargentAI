import 'package:flutter/material.dart';

import 'tokens.dart';

/// The faint halftone dot grid behind the main screen and the panels
/// (SPEC-0061 §2.10): the reels' paper, in the theme's ink at low alpha.
class HalftoneBackground extends StatelessWidget {
  const HalftoneBackground({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).extension<FbfTokens>() ?? FbfTokens.light;
    return CustomPaint(
      painter: HalftonePainter(ink: t.line, paper: t.ground),
      child: child,
    );
  }
}

/// Dots every [spacing] logical pixels, [radius] wide, at [alpha] of [ink]
/// over a solid [paper] fill. Pure drawing; the test reads these fields.
class HalftonePainter extends CustomPainter {
  const HalftonePainter({
    required this.ink,
    required this.paper,
    this.spacing = 16,
    this.radius = 0.9,
    this.alpha = 0.07,
  });

  final Color ink;
  final Color paper;
  final double spacing;
  final double radius;
  final double alpha;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = paper);
    final dot = Paint()..color = ink.withValues(alpha: alpha);
    for (var y = spacing / 2; y < size.height; y += spacing) {
      for (var x = spacing / 2; x < size.width; x += spacing) {
        canvas.drawCircle(Offset(x, y), radius, dot);
      }
    }
  }

  @override
  bool shouldRepaint(HalftonePainter old) =>
      old.ink != ink ||
      old.paper != paper ||
      old.spacing != spacing ||
      old.radius != radius ||
      old.alpha != alpha;
}
