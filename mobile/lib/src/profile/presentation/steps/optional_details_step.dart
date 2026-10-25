import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../l10n/app_localizations.dart';
import '../../application/onboarding_controller.dart';
import '../../domain/sex.dart';
import 'field_error_text.dart';

/// Step 3 — optional details (sex, body-fat %); both skippable (AC6).
class OptionalDetailsStep extends ConsumerWidget {
  const OptionalDetailsStep({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final draft =
        ref.watch(onboardingControllerProvider.select((s) => s.draft));
    final ctrl = ref.read(onboardingControllerProvider.notifier);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(l10n.optionalDetails,
            style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: 8),
        Text(l10n.youCanSkipThese),
        const SizedBox(height: 16),
        Wrap(
          spacing: 8,
          children: [
            ChoiceChip(
              label: Text(l10n.sexMale),
              selected: draft.sex == Sex.male,
              onSelected: (_) => ctrl.setOptional(sex: Sex.male),
            ),
            ChoiceChip(
              label: Text(l10n.sexFemale),
              selected: draft.sex == Sex.female,
              onSelected: (_) => ctrl.setOptional(sex: Sex.female),
            ),
            if (draft.sex != null)
              ActionChip(
                label: Text(l10n.clear),
                onPressed: () => ctrl.setOptional(clearSex: true),
              ),
          ],
        ),
        const SizedBox(height: 16),
        TextField(
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
          decoration: InputDecoration(
            labelText: l10n.bodyFatOptionalLabel,
            errorText: fieldErrorText(draft.bodyFatError(), l10n),
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
