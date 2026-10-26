// Date helpers shared by the calendar and the wire: a date without its time,
// and the backend's `YYYY-MM-DD` form.

import 'package:fitai/src/core/dates.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('dateOnly drops the time of day', () {
    expect(dateOnly(DateTime(2026, 10, 22, 15, 30)), DateTime(2026, 10, 22));
  });

  test('mondayOf lands on the Monday of the week, at midnight', () {
    final monday = DateTime(2026, 6, 29);
    expect(mondayOf(DateTime(2026, 7, 1, 13, 5)), monday); // Wednesday
    expect(mondayOf(monday), monday);
    expect(mondayOf(DateTime(2026, 7, 5)), monday); // Sunday
    expect(mondayOf(DateTime(2026, 3, 1)), DateTime(2026, 2, 23)); // month
    // A week with a DST change (last Sunday of October in Europe) still
    // yields midnight of the right day, not 23:00 of the day before.
    final afterDst = DateTime(2026, 10, 28);
    expect(mondayOf(afterDst), DateTime(2026, 10, 26));
    expect(mondayOf(afterDst).hour, 0);
  });

  test('isoDate zero-pads to the NaiveDate wire form', () {
    expect(isoDate(DateTime(2026, 3, 5, 23, 59)), '2026-03-05');
    expect(isoDate(DateTime(999, 12, 31)), '0999-12-31');
  });
}
