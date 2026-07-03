// R-0014 / SPEC-0014 §2.5.3 — ProgramDetailScreen.
//
// Displays the active program and diet. Reached from ProgramProposalsScreen
// after a successful choose, or via the home shortcut (CurrentProgramCard).

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/theme/app_theme.dart';
import '../application/program_progress.dart';
import '../application/program_providers.dart';
import '../models/program_proposal.dart';
import '../models/user_program.dart';
import 'progress_ring.dart';
import 'target_physique.dart';

class ProgramDetailScreen extends ConsumerWidget {
  const ProgramDetailScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final async = ref.watch(currentProgramProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('Your program')),
      body: async.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Text('Could not load your program'),
              TextButton(
                onPressed: () => ref.invalidate(currentProgramProvider),
                child: const Text('Retry'),
              ),
            ],
          ),
        ),
        data: (program) => program == null
            ? const Center(child: Text('No program yet'))
            : _ProgramDetail(program: program),
      ),
    );
  }
}

class _ProgramDetail extends ConsumerWidget {
  const _ProgramDetail({required this.program});

  final UserProgram program;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final p = program.program;
    final d = program.diet;
    final progress = ref.watch(weeklyProgressProvider);

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(_displayTitle(program.archetypeId),
            style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 16),
        TargetPhysique(archetypeId: program.archetypeId),
        const SizedBox(height: 16),
        if (progress != null) ...[
          WeeklyProgressCard(progress: progress),
          const SizedBox(height: 16),
        ],
        _Section(
          title: 'Training',
          icon: Icons.fitness_center,
          children: [
            _InfoRow('Split', p.split),
            _InfoRow('Training', '${p.daysPerWeek} days/week'),
            _InfoRow(
                'Session duration', '${p.estimatedSessionDurationMin} min'),
            _InfoRow('Intensity', p.intensityGuidance),
            _InfoRow('Rest', p.restGuidance),
            _InfoRow('Progression', p.progressionGuidance, last: true),
          ],
        ),
        const SizedBox(height: 16),
        _Section(
          title: 'Highlight exercises',
          icon: Icons.star_outline,
          children: [
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: p.highlightExercises
                  .map((e) => _ExercisePill(label: e))
                  .toList(),
            ),
          ],
        ),
        const SizedBox(height: 16),
        _Section(
          title: 'Nutrition',
          icon: Icons.restaurant_menu,
          children: [
            _InfoRow('Approach', d.approach),
            _InfoRow('Strategy', d.calorieStrategy),
            _InfoRow('Meals', d.mealStructure, last: true),
            const SizedBox(height: 16),
            _MacroTable(diet: d),
          ],
        ),
      ],
    );
  }
}

/// `classic-aesthetic-taper` → `Classic Aesthetic Taper` for display.
String _displayTitle(String slug) => slug
    .split('-')
    .map((w) => w.isEmpty ? w : '${w[0].toUpperCase()}${w.substring(1)}')
    .join(' ');

class _Section extends StatelessWidget {
  const _Section({
    required this.title,
    required this.icon,
    required this.children,
  });

  final String title;
  final IconData icon;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Container(
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
                child: Icon(icon, size: 19, color: Colors.white),
              ),
              const SizedBox(width: 12),
              Text(title,
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                      )),
            ],
          ),
          const SizedBox(height: 14),
          ...children,
        ],
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow(this.label, this.value, {this.last = false});

  final String label;
  final String value;
  final bool last;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 9),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SizedBox(
                width: 130,
                child: Text(
                  label,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: cs.onSurfaceVariant,
                      ),
                ),
              ),
              Expanded(
                child: Text(value,
                    style: Theme.of(context)
                        .textTheme
                        .bodyMedium
                        ?.copyWith(fontWeight: FontWeight.w600)),
              ),
            ],
          ),
        ),
        if (!last)
          Divider(height: 1, color: cs.outlineVariant.withValues(alpha: 0.35)),
      ],
    );
  }
}

class _ExercisePill extends StatelessWidget {
  const _ExercisePill({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 9),
      decoration: BoxDecoration(
        color: cs.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: cs.outlineVariant.withValues(alpha: 0.5)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.bolt, size: 15, color: cs.primary),
          const SizedBox(width: 6),
          Text(label, style: Theme.of(context).textTheme.bodyMedium),
        ],
      ),
    );
  }
}

/// Macro breakdown: a proportional protein/carbs/fat bar (by calories) plus a
/// pill per macro and the daily energy total.
class _MacroTable extends StatelessWidget {
  const _MacroTable({required this.diet});

  final GeneratedDiet diet;

  static const _protein = Color(0xFF4CAF50);
  static const _carbs = Color(0xFFFFB74D);
  static const _fat = Color(0xFF9C7BF0);

