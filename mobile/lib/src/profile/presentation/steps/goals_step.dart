import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../l10n/app_localizations.dart';
import '../../application/onboarding_controller.dart';
import '../../domain/goal.dart';

/// Step 2 — goals; at least one is required to finish (AC5).
class GoalsStep extends ConsumerWidget {
  const GoalsStep({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final goals =
        ref.watch(onboardingControllerProvider.select((s) => s.draft.goals));
    final ctrl = ref.read(onboardingControllerProvider.notifier);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(l10n.yourGoals, style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: 8),
        Text(l10n.pickAtLeastOne),
        const SizedBox(height: 16),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            for (final g in Goal.values)
              FilterChip(
                label: Text(goalName(g, l10n)),
                selected: goals.contains(g),
                onSelected: (_) => ctrl.toggleGoal(g),
              ),
          ],
        ),
      ],
    );
  }
}

/// The catalogue name of a [Goal].
String goalName(Goal g, AppLocalizations l10n) => switch (g) {
      Goal.loseFat => l10n.goalLoseFat,
      Goal.buildMuscle => l10n.goalBuildMuscle,
      Goal.recomp => l10n.goalRecomp,
      Goal.maintain => l10n.goalMaintain,
      Goal.gainStrength => l10n.goalGainStrength,
    };
