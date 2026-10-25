// SPEC-0061 §2.5: day arithmetic on dates, never on Durations, so a DST
// change cannot duplicate or skip a calendar cell.

import 'package:fitai/src/home/calendar.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('addDays lands on midnight of the target date all year', () {
    final start = DateTime(2026, 1, 1);
    for (var i = 0; i < 400; i++) {
      final d = addDays(start, i);
      expect(d.hour, 0, reason: '$d');
      expect(d.minute, 0);
    }
    expect(addDays(DateTime(2026, 10, 31), 1), DateTime(2026, 11, 1));
    expect(addDays(DateTime(2026, 3, 1), -1), DateTime(2026, 2, 28));
  });

  test('capitalizeFirst only touches the first letter', () {
    expect(capitalizeFirst('octubre 2026'), 'Octubre 2026');
    expect(capitalizeFirst(''), '');
  });
}
