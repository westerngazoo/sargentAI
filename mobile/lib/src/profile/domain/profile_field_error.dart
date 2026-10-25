/// Why a draft field fails validation. The domain reports the kind; the
/// presentation layer turns it into a localized message (R-0061 AC11), so no
/// user-facing text is built here.
enum ProfileFieldError {
  dobMissing,
  dobInFuture,
  ageOutOfRange,
  heightMissing,
  heightOutOfRange,
  weightMissing,
  weightOutOfRange,
  bodyFatOutOfRange,
}
