// R-0061 AC5 -> SPEC-0061 §2.5: the one primary action of the selected day,
// as a pure function of (day vs today, state). Slice 1f knows two states.

import 'package:fitai/src/home/day_action.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  final today = DateTime(2026, 10, 22, 15, 30); // time must not matter
  final yesterday = DateTime(2026, 10, 21);
  final tomorrow = DateTime(2026, 10, 23);

  test('today with nothing logged starts a free session', () {
    final a = primaryAction(
        day: DateTime(2026, 10, 22), today: today, state: const NothingDay());
    expect(a, isA<StartSession>());
    expect((a as StartSession).planDayIndex, isNull);
  });

  test('a past day with nothing logged is logged late', () {
    final a =
        primaryAction(day: yesterday, today: today, state: const NothingDay());
    expect(a, isA<LogLate>());
  });

  test('a done day, whatever its tense, shows what was logged', () {
    for (final d in [yesterday, today, tomorrow]) {
      expect(primaryAction(day: d, today: today, state: const DoneDay()),
          isA<ViewLog>());
    }
  });

  test('a future day offers nothing', () {
    expect(
        primaryAction(day: tomorrow, today: today, state: const NothingDay()),
        isA<NoAction>());
  });

  test('dateOnly drops the time of day', () {
    expect(dateOnly(today), DateTime(2026, 10, 22));
  });
}
