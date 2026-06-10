import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../application/onboarding_controller.dart';
import '../../domain/sex.dart';

/// Step 3 — optional details (sex, body-fat %); both skippable (AC6).
class OptionalDetailsStep extends ConsumerWidget {
  const OptionalDetailsStep({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final draft =
        ref.watch(onboardingControllerProvider.select((s) => s.draft));
    final ctrl = ref.read(onboardingControllerProvider.notifier);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('Optional details', style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: 8),
        const Text('You can skip these.'),
        const SizedBox(height: 16),
        Wrap(
          spacing: 8,
          children: [
            ChoiceChip(
              label: const Text('Male'),
              selected: draft.sex == Sex.male,
              onSelected: (_) => ctrl.setOptional(sex: Sex.male),
            ),
            ChoiceChip(
              label: const Text('Female'),
              selected: draft.sex == Sex.female,
              onSelected: (_) => ctrl.setOptional(sex: Sex.female),
            ),
            if (draft.sex != null)
              ActionChip(
                label: const Text('Clear'),
                onPressed: () => ctrl.setOptional(clearSex: true),
              ),
          ],
        ),
        const SizedBox(height: 16),
        TextField(
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
          decoration: InputDecoration(
            labelText: 'Body fat % (optional)',
            errorText: draft.bodyFatError(),
          ),
          onChanged: (v) {
            if (v.isEmpty) {
              ctrl.setOptional(clearBodyFat: true);
              return;
            }
            final parsed = double.tryParse(v);
            if (parsed != null) ctrl.setOptional(bodyFat: parsed);
          },
        ),
      ],
    );
  }
}
