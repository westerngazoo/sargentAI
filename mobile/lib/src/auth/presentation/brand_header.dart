import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';

import '../../core/brand.dart';

/// The gradient goose roundel — the Goose Fitness Physics logo, reusable at
/// any size (brand header, splash, app bars). A goose on the brand gradient.
class BrandBadge extends StatelessWidget {
  const BrandBadge({super.key, required this.size, this.shadow = true});

  final double size;
  final bool shadow;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.primary,
        borderRadius: BorderRadius.circular(size * 0.30),
        boxShadow: shadow
            ? [
                BoxShadow(
                  color: Theme.of(context).colorScheme.outline,
                  offset: Offset(size * 0.06, size * 0.06),
                ),
              ]
            : null,
      ),
      child: CustomPaint(
        painter: _GoosePainter(color: Theme.of(context).colorScheme.onPrimary),
      ),
    );
  }
}

/// The full brand mark: badge + stenciled wordmark + tagline. Sizes itself
/// relative to available width so it reads from phones to desktop.
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
            BrandBadge(size: badge),
            SizedBox(height: badge * 0.18),
            // Scale-to-fit rather than trusting the name to be short: the
            // wordmark is rendered uppercased at letterSpacing 3.5, so a
            // longer product name silently overflows on a narrow phone. The
            // seam exists to make renaming safe, so the render has to be safe
            // for any name it is given.
            FittedBox(
              fit: BoxFit.scaleDown,
              child: Text(
                Brand.appName.toUpperCase(),
                maxLines: 1,
                softWrap: false,
                style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                      letterSpacing: 3.5,
                      fontWeight: FontWeight.w800,
                    ),
              ),
            ),
            if (!compact) ...[
              const SizedBox(height: 6),
              Text(
                AppLocalizations.of(context).appTagline,
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

/// A swimming goose facing right: one filled silhouette, with the eye and the
/// wing line cut out so the badge gradient shows through.
///
/// Drawn on a 100×100 grid scaled to the canvas. `tool/brand/goose-badge.svg`
/// is the same shape, used to render the web and launcher icons; change both
/// together.
class _GoosePainter extends CustomPainter {
  const _GoosePainter({required this.color});

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final unit = size.shortestSide / 100;
    canvas
      ..save()
      ..scale(unit);
    final bounds = Offset.zero & const Size(100, 100);
    canvas.saveLayer(bounds, Paint());
    canvas.drawPath(_silhouette, Paint()..color = color);
    final cut = Paint()..blendMode = BlendMode.clear;
    canvas
      ..drawCircle(const Offset(66.5, 20.5), 2.6, cut)
      ..drawPath(
        _wing,
        cut
          ..style = PaintingStyle.stroke
          ..strokeWidth = 3.2
          ..strokeCap = StrokeCap.round,
      )
      ..restore()
      ..restore();
  }

  static final Path _silhouette = Path()
    ..moveTo(14, 54)
    ..cubicTo(26, 52, 38, 55, 47, 59)
    ..cubicTo(54, 62, 58, 57, 57, 48)
    ..cubicTo(56, 39, 51, 31, 55, 23)
    ..cubicTo(58, 15, 69, 12, 74, 19)
    ..lineTo(87, 23)
    ..lineTo(86, 26)
    ..lineTo(74, 28)
    ..cubicTo(69, 31, 66, 37, 67, 46)
    ..cubicTo(68, 56, 74, 62, 75, 70)
    ..cubicTo(76, 82, 62, 89, 45, 88)
    ..cubicTo(30, 87, 20, 79, 18, 68)
    ..cubicTo(17, 63, 15, 59, 14, 54)
    ..close();

  static final Path _wing = Path()
    ..moveTo(29, 66)
    ..cubicTo(41, 63, 53, 68, 60, 77);

  @override
  bool shouldRepaint(_GoosePainter oldDelegate) => color != oldDelegate.color;
}
