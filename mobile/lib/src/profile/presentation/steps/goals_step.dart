import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../application/onboarding_controller.dart';
import '../../domain/goal.dart';

/// Step 2 — goals; at least one is required to finish (AC5).
class GoalsStep extends ConsumerWidget {
  const GoalsStep({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final goals =
        ref.watch(onboardingControllerProvider.select((s) => s.draft.goals));
    final ctrl = ref.read(onboardingControllerProvider.notifier);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('Your goals', style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: 8),
        const Text('Pick at least one.'),
        const SizedBox(height: 16),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            for (final g in Goal.values)
              FilterChip(
                label: Text(_label(g)),
                selected: goals.contains(g),
                onSelected: (_) => ctrl.toggleGoal(g),
              ),
          ],
        ),
      ],
    );
  }
}

String _label(Goal g) => switch (g) {
      Goal.loseFat => 'Lose fat',
      Goal.buildMuscle => 'Build muscle',
      Goal.recomp => 'Recomposition',
      Goal.maintain => 'Maintain',
      Goal.gainStrength => 'Gain strength',
    };
