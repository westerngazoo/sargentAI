// R-0061 AC4 -> SPEC-0061 §2.5 (slice 2d): one glyph per day state in a
// DayCell — ● done, ○ planned, — rest, nothing for an empty day. Structural,
// no pixel goldens (they rasterize differently on the Linux runner).

import 'package:fitai/src/home/calendar.dart';
import 'package:fitai/src/program/models/day_state.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/l10n.dart';

void main() {
  Future<void> pumpCell(WidgetTester tester, DayState state) async {
    await tester.pumpWidget(localizedApp(
      home: Scaffold(
        body: Center(
          child: SizedBox(
            width: 56,
            height: 48,
            child: DayCell(
              day: DateTime(2026, 10, 22),
              state: state,
              isToday: false,
              isSelected: false,
              inMonth: true,
              onTap: () {},
            ),
          ),
        ),
      ),
    ));
  }

  const marks = ['mark-done', 'mark-planned', 'mark-rest'];

  for (final (state, mark) in <(DayState, String?)>[
    (const DoneDay(completed: [0]), 'mark-done'),
    (const DoneDay(completed: [], pending: 1), 'mark-done'),
    (const PlannedDay(0), 'mark-planned'),
    (const RestDay(1), 'mark-rest'),
    (const NothingDay(), null),
    (const NothingDay(pending: 0), null),
  ]) {
    testWidgets('${state.runtimeType} shows ${mark ?? 'no mark'}',
        (tester) async {
      await pumpCell(tester, state);
      expect(tester.takeException(), isNull);
      for (final m in marks) {
        expect(find.byKey(Key(m)), m == mark ? findsOneWidget : findsNothing,
            reason: m);
      }
    });
  }
}
