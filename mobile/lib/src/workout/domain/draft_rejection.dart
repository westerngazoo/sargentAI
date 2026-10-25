/// Why the driver rejects a draft exercise or set. The domain reports the
/// kind; the application layer turns it into a localized line
/// (`draftRejectionText`, R-0061 AC11), spoken or drawn. Same idiom as
/// `ProfileFieldError`.
enum DraftRejection {
  /// No session is in progress.
  noSession,

  /// No exercise is open to take the set.
  noExercise,

  /// The exercise name is blank.
  nameEmpty,

  /// The exercise name exceeds [ExerciseLimits.maxNameLength].
  nameTooLong,

  /// The set names no reps.
  repsMissing,

  /// The reps fall outside [SetLimits]' range.
  repsOutOfRange,

  /// The weight falls outside [SetLimits]' range.
  weightOutOfRange,

  /// The RPE falls outside [SetLimits]' range.
  rpeOutOfRange,
}
