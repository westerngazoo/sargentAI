// Progress visuals for the weekly plan target: a circular ring with the
// percentage inside, and the full "this week" card used on the plan screen.

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';
import '../application/program_progress.dart';

/// A circular progress ring with the percentage centred. Themed olive track
/// + brand-gradient sweep.
class ProgressRing extends StatelessWidget {
  const ProgressRing({
    super.key,
    required this.ratio,
    required this.size,
    this.label,
    this.stroke = 8,
  });

  final double ratio;
  final double size;

  /// Centre label; defaults to the rounded percentage.
  final String? label;
  final double stroke;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final track = cs.surfaceContainerHighest;
    final sweep = cs.primary;
    final ink = cs.onSurface;
    return SizedBox(
      width: size,
      height: size,
      child: CustomPaint(
        painter: _RingPainter(
          ratio: ratio.clamp(0.0, 1.0),
          track: track,
          sweep: sweep,
          stroke: stroke,
        ),
        child: Center(
          child: Text(
            label ??
                AppLocalizations.of(context).percent((ratio * 100).round()),
            style: TextStyle(
              color: ink,
              fontWeight: FontWeight.w800,
              fontSize: size * 0.26,
              letterSpacing: -0.5,
            ),
          ),
        ),
      ),
    );
  }
}

class _RingPainter extends CustomPainter {
  const _RingPainter({
    required this.ratio,
    required this.track,
    required this.sweep,
    required this.stroke,
  });

  final double ratio;
  final Color track;
  final Color sweep;
  final double stroke;

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    final center = rect.center;
    final radius = (size.shortestSide - stroke) / 2;
    final trackPaint = Paint()
      ..color = track
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke;
    canvas.drawCircle(center, radius, trackPaint);

    if (ratio <= 0) return;
    final sweepPaint = Paint()
      ..color = sweep
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke
      ..strokeCap = StrokeCap.round;
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: radius),
      -math.pi / 2,
      2 * math.pi * ratio,
      false,
      sweepPaint,
    );
  }

  @override
  bool shouldRepaint(_RingPainter old) =>
      old.ratio != ratio || old.sweep != sweep || old.track != track;
}

/// The full "this week" progress card for the plan screen.
class WeeklyProgressCard extends StatelessWidget {
  const WeeklyProgressCard({super.key, required this.progress});

  final WeeklyProgress progress;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Row(
          children: [
            ProgressRing(ratio: progress.ratio, size: 84),
            const SizedBox(width: 18),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Text(
                        l10n.thisWeekCaps,
                        style: Theme.of(context).textTheme.labelSmall?.copyWith(
                              letterSpacing: 1.2,
                              fontWeight: FontWeight.w700,
                              color: cs.onSurfaceVariant,
                            ),
                      ),
                      const Spacer(),
                      if (progress.weekComplete) _CompleteBadge(cs: cs),
                    ],
                  ),
                  const SizedBox(height: 6),
                  Text(
                    l10n.daysOfTarget(progress.daysDone, progress.daysTarget),
                    style: Theme.of(context)
                        .textTheme
                        .titleLarge
                        ?.copyWith(fontWeight: FontWeight.w800),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    l10n.sessionsLoggedOnProgram(progress.totalSessions),
                    style: Theme.of(context)
                        .textTheme
                        .bodySmall
                        ?.copyWith(color: cs.onSurfaceVariant),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _CompleteBadge extends StatelessWidget {
  const _CompleteBadge({required this.cs});

  final ColorScheme cs;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: cs.primaryContainer,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.check_circle, size: 14, color: cs.onPrimaryContainer),
          const SizedBox(width: 4),
          Text(
            AppLocalizations.of(context).targetMet,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  color: cs.onPrimaryContainer,
                  fontWeight: FontWeight.w700,
                ),
          ),
        ],
      ),
    );
  }
}
