// R-0061 AC1 -> SPEC-0061 §2.6: every panel closes on ✕, on a tap outside,
// on Esc and on back, and the route under it is never replaced. A panel that
// does not scroll (`scrolls: false`) gives its body the frame's full height,
// for a body that scrolls itself.

import 'package:fitai/src/core/panels/show_panel.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/l10n.dart';

void main() {
  Future<void> pumpHost(WidgetTester tester, Size size,
      {bool scrolls = true}) async {
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
                scrolls: scrolls,
                body: (_) => scrolls
                    ? const Text('panel-body')
                    : ListView(
                        key: const Key('panel-list'),
                        children: const [Text('panel-body')],
                      ),
              ),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    ));
  }

  for (final (name, size) in [
    ('phone', const Size(390, 844)),
    ('desktop', const Size(1440, 900)),
  ]) {
    group(name, () {
      testWidgets('opens with its body and a ✕, and ✕ closes it',
          (tester) async {
        await pumpHost(tester, size);
        await tester.tap(find.text('open'));
        await tester.pumpAndSettle();
        expect(find.text('panel-body'), findsOneWidget);
        expect(find.text('Panel'), findsOneWidget);

        await tester.tap(find.byKey(const Key('panel-close')));
        await tester.pumpAndSettle();
        expect(find.text('panel-body'), findsNothing);
        expect(find.text('open'), findsOneWidget, reason: 'host survives');
      });

      testWidgets('Esc closes it', (tester) async {
        await pumpHost(tester, size);
        await tester.tap(find.text('open'));
        await tester.pumpAndSettle();

        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
        expect(find.text('panel-body'), findsNothing);
      });

      testWidgets('a tap outside closes it', (tester) async {
        await pumpHost(tester, size);
        await tester.tap(find.text('open'));
        await tester.pumpAndSettle();

        await tester.tapAt(const Offset(8, 8));
        await tester.pumpAndSettle();
        expect(find.text('panel-body'), findsNothing);
      });

      testWidgets('back closes it', (tester) async {
        await pumpHost(tester, size);
        await tester.tap(find.text('open'));
        await tester.pumpAndSettle();

        final dynamic widgetsAppState = tester.state(find.byType(WidgetsApp));
        await widgetsAppState.didPopRoute();
        await tester.pumpAndSettle();
        expect(find.text('panel-body'), findsNothing);
      });

      testWidgets('a body that scrolls itself gets a bounded height',
          (tester) async {
        await pumpHost(tester, size, scrolls: false);
        await tester.tap(find.text('open'));
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        expect(find.text('panel-body'), findsOneWidget);
        // The frame holds no scroll view of its own, so a list in the body
        // gets a bounded height: the space under the title row.
        final list = tester.getRect(find.byKey(const Key('panel-list')));
        expect(list.height, greaterThan(100));
        expect(list.bottom, lessThanOrEqualTo(size.height));

        await tester.tap(find.byKey(const Key('panel-close')));
        await tester.pumpAndSettle();
        expect(find.text('panel-body'), findsNothing);
      });
    });
  }
}
