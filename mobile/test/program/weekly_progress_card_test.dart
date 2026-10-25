// R-0061 AC11 — the weekly progress card speaks the catalogue, including the
// plural line, which is the one message in the program folder with a plural.

import 'package:fitai/src/program/application/program_progress.dart';
import 'package:fitai/src/program/presentation/progress_ring.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/l10n.dart';

void main() {
  Future<void> pumpCard(WidgetTester tester, WeeklyProgress progress) async {
    await tester.pumpWidget(
      localizedApp(
          home: Scaffold(body: WeeklyProgressCard(progress: progress))),
    );
  }

  testWidgets('a week in progress shows the day count and the plural line',
      (tester) async {
    await pumpCard(
      tester,
      const WeeklyProgress(daysDone: 2, daysTarget: 4, totalSessions: 7),
    );

    expect(find.text(l10n.thisWeekCaps), findsOneWidget);
    expect(find.text(l10n.daysOfTarget(2, 4)), findsOneWidget);
    expect(find.text(l10n.sessionsLoggedOnProgram(7)), findsOneWidget);
    expect(find.text(l10n.percent(50)), findsOneWidget);
    expect(find.text(l10n.targetMet), findsNothing);
  });

  testWidgets(
      'a single session takes the singular; a met target shows the '
      'badge', (tester) async {
    await pumpCard(
      tester,
      const WeeklyProgress(daysDone: 4, daysTarget: 4, totalSessions: 1),
    );

    expect(find.text(l10n.sessionsLoggedOnProgram(1)), findsOneWidget);
    expect(
      l10n.sessionsLoggedOnProgram(1),
      isNot(equals(l10n.sessionsLoggedOnProgram(2))),
      reason: 'the catalogue carries a real plural, not a suffix',
    );
    expect(find.text(l10n.targetMet), findsOneWidget);
  });
}
