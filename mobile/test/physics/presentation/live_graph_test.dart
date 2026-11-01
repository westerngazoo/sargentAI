// R-0059 AC4 -> SPEC-0059 §2.4 (slice 59-c2a): the live graph plots the
// server's hip and knee moments against bar height, standing at the left and
// the bottom at the right, as design v2 draws it. Geometry only.

import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:fitai/src/physics/presentation/live_graph.dart';
import 'package:flutter/painting.dart';
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
}
