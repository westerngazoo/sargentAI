// R-0061 AC1 -> SPEC-0061 §2.6: a nested flow inside a panel. Back and Esc
// step back within the flow and close the panel only from its first page; a
// tap outside closes the whole panel from any page (N11: the barrier pops the
// panel route, it never `maybePop`s into the flow).

import 'package:fitai/src/core/panels/panel_flow.dart';
import 'package:fitai/src/core/panels/show_panel.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/l10n.dart';

void main() {
  /// A host whose panel holds a two-page flow: 'first' pushes 'second'.
  Future<void> pumpHost(WidgetTester tester, Size size) async {
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(localizedApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: Center(
            child: ElevatedButton(
              onPressed: () => showPanel<void>(
                context,
                title: 'Panel',
                scrolls: false,
                body: (_) => PanelFlow(
                  initialPage: Builder(
                    builder: (context) => TextButton(
                      onPressed: () => Navigator.of(context).push(
                        PanelFlowRoute<void>(
                          builder: (_) => const Text('second'),
                        ),
                      ),
                      child: const Text('first'),
                    ),
                  ),
                ),
              ),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    ));
  }

  Future<void> openSecondPage(WidgetTester tester) async {
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('first'));
    await tester.pumpAndSettle();
    expect(find.text('second'), findsOneWidget);
  }

  Future<void> systemBack(WidgetTester tester) async {
    final dynamic widgetsAppState = tester.state(find.byType(WidgetsApp));
    await widgetsAppState.didPopRoute();
    await tester.pumpAndSettle();
  }

  for (final (name, size) in [
    ('phone', const Size(390, 844)),
    ('desktop', const Size(1440, 900)),
  ]) {
    group(name, () {
      testWidgets('back steps within the flow, then closes the panel',
          (tester) async {
        await pumpHost(tester, size);
        await openSecondPage(tester);

        await systemBack(tester);
        expect(find.text('second'), findsNothing);
        expect(find.text('first'), findsOneWidget, reason: 'panel still open');

        await systemBack(tester);
        expect(find.text('first'), findsNothing);
        expect(find.text('open'), findsOneWidget, reason: 'host survives');
      });

      testWidgets('Esc steps within the flow, then closes the panel',
          (tester) async {
        await pumpHost(tester, size);
        await openSecondPage(tester);

        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
        expect(find.text('second'), findsNothing);
        expect(find.text('first'), findsOneWidget);

        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
        expect(find.text('first'), findsNothing);
      });

      testWidgets('a tap outside closes the whole panel from the second page',
          (tester) async {
        await pumpHost(tester, size);
        await openSecondPage(tester);

        await tester.tapAt(const Offset(8, 8));
        await tester.pumpAndSettle();
        expect(find.text('second'), findsNothing);
        expect(find.text('first'), findsNothing);
        expect(find.text('open'), findsOneWidget);
      });

      testWidgets('✕ closes the whole panel from the second page',
          (tester) async {
        await pumpHost(tester, size);
        await openSecondPage(tester);

        await tester.tap(find.byKey(const Key('panel-close')));
        await tester.pumpAndSettle();
        expect(find.text('second'), findsNothing);
        expect(find.text('first'), findsNothing);
      });
    });
  }
}
