// R-0014 / SPEC-0014 §2.5.3 — ProgramDetailScreen.
//
// Displays the active program and diet. Reached from ProgramProposalsScreen
// after a successful choose, or via the home shortcut (CurrentProgramCard).

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

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
        Text(program.archetypeId,
            style: Theme.of(context).textTheme.titleLarge),
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
      error: (_, __) => _GetProgramCta(context),
      data: (program) {
        if (program == null) return _GetProgramCta(context);
        return Card(
          margin: const EdgeInsets.all(16),
          child: ListTile(
            title: Text(program.program.split),
            subtitle: Text('${program.program.daysPerWeek} days/week · '
                '${program.diet.estimatedKcal} kcal'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => Navigator.of(context).pushNamed('/programs/current'),
          ),
        );
      },
    );
  }

  Widget _GetProgramCta(BuildContext context) => Card(
        margin: const EdgeInsets.all(16),
        child: ListTile(
          leading: const Icon(Icons.fitness_center),
          title: const Text('Get your program'),
          subtitle: const Text('Take a photo to get a personalized plan'),
          trailing: const Icon(Icons.chevron_right),
          onTap: () => Navigator.of(context).pushNamed('/onboarding'),
        ),
      );
}
