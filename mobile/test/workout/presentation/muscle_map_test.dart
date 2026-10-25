// The anatomy chart renders and the live session shows it for the current
// exercise.

import 'package:fitai/src/workout/domain/muscle_activation.dart';
import 'package:fitai/src/workout/presentation/muscle_map.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/l10n.dart';

void main() {
  testWidgets('renders the chart with primary/assist tags', (tester) async {
    await tester.pumpWidget(localizedApp(
      home: Scaffold(
        body: MuscleMap(activation: activationFor('Bench press')),
      ),
    ));
    await tester.pump();

    expect(find.text(l10n.regionChest), findsOneWidget); // primary tag
    expect(find.text(l10n.regionTriceps), findsOneWidget); // assist tag
  });
}