  @override
  Widget build(BuildContext context) {
    // Energy share drives the bar: 4/4/9 kcal per gram.
    final pKcal = diet.proteinG * 4.0;
    final cKcal = diet.carbsG * 4.0;
    final fKcal = diet.fatG * 9.0;
    final total = (pKcal + cKcal + fKcal).clamp(1.0, double.infinity);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        ClipRRect(
          borderRadius: BorderRadius.circular(999),
          child: Row(
            children: [
              Expanded(
                flex: (pKcal / total * 1000).round().clamp(1, 1000),
                child: Container(height: 12, color: _protein),
              ),
              Expanded(
                flex: (cKcal / total * 1000).round().clamp(1, 1000),
                child: Container(height: 12, color: _carbs),
              ),
              Expanded(
                flex: (fKcal / total * 1000).round().clamp(1, 1000),
                child: Container(height: 12, color: _fat),
              ),
            ],
          ),
        ),
        const SizedBox(height: 14),
        Row(
          children: [
            _macroPill('Protein', '${diet.proteinG} g', _protein, context),
            const SizedBox(width: 10),
            _macroPill('Carbs', '${diet.carbsG} g', _carbs, context),
            const SizedBox(width: 10),
            _macroPill('Fat', '${diet.fatG} g', _fat, context),
          ],
        ),
        const SizedBox(height: 14),
        Row(
          children: [
            Icon(Icons.local_fire_department,
                size: 18, color: Theme.of(context).colorScheme.primary),
            const SizedBox(width: 6),
            Text('${diet.estimatedKcal}',
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                    )),
            const SizedBox(width: 4),
            Text('kcal / day',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    )),
          ],
        ),
      ],
    );
  }

  Widget _macroPill(
          String label, String value, Color color, BuildContext context) =>
      Expanded(
        child: Container(
          padding: const EdgeInsets.symmetric(vertical: 12, horizontal: 12),
          decoration: BoxDecoration(
            color: color.withValues(alpha: 0.12),
            borderRadius: BorderRadius.circular(14),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    width: 8,
                    height: 8,
                    decoration:
                        BoxDecoration(color: color, shape: BoxShape.circle),
                  ),
                  const SizedBox(width: 6),
                  Text(label,
                      style: Theme.of(context).textTheme.labelSmall?.copyWith(
                            color:
                                Theme.of(context).colorScheme.onSurfaceVariant,
                          )),
                ],
              ),
              const SizedBox(height: 4),
              Text(value,
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                      )),
            ],
          ),
        ),
      );
}

/// Home-screen shortcut card — taps through to [ProgramDetailScreen].
///
/// When no program exists (404) or an error occurs, shows a "Get your program"
/// CTA that navigates toward the match flow (SPEC-0014 §2.5.4).
class CurrentProgramCard extends ConsumerWidget {
  const CurrentProgramCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final async = ref.watch(currentProgramProvider);
    return async.when(
      loading: () => const LinearProgressIndicator(),
      error: (_, __) => _getProgramCta(context),
      data: (program) {
        if (program == null) return _getProgramCta(context);
        final progress = ref.watch(weeklyProgressProvider);
        return Container(
          margin: const EdgeInsets.symmetric(horizontal: 16),
          decoration: BoxDecoration(
            gradient: brandGradient(),
            borderRadius: BorderRadius.circular(24),
          ),
          child: Material(
            color: Colors.transparent,
            child: InkWell(
              borderRadius: BorderRadius.circular(24),
              onTap: () => context.go('/programs/current'),
              child: Padding(
                padding: const EdgeInsets.all(20),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Text(
                          'YOUR PROGRAM',
                          style:
                              Theme.of(context).textTheme.labelSmall?.copyWith(
                                    color: Colors.white.withValues(alpha: 0.8),
                                    letterSpacing: 1.2,
                                    fontWeight: FontWeight.w700,
                                  ),
                        ),
                        const Spacer(),
                        Icon(Icons.arrow_forward, color: Colors.white),
                      ],
                    ),
                    const SizedBox(height: 10),
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.center,
                      children: [
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                program.program.split,
                                maxLines: 2,
                                overflow: TextOverflow.ellipsis,
                                style: Theme.of(context)
                                    .textTheme
                                    .titleMedium
                                    ?.copyWith(color: Colors.white),
                              ),
                              const SizedBox(height: 14),
                              Wrap(
                                spacing: 8,
                                runSpacing: 8,
                                children: [
                                  _StatPill(
                                    icon: Icons.calendar_today,
                                    label:
                                        '${program.program.daysPerWeek} days/week',
                                  ),
                                  _StatPill(
                                    icon: Icons.local_fire_department,
                                    label: '${program.diet.estimatedKcal} kcal',
                                  ),
                                ],
                              ),
                            ],
                          ),
                        ),
                        if (progress != null) ...[
                          const SizedBox(width: 12),
                          ProgressRing(
                            ratio: progress.ratio,
                            size: 72,
                            stroke: 7,
                            onGradient: true,
                            label:
                                '${progress.daysDone}/${progress.daysTarget}',
                          ),
                        ],
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _getProgramCta(BuildContext context) {
    return Container(
      margin: const EdgeInsets.symmetric(horizontal: 16),
      decoration: BoxDecoration(
        gradient: brandGradient(),
        borderRadius: BorderRadius.circular(24),
      ),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          borderRadius: BorderRadius.circular(24),
          onTap: () => context.go('/programs/get'),
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Row(
              children: [
                CircleAvatar(
                  radius: 26,
                  backgroundColor: Colors.white.withValues(alpha: 0.18),
                  child: Icon(Icons.fitness_center, color: Colors.white),
                ),
                const SizedBox(width: 16),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Get your program',
                        style:
                            Theme.of(context).textTheme.titleMedium?.copyWith(
                                  color: Colors.white,
                                ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        'Take a photo to get a personalized plan',
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: Colors.white.withValues(alpha: 0.85),
                            ),
                      ),
                    ],
                  ),
                ),
                Icon(Icons.arrow_forward, color: Colors.white),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// Translucent stat pill on the hero program card.
class _StatPill extends StatelessWidget {
  const _StatPill({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.18),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: Colors.white),
          const SizedBox(width: 6),
          Text(
            label,
            style: Theme.of(context).textTheme.labelMedium?.copyWith(
                  color: Colors.white,
                  fontWeight: FontWeight.w600,
                ),
          ),
        ],
      ),
    );
  }
}
