// R-0061 AC8 -> SPEC-0061 §2.6: the Medidas panel holds the measurement form
// and the coach card. Saving logs today's weigh-in and closes the panel.

import 'package:fitai/src/core/dates.dart';
import 'package:fitai/src/core/panels/show_panel.dart';
import 'package:fitai/src/measurements/models/coach_suggestion.dart';
import 'package:fitai/src/measurements/presentation/coach_card.dart';
import 'package:fitai/src/measurements/presentation/log_measurement_form.dart';
import 'package:fitai/src/measurements/presentation/measurements_panel.dart';
import 'package:fitai/src/measurements/services/adjustment_service.dart';
import 'package:fitai/src/measurements/services/measurement_service.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/l10n.dart';
import '../support/measurement_fakes.dart';

void main() {
  late MockMeasurementService service;

  setUp(() {
    service = MockMeasurementService();
    when(() => service.list()).thenAnswer((_) async => const []);
  });

  Future<void> pumpHost(WidgetTester tester) async {
    await tester.pumpWidget(ProviderScope(
      overrides: [
        measurementServiceProvider.overrideWithValue(service),
        coachAdviceProvider.overrideWith((ref) async => const CoachAdvice(
              windowWeeks: 8,
              suggestions: [
                CoachSuggestion(
                  kind: 'progress_lift',
                  severity: 'info',
                  rationale: 'Squat is trending up.',
                ),
              ],
            )),
      ],
      child: localizedApp(
        home: Builder(
          builder: (context) => Scaffold(
            body: Center(
              child: ElevatedButton(
                onPressed: () => showPanel<void>(
                  context,
                  title: l10n.menuMeasurements,
                  body: (_) => const MeasurementsPanel(),
                ),
                child: const Text('open'),
              ),
            ),
          ),
        ),
      ),
    ));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
  }

  testWidgets('the panel holds the form and the coach card', (tester) async {
    await pumpHost(tester);
    expect(find.byType(LogMeasurementForm), findsOneWidget);
    expect(find.text(l10n.logMeasurement), findsOneWidget);
    expect(find.text(l10n.saveMeasurement), findsOneWidget);
    expect(find.byType(CoachCard), findsOneWidget);
    expect(find.text(l10n.coachSuggestions), findsOneWidget);
  });

  testWidgets('saving logs today\'s weight and closes the panel',
      (tester) async {
    when(() => service.create(
          measuredOn: any(named: 'measuredOn'),
          weightKg: any(named: 'weightKg'),
          bodyFatPercentage: any(named: 'bodyFatPercentage'),
        )).thenAnswer((_) async => sampleMeasurement());
    await pumpHost(tester);

    await tester.enterText(find.byType(TextField).first, '80.5');
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.saveMeasurement));
    await tester.pumpAndSettle();

    verify(() => service.create(
          measuredOn: isoDate(DateTime.now()),
          weightKg: 80.5,
          bodyFatPercentage: null,
        )).called(1);
    expect(find.byType(MeasurementsPanel), findsNothing);
    expect(find.text(l10n.measurementLogged), findsOneWidget);
  });
}
