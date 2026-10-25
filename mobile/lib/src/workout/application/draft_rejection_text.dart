import '../../../l10n/app_localizations.dart';
import '../domain/draft_rejection.dart';
import '../domain/exercise_draft.dart';
import '../domain/set_draft.dart';

/// The catalogue line for a [DraftRejection], drawn inline or spoken by the
/// voice coach. The ranges quoted are [SetLimits] and [ExerciseLimits], so
/// the message and the check cannot drift apart.
String draftRejectionText(DraftRejection rejection, AppLocalizations l10n) =>
    switch (rejection) {
      DraftRejection.noSession => l10n.rejectStartWorkoutFirst,
      DraftRejection.noExercise => l10n.rejectAddExerciseFirst,
      DraftRejection.nameEmpty => l10n.rejectEnterExerciseName,
      DraftRejection.nameTooLong =>
        l10n.rejectNameTooLong(ExerciseLimits.maxNameScalars),
      DraftRejection.repsMissing => l10n.rejectEnterReps,
      DraftRejection.repsOutOfRange =>
        l10n.rejectRepsOutOfRange(SetLimits.minReps, SetLimits.maxReps),
      DraftRejection.weightOutOfRange =>
        l10n.rejectWeightOutOfRange(SetLimits.maxWeightKg.toInt()),
      DraftRejection.rpeOutOfRange =>
        l10n.rejectRpeOutOfRange(SetLimits.minRpe, SetLimits.maxRpe),
    };
