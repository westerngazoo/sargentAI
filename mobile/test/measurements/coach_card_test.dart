// R-0017 AC8 — Coach suggestions card: renders suggestions with severity,
// shows the positive on-track empty state, and hides without a program.

import 'package:fitai/src/measurements/models/coach_suggestion.dart';
import 'package:fitai/src/measurements/presentation/coach_card.dart';
import 'package:fitai/src/measurements/services/adjustment_service.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

Widget _harness(CoachAdvice advice) => ProviderScope(
      overrides: [
        coachAdviceProvider.overrideWith((ref) async => advice),
      ],
      child: const MaterialApp(
        home: Scaffold(body: SingleChildScrollView(child: CoachCard())),
      ),
    );

void main() {
  testWidgets('renders suggestions with severity badge and rationale',
      (tester) async {
    const advice = CoachAdvice(
      windowWeeks: 8,
      suggestions: [
        CoachSuggestion(
          kind: 'deload_lift',
          severity: 'action',
          rationale:
              "Bench Press hasn't set a new estimated-1RM peak — deload ~10%.",
        ),
        CoachSuggestion(
          kind: 'progress_lift',
          severity: 'info',
          rationale: 'Squat is trending up — add the next 2.5 kg.',
        ),
      ],
    );
    await tester.pumpWidget(_harness(advice));
    await tester.pumpAndSettle();

    expect(find.text('Coach suggestions'), findsOneWidget);
    expect(find.textContaining('Bench Press'), findsOneWidget);
    expect(find.textContaining('Squat'), findsOneWidget);
    expect(find.text('ACTION'), findsOneWidget); // only the action suggestion
    expect(find.text('last 8 wks'), findsOneWidget);
  });

  testWidgets('empty suggestions show the positive on-track state',
      (tester) async {
    const advice = CoachAdvice(windowWeeks: 8, suggestions: []);
    await tester.pumpWidget(_harness(advice));
    await tester.pumpAndSettle();

    expect(find.text('Coach suggestions'), findsOneWidget);
    expect(
      find.textContaining("On track — keep doing what you're doing."),
      findsOneWidget,
    );
  });

  testWidgets('hidden entirely when there is no active program',
      (tester) async {
    const advice = CoachAdvice(
      windowWeeks: 8,
      suggestions: [],
      reason: 'no_active_program',
    );
    await tester.pumpWidget(_harness(advice));
    await tester.pumpAndSettle();

    expect(find.text('Coach suggestions'), findsNothing);
  });

  test('wire parsing maps the tagged change and severity', () {
    final advice = CoachAdvice.fromJson({
      'window_weeks': 8,
      'suggestions': [
        {
          'change': {'kind': 'adjust_kcal', 'delta_pct': 10},
          'severity': 'action',
          'rationale': 'Your plan aims at gaining weight…',
        },
      ],
    });
    expect(advice.hasProgram, isTrue);
    expect(advice.suggestions.single.kind, 'adjust_kcal');
    expect(advice.suggestions.single.isAction, isTrue);
  });
}
