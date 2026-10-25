// SPEC-0061 §2.10: the halftone ground paints the theme's ink at low alpha
// over the ground colour. Paint-colour assertions, not pixel goldens.

import 'package:fitai/src/core/theme/halftone.dart';
import 'package:fitai/src/core/theme/tokens.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('the painter takes its colours from the tokens and repaints on change',
      () {
    final light = HalftonePainter(
        ink: FbfTokens.light.line, paper: FbfTokens.light.ground);
    final dark =
        HalftonePainter(ink: FbfTokens.dark.line, paper: FbfTokens.dark.ground);
    expect(light.alpha, lessThan(0.15), reason: 'faint, never a texture');
    expect(light.shouldRepaint(dark), isTrue);
    expect(light.shouldRepaint(HalftonePainter(
        ink: FbfTokens.light.line, paper: FbfTokens.light.ground)), isFalse);
  });

  testWidgets('HalftoneBackground reads the theme extension', (tester) async {
    await tester.pumpWidget(MaterialApp(
      theme: ThemeData(extensions: const [FbfTokens.dark]),
      home: const HalftoneBackground(child: SizedBox.expand()),
    ));
    final paint = tester.widget<CustomPaint>(find.byType(CustomPaint).first);
    final painter = paint.painter! as HalftonePainter;
    expect(painter.ink, FbfTokens.dark.line);
    expect(painter.paper, FbfTokens.dark.ground);
  });
}
