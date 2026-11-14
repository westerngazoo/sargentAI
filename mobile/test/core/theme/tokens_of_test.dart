// SPEC-0061 §2.10: one lookup for the theme's tokens, light when a theme
// carries none (as in a bare test app).

import 'package:fitai/src/core/theme/app_theme.dart';
import 'package:fitai/src/core/theme/tokens.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  Future<FbfTokens> tokensIn(WidgetTester tester, ThemeData? theme) async {
    late FbfTokens found;
    await tester.pumpWidget(MaterialApp(
      theme: theme,
      home: Builder(builder: (context) {
        found = FbfTokens.of(context);
        return const SizedBox();
      }),
    ));
    return found;
  }

  testWidgets('reads the theme\'s tokens', (tester) async {
    expect(await tokensIn(tester, AppTheme.dark()), same(FbfTokens.dark));
  });

  testWidgets('falls back to light when the theme carries none',
      (tester) async {
    expect(await tokensIn(tester, ThemeData()), same(FbfTokens.light));
  });
}
