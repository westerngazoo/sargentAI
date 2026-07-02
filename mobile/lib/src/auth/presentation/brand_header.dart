import 'package:flutter/material.dart';

import '../../core/brand.dart';
import '../../core/theme/app_theme.dart';

/// The Sargent AI brand mark: sergeant rank chevrons on the brand-gradient
/// badge, the stenciled wordmark, and a tagline. Sizes itself relative to
/// the available width so it reads well from phones to desktop.
class BrandHeader extends StatelessWidget {
  const BrandHeader({super.key, this.compact = false});

  /// Compact drops the tagline and shrinks the badge (splash, tight spots).
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return LayoutBuilder(
      builder: (context, constraints) {
        final bound = constraints.hasBoundedWidth
            ? constraints.maxWidth
            : MediaQuery.sizeOf(context).width;
        final badge =
            (bound * 0.26).clamp(64.0, compact ? 80.0 : 104.0).toDouble();
        return Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: badge,
              height: badge,
              decoration: BoxDecoration(
                gradient: brandGradient(),
                borderRadius: BorderRadius.circular(badge * 0.30),
                boxShadow: [
                  BoxShadow(
                    color: AppTheme.gradEnd.withValues(alpha: 0.4),
                    blurRadius: 26,
                    offset: const Offset(0, 10),
                  ),
                ],
              ),
              child: const CustomPaint(
                painter: _ChevronsPainter(color: Colors.white),
              ),
            ),
            SizedBox(height: badge * 0.18),
            Text(
              Brand.appName.toUpperCase(),
              style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                    letterSpacing: 3.5,
                    fontWeight: FontWeight.w800,
                  ),
            ),
            if (!compact) ...[
              const SizedBox(height: 6),
              Text(
                Brand.tagline,
                textAlign: TextAlign.center,
                style: Theme.of(context)
                    .textTheme
                    .bodyMedium
                    ?.copyWith(color: cs.onSurfaceVariant),
              ),
            ],
          ],
        );
      },
    );
  }
}

/// Three upward sergeant chevrons — the rank insignia as the logo.
class _ChevronsPainter extends CustomPainter {
  const _ChevronsPainter({required this.color});

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = size.height * 0.11
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;
    for (var i = 0; i < 3; i++) {
      final baseY = size.height * (0.40 + 0.20 * i);
      final path = Path()
        ..moveTo(size.width * 0.24, baseY)
        ..lineTo(size.width * 0.50, baseY - size.height * 0.17)
        ..lineTo(size.width * 0.76, baseY);
      canvas.drawPath(path, paint);
    }
  }

  @override
  bool shouldRepaint(_ChevronsPainter oldDelegate) =>
      color != oldDelegate.color;
}
