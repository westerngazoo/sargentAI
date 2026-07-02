import 'package:flutter/material.dart';

/// The fitAI brand mark shown on the auth screens and splash: a gradient
/// roundel with the dumbbell glyph, the wordmark, and a tagline.
class BrandHeader extends StatelessWidget {
  const BrandHeader({super.key, this.compact = false});

  /// Compact drops the tagline (used where vertical space is tight).
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Column(
      children: [
        Container(
          width: 84,
          height: 84,
          decoration: BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: [cs.primary, cs.tertiary],
            ),
            borderRadius: BorderRadius.circular(26),
            boxShadow: [
              BoxShadow(
                color: cs.primary.withValues(alpha: 0.35),
                blurRadius: 24,
                offset: const Offset(0, 8),
              ),
            ],
          ),
          child: Icon(Icons.fitness_center, size: 40, color: cs.onPrimary),
        ),
        const SizedBox(height: 16),
        Text(
          'fitAI',
          style: Theme.of(context).textTheme.headlineMedium,
        ),
        if (!compact) ...[
          const SizedBox(height: 4),
          Text(
            'Train smarter. Every session, personalized.',
            textAlign: TextAlign.center,
            style: Theme.of(context)
                .textTheme
                .bodyMedium
                ?.copyWith(color: cs.onSurfaceVariant),
          ),
        ],
      ],
    );
  }
}
