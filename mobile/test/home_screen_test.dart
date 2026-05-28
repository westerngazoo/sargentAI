// Widget test for the placeholder home screen.
//
// Authored by qa agent during R-0001 step 3 (test planning).
// Pre-implementation red state = compile failure: the `fitai` Flutter
// package does not exist yet (no pubspec.yaml, no lib/screens/home_screen.dart).
// Implementation step 5 (SPEC-0001 §3.10–§3.15) makes this green.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:fitai/screens/home_screen.dart';

void main() {
  testWidgets('renders fitAI placeholder', (tester) async {
    await tester.pumpWidget(const MaterialApp(home: HomeScreen()));
    expect(find.text('fitAI'), findsOneWidget);
  });
}
