import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../../../../l10n/app_localizations.dart';
import '../../application/onboarding_controller.dart';
import 'field_error_text.dart';

/// Step 1 — body stats (date of birth, height, weight), all required (AC4).
class BodyStatsStep extends ConsumerWidget {
  const BodyStatsStep({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final draft =
        ref.watch(onboardingControllerProvider.select((s) => s.draft));
    final ctrl = ref.read(onboardingControllerProvider.notifier);
    final today = DateTime.now();
    final dob = draft.dateOfBirth;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(l10n.aboutYou, style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: 16),
        InputDecorator(
          decoration: InputDecoration(
            labelText: l10n.dateOfBirthLabel,
            errorText: dob == null
                ? null
                : fieldErrorText(draft.dobError(today), l10n),
          ),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(dob == null ? l10n.notSet : _formatDate(context, dob)),
              TextButton(
                onPressed: () async {
                  final picked = await showDatePicker(
                    context: context,
                    initialDate:
                        DateTime(today.year - 25, today.month, today.day),
                    firstDate: DateTime(today.year - 120),
                    lastDate: today,
                    // A birth date starts from the year, not from a month
                    // grid 25 years back (beta, 2026-10-22).
                    initialDatePickerMode: DatePickerMode.year,
                  );
                  if (picked != null) ctrl.setBodyStats(dob: picked);
                },
                child: Text(l10n.choose),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        TextField(
          keyboardType: TextInputType.number,
          decoration: InputDecoration(
            labelText: l10n.heightCmLabel,
            errorText: draft.heightCm == null
                ? null
                : fieldErrorText(draft.heightError(), l10n),
          ),
          onChanged: (v) => ctrl.setBodyStats(height: int.tryParse(v)),
        ),
        const SizedBox(height: 12),
        TextField(
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
          decoration: InputDecoration(
            labelText: l10n.weightKgLabel,
            errorText: draft.weightKg == null
                ? null
                : fieldErrorText(draft.weightError(), l10n),
          ),
          onChanged: (v) => ctrl.setBodyStats(weight: double.tryParse(v)),
        ),
      ],
    );
  }
}

/// The picked date in the app locale (SPEC-0061 §2.9: dates use `intl` with
/// the resolved locale).
String _formatDate(BuildContext context, DateTime d) =>
    DateFormat.yMMMd(Localizations.localeOf(context).toString()).format(d);
