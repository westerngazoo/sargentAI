// R-0059 AC3, AC4 -> SPEC-0059 §2.4 (slice 59-c1): the squat check draws
// the figure with its balance line, hip lever and both vectors; play/pause
// and the scrubber change the frame shown; a stopped range says in plain
// words how far the lifter gets and why; a rejected one shows the reason
// and no figure; the dorsiflexion is asked for, with no default, before
// anything is sent. The stamp reads COMPROBABLE. Slice 59-c2a: the
// readouts, moment bars, live graph and "La cuenta" follow the frame shown;
// the caveats sit right under the figure, ahead of the readouts.

import 'package:fitai/src/core/theme/tokens.dart';
import 'package:fitai/src/physics/application/squat_check_controller.dart';
import 'package:fitai/src/physics/domain/caveat_text.dart';
import 'package:fitai/src/physics/domain/lever_text.dart';
import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:fitai/src/physics/presentation/figure_painter.dart';
import 'package:fitai/src/physics/presentation/live_graph.dart';
import 'package:fitai/src/physics/presentation/squat_check.dart';
import 'package:fitai/src/profile/application/profile_providers.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/l10n.dart';
import '../../support/physics_fakes.dart';
import '../../support/profile_fakes.dart';

/// A recorded paint's colour as ARGB: `Paint` keeps colour as floats, so a
/// round-tripped `Color` need not be `==` to the token it was set from.
int _argb(Object? paint) => (paint! as Paint).color.toARGB32();

