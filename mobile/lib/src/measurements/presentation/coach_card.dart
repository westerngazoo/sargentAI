// R-0017 AC8 — the "Coach suggestions" card. Display-only in v1: each
// suggestion shows its severity and plain-language rationale; the positive
// empty state says "on track" instead of showing nothing. Hidden entirely when
// the user has no active program (nothing to coach against) or on error.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/theme/app_theme.dart';
import '../models/coach_suggestion.dart';
import '../services/adjustment_service.dart';

class CoachCard extends ConsumerWidget {
  const CoachCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final advice = ref.watch(coachAdviceProvider);
    return advice.when(
      loading: () => const SizedBox.shrink(),
      error: (_, __) => const SizedBox.shrink(),
      data: (a) =>
          !a.hasProgram ? const SizedBox.shrink() : _CoachBody(advice: a),
    );
  }
}

class _CoachBody extends StatelessWidget {
  const _CoachBody({required this.advice});

  final CoachAdvice advice;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Container(
      // Bottom margin lives here so the gap collapses when the card is hidden.
      margin: const EdgeInsets.only(bottom: 24),
      decoration: BoxDecoration(
        color: cs.surfaceContainerLow,
        borderRadius: BorderRadius.circular(20),
        border: Border.all(color: cs.outlineVariant.withValues(alpha: 0.4)),
      ),
      padding: const EdgeInsets.fromLTRB(18, 16, 18, 18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 34,
                height: 34,
                decoration: BoxDecoration(
                  gradient: const LinearGradient(
                    colors: [AppTheme.gradStart, AppTheme.gradEnd],
                  ),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: const Icon(Icons.military_tech,
                    size: 19, color: Colors.white),
              ),
              const SizedBox(width: 12),
              Text(
                'Coach suggestions',
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
              ),
              const Spacer(),
              Text(
                'last ${advice.windowWeeks} wks',
                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                      color: cs.onSurfaceVariant,
                    ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          if (advice.suggestions.isEmpty)
            Row(
              children: [
                Icon(Icons.check_circle_outline, size: 18, color: cs.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    "On track — keep doing what you're doing.",
                    style: Theme.of(context).textTheme.bodyMedium,
                  ),
                ),
              ],
            )
          else
            ...advice.suggestions.map((s) => _SuggestionTile(suggestion: s)),
        ],
      ),
    );
  }
}

class _SuggestionTile extends StatelessWidget {
  const _SuggestionTile({required this.suggestion});

  final CoachSuggestion suggestion;

  IconData get _icon => switch (suggestion.kind) {
        'deload_lift' => Icons.south,
        'progress_lift' => Icons.north,
        'add_weekly_sets' => Icons.add_circle_outline,
        'reduce_days_per_week' => Icons.event_available,
        'adjust_kcal' => Icons.restaurant,
        _ => Icons.tips_and_updates,
      };

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final accent = suggestion.isAction ? cs.primary : cs.onSurfaceVariant;
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 28,
            height: 28,
            decoration: BoxDecoration(
              color: accent.withValues(alpha: 0.12),
              borderRadius: BorderRadius.circular(9),
            ),
            child: Icon(_icon, size: 16, color: accent),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (suggestion.isAction)
                  Text(
                    'ACTION',
                    style: Theme.of(context).textTheme.labelSmall?.copyWith(
                          letterSpacing: 1.1,
                          fontWeight: FontWeight.w700,
                          color: cs.primary,
                        ),
                  ),
                Text(
                  suggestion.rationale,
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
