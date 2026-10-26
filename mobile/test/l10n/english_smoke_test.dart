// R-0061 AC11: the English catalogue is wired end to end — pumping a widget
// under `en` renders the English entry, not the Spanish template.

import 'package:fitai/l10n/app_localizations.dart';
import 'package:fitai/src/l10n/locale_rule.dart';
import 'package:fitai/src/nutrition/presentation/meal_quick_log_sheet.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('the meal form reads the English catalogue under en',
      (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          locale: const Locale('en'),
          supportedLocales: supportedLocales,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          home: const Scaffold(body: MealQuickLogForm()),
        ),
      ),
    );
    await tester.pump();

    expect(find.text('Protein (g)'), findsOneWidget);
    expect(find.text('Save meal'), findsOneWidget);
    expect(find.text('Guardar comida'), findsNothing);
  });
}
