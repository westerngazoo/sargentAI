// The target-physique silhouette for a program's archetype — a parametric
// front-view build (shoulder width, waist, muscularity) rendered as a
// gradient-filled figure. "This is what this program builds toward."

import 'package:flutter/material.dart';

import '../../core/theme/app_theme.dart';

/// Physique shape parameters, 0..1 relative to the canvas.
class PhysiqueParams {
  const PhysiqueParams({
    required this.shoulder,
    required this.waist,
    required this.muscularity,
    required this.blurb,
  });

  /// Half-width of the shoulders (the V's top).
  final double shoulder;

  /// Half-width of the waist (the V's bottom) — smaller = harder taper.
  final double waist;

  /// Limb/muscle thickness 0.8 (lean) .. 1.25 (mass).
  final double muscularity;

  /// One-line description of the target look.
  final String blurb;

  double get taper => shoulder / waist;
}

/// Archetype id → target physique. Hand-authored to match each program's
/// aesthetic (V-taper, mass, powerbuilder…).
const Map<String, PhysiqueParams> _byArchetype = {
  'classic-aesthetic-taper': PhysiqueParams(
    shoulder: 0.40,
    waist: 0.155,
    muscularity: 1.0,
    blurb: 'Wide shoulders, small waist — the golden-era X-frame.',
  ),
  'modern-precision-hypertrophy': PhysiqueParams(
    shoulder: 0.38,
    waist: 0.17,
    muscularity: 1.05,
    blurb: 'Balanced, detailed, athletic — proportion over pure size.',
  ),
  'high-intensity-minimalist': PhysiqueParams(
    shoulder: 0.36,
    waist: 0.175,
    muscularity: 0.95,
    blurb: 'Lean and hard — dense muscle, minimal excess.',
  ),
  'powerbuilder-leverage': PhysiqueParams(
    shoulder: 0.40,
    waist: 0.225,
    muscularity: 1.18,
    blurb: 'Thick and powerful — a strongman-meets-bodybuilder build.',
  ),
  'heavy-duty-mass': PhysiqueParams(
    shoulder: 0.41,
    waist: 0.215,
    muscularity: 1.2,
    blurb: 'Dense, heavy-duty mass — maximum muscle, blocky and strong.',
  ),
  'mass-monster-volume': PhysiqueParams(
    shoulder: 0.44,
    waist: 0.24,
    muscularity: 1.25,
    blurb: 'Maximum size — the mass-monster silhouette.',
  ),
};

PhysiqueParams physiqueFor(String archetypeId) =>
    _byArchetype[archetypeId] ??
    const PhysiqueParams(
      shoulder: 0.38,
      waist: 0.18,
      muscularity: 1.0,
      blurb: 'Your program\'s target build.',
    );

/// The hero "target physique" card for the program screen.
class TargetPhysique extends StatelessWidget {
  const TargetPhysique({super.key, required this.archetypeId});

  final String archetypeId;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final p = physiqueFor(archetypeId);
    return Container(
      decoration: BoxDecoration(
        color: cs.surfaceContainerLow,
        borderRadius: BorderRadius.circular(24),
        border: Border.all(color: cs.outlineVariant.withValues(alpha: 0.4)),
      ),
      padding: const EdgeInsets.all(18),
      child: Row(
        children: [
          SizedBox(
            width: 96,
            height: 150,
            child: CustomPaint(painter: _PhysiquePainter(p)),
          ),
          const SizedBox(width: 18),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'TARGET PHYSIQUE',
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                        letterSpacing: 1.2,
                        fontWeight: FontWeight.w700,
                        color: cs.onSurfaceVariant,
                      ),
                ),
                const SizedBox(height: 6),
                Text(p.blurb, style: Theme.of(context).textTheme.bodyMedium),
                const SizedBox(height: 12),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    _tag(context, Icons.open_in_full,
                        'Taper ${p.taper.toStringAsFixed(1)}:1'),
                    _tag(
                      context,
                      Icons.fitness_center,
                      p.muscularity >= 1.15
                          ? 'Mass'
                          : (p.muscularity <= 0.97 ? 'Lean' : 'Balanced'),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _tag(BuildContext context, IconData icon, String label) {
    final cs = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        color: cs.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 13, color: cs.primary),
          const SizedBox(width: 5),
          Text(label, style: Theme.of(context).textTheme.labelSmall),
        ],
      ),
    );
  }
}

