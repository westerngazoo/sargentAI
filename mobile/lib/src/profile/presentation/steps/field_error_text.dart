import '../../../../l10n/app_localizations.dart';
import '../../domain/profile_draft.dart';
import '../../domain/profile_field_error.dart';

/// The catalogue message for a draft field error, or `null` when there is
/// none — the shape `InputDecoration.errorText` takes. The ranges quoted are
/// [ProfileLimits], so the message and the check cannot drift apart.
String? fieldErrorText(ProfileFieldError? error, AppLocalizations l10n) =>
    switch (error) {
      null => null,
      ProfileFieldError.dobMissing => l10n.dobMissing,
      ProfileFieldError.dobInFuture => l10n.dobInFuture,
      ProfileFieldError.ageOutOfRange =>
        l10n.ageOutOfRange(ProfileLimits.minAge, ProfileLimits.maxAge),
      ProfileFieldError.heightMissing => l10n.heightMissing,
      ProfileFieldError.heightOutOfRange =>
        l10n.heightOutOfRange(ProfileLimits.minHeight, ProfileLimits.maxHeight),
      ProfileFieldError.weightMissing => l10n.weightMissing,
      ProfileFieldError.weightOutOfRange => l10n.weightOutOfRange(
          ProfileLimits.minWeight.toInt(),
          ProfileLimits.maxWeight.toInt(),
        ),
      ProfileFieldError.bodyFatOutOfRange => l10n.bodyFatOutOfRange(
          ProfileLimits.minBodyFat.toInt(),
          ProfileLimits.maxBodyFat.toInt(),
        ),
    };
