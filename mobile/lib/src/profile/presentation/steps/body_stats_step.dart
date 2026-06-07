import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../application/onboarding_controller.dart';

/// Step 1 — body stats (date of birth, height, weight), all required (AC4).
class BodyStatsStep extends ConsumerWidget {
  const BodyStatsStep({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final draft =
        ref.watch(onboardingControllerProvider.select((s) => s.draft));
    final ctrl = ref.read(onboardingControllerProvider.notifier);
    final today = DateTime.now();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('About you', style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: 16),
        InputDecorator(
          decoration: InputDecoration(
            labelText: 'Date of birth',
            errorText: draft.dateOfBirth == null ? null : draft.dobError(today),
          ),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(draft.dateOfBirth == null
                  ? 'Not set'
                  : _fmt(draft.dateOfBirth!)),
              TextButton(
                onPressed: () async {
                  final picked = await showDatePicker(
                    context: context,
                    initialDate:
                        DateTime(today.year - 25, today.month, today.day),
                    firstDate: DateTime(today.year - 120),
                    lastDate: today,
                  );
                  if (picked != null) ctrl.setBodyStats(dob: picked);
                },
                child: const Text('Choose'),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        TextField(
          keyboardType: TextInputType.number,
          decoration: InputDecoration(
            labelText: 'Height (cm)',
            errorText: draft.heightCm == null ? null : draft.heightError(),
          ),
          onChanged: (v) => ctrl.setBodyStats(height: int.tryParse(v)),
        ),
        const SizedBox(height: 12),
        TextField(
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
          decoration: InputDecoration(
            labelText: 'Weight (kg)',
            errorText: draft.weightKg == null ? null : draft.weightError(),
          ),
          onChanged: (v) => ctrl.setBodyStats(weight: double.tryParse(v)),
        ),
      ],
    );
  }
}

String _fmt(DateTime d) => '${d.year.toString().padLeft(4, '0')}-'
    '${d.month.toString().padLeft(2, '0')}-'
    '${d.day.toString().padLeft(2, '0')}';