class _PhysiquePainter extends CustomPainter {
  const _PhysiquePainter(this.p);
  final PhysiqueParams p;

  @override
  void paint(Canvas canvas, Size size) {
    final w = size.width;
    final h = size.height;
    final cx = w / 2;
    final m = p.muscularity;

    // Vertical anchors (fractions of height).
    final headTop = h * 0.02;
    final headR = h * 0.075 * m.clamp(0.9, 1.15);
    final neckY = headTop + headR * 2;
    final shoulderY = h * 0.24;
    final chestY = h * 0.36;
    final waistY = h * 0.52;
    final hipY = h * 0.58;
    final kneeY = h * 0.80;
    final footY = h * 0.99;

    final shoulderX = w * p.shoulder * m;
    final chestX = w * (p.shoulder * 0.86) * m;
    final waistX = w * p.waist;
    final hipX = w * (p.waist + 0.03) * m;
    final thighX = w * (p.waist * 0.62 + 0.02) * m;
    final ankleX = w * 0.045;

    // Torso path (right half mirrored) — shoulders → chest → waist → hip.
    final torso = Path()
      ..moveTo(cx, neckY)
      ..lineTo(cx + shoulderX, shoulderY)
      ..quadraticBezierTo(cx + chestX, chestY, cx + waistX, waistY)
      ..lineTo(cx + hipX, hipY)
      // right leg
      ..quadraticBezierTo(cx + hipX, kneeY * 0.78, cx + thighX, kneeY)
      ..lineTo(cx + ankleX * 1.6, footY)
      ..lineTo(cx + ankleX * 0.2, footY)
      ..lineTo(cx + w * 0.02, hipY + h * 0.02)
      // crotch up center then down left leg
      ..lineTo(cx - w * 0.02, hipY + h * 0.02)
      ..lineTo(cx - ankleX * 0.2, footY)
      ..lineTo(cx - ankleX * 1.6, footY)
      ..lineTo(cx - thighX, kneeY)
      ..quadraticBezierTo(cx - hipX, kneeY * 0.78, cx - hipX, hipY)
      ..lineTo(cx - waistX, waistY)
      ..quadraticBezierTo(cx - chestX, chestY, cx - shoulderX, shoulderY)
      ..close();

    final grad = AppTheme.gradEnd;
    final grad2 = AppTheme.gradStart;
    final fill = Paint()
      ..shader = LinearGradient(
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
        colors: [grad2, grad],
      ).createShader(Offset.zero & size);

    // Arms (behind torso): shoulder → elbow → forearm, slight outward bulge.
    final armPaint = Paint()
      ..color = grad
      ..style = PaintingStyle.stroke
      ..strokeWidth = w * 0.085 * m
      ..strokeCap = StrokeCap.round;
    for (final s in [1, -1]) {
      final ax = cx + s * shoulderX;
      final arm = Path()
        ..moveTo(ax, shoulderY + h * 0.01)
        ..quadraticBezierTo(ax + s * w * 0.10 * m, chestY, ax + s * w * 0.02,
            waistY - h * 0.02);
      canvas.drawPath(arm, armPaint);
    }

    canvas.drawPath(torso, fill);

    // Head.
    canvas.drawCircle(
        Offset(cx, headTop + headR), headR, Paint()..color = grad);

    // A soft centre line for the physique's definition.
    final line = Paint()
      ..color = Colors.white.withValues(alpha: 0.14)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.4;
    canvas.drawLine(Offset(cx, chestY - h * 0.02), Offset(cx, waistY), line);
  }

  @override
  bool shouldRepaint(_PhysiquePainter old) =>
      old.p.shoulder != p.shoulder ||
      old.p.waist != p.waist ||
      old.p.muscularity != p.muscularity;
}
