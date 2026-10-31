// R-0059 AC3, AC4 -> SPEC-0059 §2.4 (slice 59-c1): the squat check draws
// the figure with its balance line, hip lever and both vectors; play/pause
// and the scrubber change the frame shown; a stopped range says in plain
// words how far the lifter gets and why; a rejected one shows the reason
// and no figure; the dorsiflexion is asked for, with no default, before
// anything is sent. The stamp reads COMPROBABLE.

import 'package:fitai/src/physics/application/squat_check_controller.dart';
import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:fitai/src/physics/presentation/figure_painter.dart';
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

      expect(
          render, paints..something(line(g.ground.from, g.ground.to, t.line)));
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
          find.text('Sin carga, tu tobillo te detiene al 57 % de la bajada.'),
          findsOneWidget);
      expect(figure, findsOneWidget);
    });

    testWidgets('with a bar the stopped line names the load', (tester) async {
      when(() => api.range(any()))
          .thenAnswer((_) async => squatResult(const Stopped(0.57, ankle)));
      await pump(tester);

      expect(
          find.text(l10n.squatStopped(l10n.squatWithLoad('84'), 'ankle', 57)),
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
}