void main() {
  setUpAll(registerSquatFallbacks);

  late MockSquatCheckApi api;

  setUp(() {
    api = MockSquatCheckApi();
    when(() => api.range(any())).thenAnswer((_) async => squatGolden());
  });

  Future<void> pump(WidgetTester tester, {double? dorsiflexion = 29}) async {
    tester.view.physicalSize = const Size(390, 1400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(ProviderScope(
      overrides: [
        squatCheckApiProvider.overrideWithValue(api),
        profileProvider.overrideWith((_) async => sampleProfile()),
        squatDorsiflexionProvider.overrideWith((_) => dorsiflexion),
      ],
      child: localizedApp(
        home: const Scaffold(
          body: SingleChildScrollView(child: SquatCheck()),
        ),
      ),
    ));
    await tester.pumpAndSettle();
  }

  final figure = find.byWidgetPredicate(
      (w) => w is CustomPaint && w.painter is SquatFigurePainter);

  SquatFigurePainter painter(WidgetTester tester) =>
      tester.widget<CustomPaint>(figure).painter! as SquatFigurePainter;

  group('the figure (AC4)', () {
    testWidgets('draws the balance line, the lever and both vectors',
        (tester) async {
      await pump(tester);

      expect(figure, findsOneWidget);
      final g = painter(tester).geometry(tester.getSize(figure));
      expect(g.balanceLine.$1.dy, greaterThan(g.balanceLine.$2.dy));
      expect(g.lever, isNotNull);
      expect(g.barForce, isNotNull);
      expect(g.ground.length, greaterThan(0));
    });

    testWidgets('paints the lever and both arrows, in their colours',
        (tester) async {
      await pump(tester);
      final g = painter(tester).geometry(tester.getSize(figure));
      final t = painter(tester).tokens;
      final render = tester.renderObject(figure);

      // One drawLine call from [from] to [to] in [color].
      PaintPatternPredicate line(Offset from, Offset to, Color color) =>
          (method, args) =>
              method == #drawLine &&
              args[0] == from &&
              args[1] == to &&
              _argb(args[2]) == color.toARGB32();
      // The lever's first ink dash: it starts at the hip and stays level.
      bool leverDash(Symbol method, List<dynamic> args) =>
          method == #drawLine &&
          args[0] == g.lever!.$1 &&
          (args[1] as Offset).dy == g.lever!.$1.dy &&
          _argb(args[2]) == t.line.toARGB32();

      // The ground arrow's paper underlay comes before its ink stroke.
      expect(
          render,
          paints
            ..something(line(g.ground.from, g.ground.to, t.panel))
            ..something(line(g.ground.from, g.ground.to, t.line)));
      expect(render, paints..something(leverDash));
      expect(
          render,
          paints
            ..something(line(
                g.barForce!.from, g.barForce!.to, t.heat(g.barForce!.heat!))));
    });

    testWidgets('opens on the last frame the server returned', (tester) async {
      await pump(tester);
      expect(painter(tester).current.frame.progress, 1.0);
    });

    testWidgets('the stamp reads COMPROBABLE, never COMPROBADO',
        (tester) async {
      await pump(tester);
      expect(find.text(l10n.squatStamp), findsOneWidget);
      expect(l10n.squatStamp, 'COMPROBABLE');
      expect(find.textContaining('COMPROBADO'), findsNothing);
    });
  });

  group('playback (AC4)', () {
    testWidgets('scrubbing changes the frame shown', (tester) async {
      await pump(tester);

      await tester.drag(find.byType(Slider), const Offset(-1000, 0));
      await tester.pump();

      expect(painter(tester).current.progress, 0.0);
      expect(painter(tester).current.frame.progress, 0.0);
    });

    testWidgets('play moves the figure, pause holds it', (tester) async {
      await pump(tester);

      await tester.tap(find.byTooltip(l10n.squatPlay));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 700));
      final playing = painter(tester).current.progress;
      expect(playing, lessThan(1.0));

      await tester.tap(find.byTooltip(l10n.squatPause));
      await tester.pump();
      final held = painter(tester).current.progress;
      await tester.pump(const Duration(milliseconds: 700));
      expect(painter(tester).current.progress, held);
    });
  });

  group('the range\'s end (AC3)', () {
    const ankle = Rejection(
      reason: RejectionReason.depthNotReachable,
      limiting: Joint.ankle,
      neededDeg: 38,
      availableDeg: 29,
    );

    testWidgets('a stopped range says how far, and why, in plain words',
        (tester) async {
      when(() => api.range(any())).thenAnswer((_) async => squatResult(
          const Stopped(0.57, ankle),
          frames: squatGoldenBodyweightFrames()));
      await pump(tester);

      expect(
          find.text('Sin carga, tu tobillo te detiene al 57 % de la bajada. '
              'El fondo pide 38° y tienes 29°.'),
          findsOneWidget);
      expect(figure, findsOneWidget);
    });

    testWidgets('with a bar the stopped line names the load', (tester) async {
      when(() => api.range(any()))
          .thenAnswer((_) async => squatResult(const Stopped(0.57, ankle)));
      await pump(tester);

      expect(
          find.text(
              '${l10n.squatStopped(l10n.squatWithLoad('84'), 'ankle', 57)} '
              '${l10n.squatStoppedDegrees(38, 29)}'),
          findsOneWidget);
    });

    testWidgets('stopped by the model\'s convention says so', (tester) async {
      when(() => api.range(any()))
          .thenAnswer((_) async => squatResult(const Stopped(0.8, null)));
      await pump(tester);

      expect(
          find.text(l10n.squatStopped(l10n.squatWithLoad('84'), 'other', 80)),
          findsOneWidget);
    });

    testWidgets('a rejected range shows the reason and no figure',
        (tester) async {
      when(() => api.range(any())).thenAnswer((_) async => squatResult(
          const Rejected(Rejection(reason: RejectionReason.doesNotBalance)),
          frames: const []));
      await pump(tester);

      expect(find.text(l10n.squatRejected('balance')), findsOneWidget);
      expect(figure, findsNothing);
      expect(find.byType(Slider), findsNothing);
    });

    testWidgets('a bottom range has no stop line', (tester) async {
      await pump(tester);
      expect(find.textContaining('% de la bajada'), findsNothing);
    });
  });

  group('the dorsiflexion (SPEC-0045 §2.6.1.1: no default)', () {
    final field = find.widgetWithText(TextField, l10n.squatDorsiflexionLabel);

    testWidgets('is asked for, and nothing is sent until it is typed',
        (tester) async {
      await pump(tester, dorsiflexion: null);

      expect(field, findsOneWidget);
      expect(figure, findsNothing);
      verifyNever(() => api.range(any()));

      await tester.enterText(field, '29');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();

      final req = verify(() => api.range(captureAny())).captured.single
          as SquatRangeRequest;
      expect(req.dorsiflexionDeg, 29);
      expect(figure, findsOneWidget);
    });

    testWidgets('a value outside 0–90 is refused before it is sent',
        (tester) async {
      await pump(tester, dorsiflexion: null);

      await tester.enterText(field, '120');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();

      expect(find.text(l10n.squatDorsiflexionInvalid), findsOneWidget);
      verifyNever(() => api.range(any()));
    });
  });

  group('the load', () {
    final field = find.widgetWithText(TextField, l10n.squatLoadLabel);

    testWidgets('a typed bar is sent; an empty field is bodyweight',
        (tester) async {
      await pump(tester);
      clearInteractions(api);

      await tester.enterText(field, '100');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();
      var req = verify(() => api.range(captureAny())).captured.single
          as SquatRangeRequest;
      expect(req.barKg, 100);

      await tester.enterText(field, '');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();
      req = verify(() => api.range(captureAny())).captured.single
          as SquatRangeRequest;
      expect(req.barKg, isNull);
    });
  });
  group('readouts, bars and the graph follow the frame (AC4)', () {
    Future<void> scrubToStart(WidgetTester tester) async {
      await tester.drag(find.byType(Slider), const Offset(-1000, 0));
      await tester.pump();
    }

    Finder readout(String key, String value) =>
        find.descendant(of: find.byKey(Key(key)), matching: find.text(value));

    testWidgets('the readouts show the frame\'s numbers', (tester) async {
      await pump(tester);
      final bottom = squatGolden().frames.last;

      expect(readout('readout-hip', '438'), findsOneWidget);
      expect(readout('readout-lever', '38'), findsOneWidget);
      expect(readout('readout-knee', '201'), findsOneWidget);
      expect(readout('readout-trunk', '51'), findsOneWidget);
      expect(
          find.text(l10n.squatAngles('${bottom.kneeFlexionDeg.round()}',
              '${bottom.hipFlexionDeg.round()}', 100)),
          findsOneWidget);

      await scrubToStart(tester);
      expect(readout('readout-hip', '34'), findsOneWidget);
    });

    testWidgets('a bodyweight range has no lever readout', (tester) async {
      when(() => api.range(any())).thenAnswer((_) async =>
          squatResult(const Bottom(), frames: squatGoldenBodyweightFrames()));
      await pump(tester);
      expect(find.byKey(const Key('readout-lever')), findsNothing);
      expect(find.byKey(const Key('readout-hip')), findsOneWidget);
    });

    testWidgets('a negative moment draws left of zero, in grey',
        (tester) async {
      await pump(tester);
      await scrubToStart(tester);

      final zero = tester.getRect(find.byKey(const Key('moment-zero-knee')));
      final knee = tester.getRect(find.byKey(const Key('moment-bar-knee')));
      final hip = tester.getRect(find.byKey(const Key('moment-bar-hip')));
      expect(knee.right, lessThanOrEqualTo(zero.left + 0.5));
      expect(hip.left, greaterThanOrEqualTo(zero.left - 0.5));
      final bar =
          tester.widget<ColoredBox>(find.byKey(const Key('moment-bar-knee')));
      expect(bar.color, FbfTokens.light.muted);
      expect(find.text('-71 N·m'), findsOneWidget);
    });

    testWidgets('the readout boxes: heat on the torques, not on the lever',
        (tester) async {
      await pump(tester);
      Color edge(String key) {
        final box = tester.widget<Container>(find
            .descendant(
                of: find.byKey(Key(key)), matching: find.byType(Container))
            .first);
        return ((box.decoration! as BoxDecoration).border! as Border)
            .left
            .color;
      }

      final t = FbfTokens.light;
      expect(edge('readout-hip'), t.heat(1));
      expect(edge('readout-lever'), t.line);
      expect(edge('readout-trunk'), t.line);
    });

    testWidgets('the graph\'s point follows the frame', (tester) async {
      await pump(tester);
      LiveGraphPainter graph() => tester
          .widget<CustomPaint>(find.byWidgetPredicate(
              (w) => w is CustomPaint && w.painter is LiveGraphPainter))
          .painter! as LiveGraphPainter;
      expect(graph().current.progress, 1.0);
      await scrubToStart(tester);
      expect(graph().current.progress, 0.0);
    });

    testWidgets('La cuenta shows the server\'s sum for the frame',
        (tester) async {
      await pump(tester);
      expect(find.text(leverText(squatGolden().frames.last, l10n)!),
          findsOneWidget);
    });
  });

  group('the caveats (AC5)', () {
    testWidgets('sit right under the figure, ahead of the readouts',
        (tester) async {
      await pump(tester);
      final golden = squatGolden();
      final caveats = find.byKey(const Key('squat-caveats'));
      expect(caveats, findsOneWidget);
      for (final line in caveatLines(golden, golden.frames.last, l10n)) {
        expect(find.descendant(of: caveats, matching: find.text(line)),
            findsOneWidget,
            reason: line);
      }
      final top = tester.getRect(caveats).top;
      expect(top, greaterThan(tester.getRect(figure).bottom));
      expect(top,
          lessThan(tester.getRect(find.byKey(const Key('readout-hip'))).top));
    });
  });
}
