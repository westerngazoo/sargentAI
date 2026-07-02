// R-0014 / SPEC-0014 §2.5.3 — ProgramDetailScreen.
//
// Displays the active program and diet. Reached from ProgramProposalsScreen
// after a successful choose, or via the home shortcut (CurrentProgramCard).

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/theme/app_theme.dart';
import '../application/program_providers.dart';
import '../models/program_proposal.dart';
import '../models/user_program.dart';

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

class _ProgramDetail extends StatelessWidget {
  const _ProgramDetail({required this.program});

  final UserProgram program;

  @override
  Widget build(BuildContext context) {
    final p = program.program;
    final d = program.diet;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(_displayTitle(program.archetypeId),
            style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 8),
        _Section(
          title: 'Training',
          children: [
            _InfoRow('Split', p.split),
            _InfoRow('Training', '${p.daysPerWeek} days/week'),
            _InfoRow(
                'Session duration', '${p.estimatedSessionDurationMin} min'),
            _InfoRow('Intensity', p.intensityGuidance),
            _InfoRow('Rest', p.restGuidance),
            _InfoRow('Progression', p.progressionGuidance),
          ],
        ),
        const SizedBox(height: 16),
        _Section(
          title: 'Highlight exercises',
          children: [
            Wrap(
              spacing: 8,
              runSpacing: 4,
              children: p.highlightExercises
                  .map((e) => Chip(label: Text(e)))
                  .toList(),
            ),
          ],
        ),
        const SizedBox(height: 16),
        _Section(
          title: 'Nutrition',
          children: [
            _InfoRow('Approach', d.approach),
            _InfoRow('Strategy', d.calorieStrategy),
            _InfoRow('Meals', d.mealStructure),
            const SizedBox(height: 8),
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
  const _Section({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: Theme.of(context).textTheme.titleMedium),
          const Divider(),
          ...children,
        ],
      );
}

class _InfoRow extends StatelessWidget {
  const _InfoRow(this.label, this.value);

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(
              width: 130,
              child: Text(
                label,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
            Expanded(child: Text(value)),
          ],
        ),
      );
}

class _MacroTable extends StatelessWidget {
  const _MacroTable({required this.diet});

  final GeneratedDiet diet;

  @override
  Widget build(BuildContext context) => Row(
        mainAxisAlignment: MainAxisAlignment.spaceAround,
        children: [
          _macroCell('Protein', '${diet.proteinG}g', context),
          _macroCell('Carbs', '${diet.carbsG}g', context),
          _macroCell('Fat', '${diet.fatG}g', context),
          _macroCell('kcal', '${diet.estimatedKcal}', context),
        ],
      );

  Widget _macroCell(String label, String value, BuildContext context) => Column(
        children: [
          Text(label, style: Theme.of(context).textTheme.labelSmall),
          Text(value),
        ],
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
        return Container(
          margin: const EdgeInsets.symmetric(horizontal: 16),
          decoration: BoxDecoration(
            gradient: sunsetGradient(),
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
                    const SizedBox(height: 8),
                    Text(
                      program.program.split,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            color: Colors.white,
                          ),
                    ),
                    const SizedBox(height: 14),
                    Row(
                      children: [
                        _StatPill(
                          icon: Icons.calendar_today,
                          label: '${program.program.daysPerWeek} days/week',
                        ),
                        const SizedBox(width: 8),
                        _StatPill(
                          icon: Icons.local_fire_department,
                          label: '${program.diet.estimatedKcal} kcal',
                        ),
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
        gradient: sunsetGradient(),
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
