/// Why the driver rejects a draft exercise or set. The domain reports the
/// kind; the presentation layer turns it into a localized line
/// (`draftRejectionText`, R-0061 AC11), spoken or drawn. Same idiom as
/// `ProfileFieldError`.
enum DraftRejection {
  noSession,
  noExercise,
  nameEmpty,
  nameTooLong,
  repsMissing,
  repsOutOfRange,
  weightOutOfRange,
  rpeOutOfRange,
}
