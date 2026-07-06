// R-0030 (amended, R-0057) widget tests — BodyTypePickerScreen.
//
// AC-coverage (amended as-built acceptance criteria):
// - AC2  grid renders the 3 body-shape cards (Material-icon shapes)
// - AC4  band chips appear only after a shape is selected; shape+band form the
//        synthetic selection
// - AC9  Confirm ("Find my program") is disabled until BOTH a shape and a band
//        are selected
// - AC6/AC8 confirming calls syntheticMatch with the selected shape+band
//        (the entry into the existing rank()/proposals flow)
//
// Backfilled under R-0057 — the feature shipped (PR #30) with no tests.

import 'dart:async';

import 'package:fitai/src/program/models/synthetic_match.dart';
import 'package:fitai/src/program/presentation/body_type_picker_screen.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/program_fakes.dart';

void main() {
  late MockProgramService programService;

  setUp(() {
    programService = MockProgramService();
  });

  Future<void> pumpPicker(WidgetTester tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          programServiceProvider.overrideWithValue(programService),
        ],
        child: const MaterialApp(home: BodyTypePickerScreen()),
      ),
    );
    await tester.pumpAndSettle();
  }

  // AC2 — the grid shows all three shapes.
  testWidgets('grid renders all three body-shape cards', (tester) async {
    await pumpPicker(tester);

    for (final shape in BodyShape.values) {
      expect(find.text(shape.label), findsOneWidget);
    }
    // The three shape icons are present.
    expect(find.byIcon(Icons.straighten), findsOneWidget); // ectomorph
    expect(find.byIcon(Icons.fitness_center), findsOneWidget); // mesomorph
    expect(find.byIcon(Icons.circle_outlined), findsOneWidget); // endomorph
  });

  // AC4 — band chips are hidden until a shape is chosen, then appear.
  testWidgets('fat-band chips appear only after a shape is selected',
      (tester) async {
    await pumpPicker(tester);

    // Before selection: no band section, no band labels.
    expect(find.text('Body fat level'), findsNothing);
    expect(find.text('Moderate'), findsNothing);

    // Select a shape.
    await tester.tap(find.text(BodyShape.mesomorph.label));
    await tester.pumpAndSettle();

    // After selection: band section + all three bands visible, shape highlighted.
    expect(find.text('Body fat level'), findsOneWidget);
    for (final band in FatBand.values) {
      expect(find.text(band.label), findsOneWidget);
    }
    expect(find.byIcon(Icons.check_circle), findsOneWidget);
  });

  // AC9 — Confirm is disabled until both selections are made.
  testWidgets('Confirm is disabled until both a shape and a band are selected',
      (tester) async {
    await pumpPicker(tester);

    FilledButton confirmButton() =>
        tester.widget<FilledButton>(find.byType(FilledButton));

    // Nothing selected → disabled.
    expect(confirmButton().onPressed, isNull);

    // Shape only → still disabled.
    await tester.tap(find.text(BodyShape.mesomorph.label));
    await tester.pumpAndSettle();
    expect(confirmButton().onPressed, isNull);

    // Shape + band → enabled.
    await tester.tap(find.text(FatBand.moderate.label));
    await tester.pumpAndSettle();
    expect(confirmButton().onPressed, isNotNull);
  });

  // AC6/AC8 — confirming calls syntheticMatch with the chosen shape + band.
  testWidgets(
      'confirming calls syntheticMatch with the selected shape and band',
      (tester) async {
    // Never-completing future: we only assert the call, not the navigation.
    when(() => programService.syntheticMatch(
          BodyShape.endomorph,
          FatBand.bulky,
        )).thenAnswer((_) => Completer<SyntheticMatchResponse>().future);

    await pumpPicker(tester);

    await tester.tap(find.text(BodyShape.endomorph.label));
    await tester.pumpAndSettle();
    // The last band chip can sit below the test fold — bring it into view.
    await tester.ensureVisible(find.text(FatBand.bulky.label));
    await tester.pumpAndSettle();
    await tester.tap(find.text(FatBand.bulky.label));
    await tester.pumpAndSettle();
    // Tap the confirm button by type (robust against layout/fold changes).
    await tester.ensureVisible(find.byType(FilledButton));
    await tester.pumpAndSettle();
    await tester.tap(find.byType(FilledButton));
    await tester.pump(); // kick off _confirm; syntheticMatch is invoked

    verify(() => programService.syntheticMatch(
          BodyShape.endomorph,
          FatBand.bulky,
        )).called(1);
  });
}
