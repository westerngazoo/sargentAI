// R-0059 AC4 -> SPEC-0059 §2.4 (slice 59-c2a): the live graph plots the
// server's hip and knee moments against bar height, standing at the left and
// the bottom at the right, as design v2 draws it. Geometry only.

import 'package:fitai/src/core/theme/tokens.dart';
import 'package:fitai/src/physics/domain/playback.dart';
import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:fitai/src/physics/presentation/live_graph.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/physics_fakes.dart';

void main() {
  const plot = Rect.fromLTWH(40, 10, 280, 100);
  final frames = squatGolden().frames;
  final scale = GraphScale(frames, plot);

  test('standing is at the left, the bottom at the right', () {
    expect(scale.at(frames.first, Joint.hip).dx, plot.left);
    expect(scale.at(frames.last, Joint.hip).dx, plot.right);
  });

  test('a larger moment sits higher', () {
    final up = scale.at(frames.first, Joint.hip).dy;
    final down = scale.at(frames.last, Joint.hip).dy;
    expect(down, lessThan(up));
    expect(scale.at(frames.last, Joint.hip).dy, plot.top);
  });

  test('a negative moment sits below the zero line, inside the plot', () {
    final knee = scale.at(frames.first, Joint.knee).dy;
    expect(knee, greaterThan(scale.zeroY));
    expect(knee, plot.bottom);
    expect(scale.zeroY, inExclusiveRange(plot.top, plot.bottom));
  });

  test('bodyweight frames use the shoulder\'s height', () {
    final bw = squatGoldenBodyweightFrames();
    final s = GraphScale(bw, plot);
    expect(s.at(bw.first, Joint.hip).dx, plot.left);
    expect(s.at(bw.last, Joint.hip).dx, plot.right);
  });

  test('one frame sits in the middle instead of dividing by zero', () {
    final one = [frames.last];
    expect(GraphScale(one, plot).at(one.first, Joint.hip).dx, plot.center.dx);
  });

  group('the painter', () {
    final light = FbfTokens.light;

    PaintPatternPredicate circle(Color fill) => (method, args) =>
        method == #drawCircle &&
        (args[2] as Paint).style == PaintingStyle.fill &&
        (args[2] as Paint).color.toARGB32() == fill.toARGB32();

    void paintAt(RangeFrame f, Matcher expected) {
      final painter = LiveGraphPainter(
          frames: frames,
          current: PlaybackFrame(f.progress, f.posture, f),
          tokens: light);
      expect((Canvas c) => painter.paint(c, const Size(300, 150)), expected);
    }

    test('the hip point is heated by the frame\'s moment, not always red', () {
      // Standing, the hip moment is small: the point is not the ramp's top.
      final standing = frames.first;
      final heat = standing.mechanics.moment(Joint.hip) /
          frames.last.mechanics.moment(Joint.hip);
      paintAt(standing, paints..something(circle(light.heat(heat))));
      paintAt(frames.last, paints..something(circle(light.heat(1))));
    });
  });
}
