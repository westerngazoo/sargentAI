// R-0059 AC4, AC7 -> SPEC-0059 §2.4 (slice 59-c1): the figure's geometry.
// One metres-to-canvas fit for the whole range, so the figure never jumps;
// the balance line on the mid-foot; the hip lever from the hip to the bar's
// line; the bar and ground vectors to one scale chosen from the largest
// `ground_n`. Every magnitude is the returned frame's. Pure: no mocks.

import 'dart:ui';

import 'package:fitai/src/physics/domain/playback.dart';
import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:fitai/src/physics/presentation/figure_geometry.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/physics_fakes.dart';

void main() {
  const size = Size(240, 320);
  final range = squatGolden().frames; // progress 0 and 1, bar 84 kg
  final [top, bottom] = range;

  FigureGeometry at(RangeFrame f, {List<RangeFrame>? over}) => figureGeometry(
      over ?? range, playbackAt(over ?? range, f.progress)!, size);

  bool inside(Offset p) =>
      p.dx >= 0 && p.dx <= size.width && p.dy >= 0 && p.dy <= size.height;

  group('the fit', () {
    test('keeps every joint and the bar of every frame on the canvas', () {
      final fit = fitRange(range, size);
      for (final f in range) {
        final p = f.posture;
        for (final m in [p.ankle, p.knee, p.hip, p.shoulder, p.load!.at]) {
          expect(inside(fit.toCanvas(m)), isTrue);
        }
      }
    });

    test('is one scale for the whole range, so the figure does not jump', () {
      // The ankle does not move between the golden's two frames.
      expect(at(top).ankle, at(bottom).ankle);
      expect(at(top).groundY, at(bottom).groundY);
    });

    test('draws metres up as canvas up', () {
      final g = at(top);
      expect(g.shoulder.dy, lessThan(g.hip.dy));
      expect(g.hip.dy, lessThan(g.knee.dy));
      expect(g.head.dy, lessThan(g.shoulder.dy));
      expect(g.ankle.dy, lessThan(g.groundY));
    });
  });

  group('the balance line', () {
    test('rises from the mid-foot on the ground', () {
      final g = at(bottom);
      final (from, to) = g.balanceLine;
      final midFoot = fitRange(range, size).toCanvas(const Point(0, 0));
      expect(from, midFoot);
      expect(to.dx, from.dx);
      expect(to.dy, lessThan(g.head.dy));
    });
  });

  group('the hip lever', () {
    test('runs level from the hip to the bar line, the server lever long', () {
      final g = at(bottom);
      final (from, to) = g.lever!;
      expect(from, g.hip);
      expect(to.dy, g.hip.dy);
      expect(to.dx, g.bar!.dx);
      final scale = fitRange(range, size).scale;
      expect(
          (to.dx - from.dx).abs(), closeTo(bottom.hip!.leverM * scale, 1e-6));
    });

    test('is absent for bodyweight only', () {
      final bw = squatGoldenBodyweightFrames();
      final g = at(bw.last, over: bw);
      expect(g.lever, isNull);
      expect(g.bar, isNull);
      expect(g.barForce, isNull);
    });
  });

  group('the force vectors', () {
    test('the largest ground force spans the chosen length', () {
      final nPerPx = newtonsPerPixel(range, size);
      expect(
          nPerPx, closeTo(1579.41 / (maxVectorFraction * size.height), 1e-9));
      final g = at(top);
      expect(g.ground.length, closeTo(maxVectorFraction * size.height, 1e-9));
    });

    test('ground and bar share one scale, from the frame\'s own numbers', () {
      final g = at(bottom);
      expect(g.barForce!.length / g.ground.length,
          closeTo(bottom.hip!.barN / bottom.groundN, 1e-9));
    });

    test('the ground pushes up from the mid-foot, the bar pulls down', () {
      final g = at(bottom);
      expect(g.ground.from, g.balanceLine.$1);
      expect(g.ground.to.dy, lessThan(g.ground.from.dy));
      expect(g.barForce!.from.dx, g.bar!.dx);
      expect(g.barForce!.to.dy, greaterThan(g.barForce!.from.dy));
    });

    test('heat is the force against the largest ground force', () {
      final g = at(bottom);
      expect(g.ground.heat, closeTo(1.0, 1e-9));
      expect(g.barForce!.heat, closeTo(bottom.hip!.barN / 1579.41, 1e-9));
    });
  });

  group('the body', () {
    test('with a bar the arms reach it; without one they hang', () {
      final loaded = at(bottom);
      expect(loaded.arm.last, loaded.bar);

      final bw = squatGoldenBodyweightFrames();
      final hanging = at(bw.first, over: bw);
      expect(hanging.arm.first, hanging.shoulder);
      expect(hanging.arm.last.dx, hanging.shoulder.dx);
      expect(hanging.arm.last.dy, greaterThan(hanging.shoulder.dy));
    });

    test('the plate is drawn to scale around the bar', () {
      final g = at(top);
      expect(g.plateRadius,
          closeTo(plateRadiusM * fitRange(range, size).scale, 1e-9));
    });
  });
}
