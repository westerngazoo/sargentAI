// R-0061 AC11: the driver and the drafts report a typed DraftRejection; the
// application layer renders it from the catalogue, quoting the shared limits.

import 'package:fitai/src/workout/domain/draft_rejection.dart';
import 'package:fitai/src/workout/domain/exercise_draft.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:fitai/src/workout/application/draft_rejection_text.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/l10n.dart';

void main() {
  test('every rejection renders its catalogue entry', () {
    expect(draftRejectionText(DraftRejection.noSession, l10n),
        l10n.rejectStartWorkoutFirst);
    expect(draftRejectionText(DraftRejection.noExercise, l10n),
        l10n.rejectAddExerciseFirst);
    expect(draftRejectionText(DraftRejection.nameEmpty, l10n),
        l10n.rejectEnterExerciseName);
    expect(draftRejectionText(DraftRejection.nameTooLong, l10n),
        l10n.rejectNameTooLong(ExerciseLimits.maxNameScalars));
    expect(draftRejectionText(DraftRejection.repsMissing, l10n),
        l10n.rejectEnterReps);
    expect(draftRejectionText(DraftRejection.repsOutOfRange, l10n),
        l10n.rejectRepsOutOfRange(SetLimits.minReps, SetLimits.maxReps));
    expect(draftRejectionText(DraftRejection.weightOutOfRange, l10n),
        l10n.rejectWeightOutOfRange(SetLimits.maxWeightKg.toInt()));
    expect(draftRejectionText(DraftRejection.rpeOutOfRange, l10n),
        l10n.rejectRpeOutOfRange(SetLimits.minRpe, SetLimits.maxRpe));
  });
}
