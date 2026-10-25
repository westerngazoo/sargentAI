/// Calendar-date helpers shared by the calendar and the wire: the app reasons
/// on local dates (SPEC-0061 §2.4: `today` is the client's local date).
library;

/// Strips the time, so two `DateTime`s on the same calendar day compare equal.
DateTime dateOnly(DateTime d) => DateTime(d.year, d.month, d.day);

/// `YYYY-MM-DD`, zero-padded: the backend's `NaiveDate` wire form.
String isoDate(DateTime d) => '${d.year.toString().padLeft(4, '0')}-'
    '${d.month.toString().padLeft(2, '0')}-'
    '${d.day.toString().padLeft(2, '0')}';
