// Date helpers shared by the calendar and the wire: a date without its time,
// and the backend's `YYYY-MM-DD` form.

import 'package:fitai/src/core/dates.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('dateOnly drops the time of day', () {
    expect(dateOnly(DateTime(2026, 10, 22, 15, 30)), DateTime(2026, 10, 22));
  });

  test('isoDate zero-pads to the NaiveDate wire form', () {
    expect(isoDate(DateTime(2026, 3, 5, 23, 59)), '2026-03-05');
    expect(isoDate(DateTime(999, 12, 31)), '0999-12-31');
  });
}
