// SPEC-0059 §2.4, SPEC-0061 §2.10: the tension ramp's normaliser is the
// range's largest joint moment magnitude. Pure: no mocks.

import 'package:fitai/src/physics/domain/moment_scale.dart';
import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/physics_fakes.dart';

RangeFrame _withMoments(RangeFrame f, List<double> moments) => RangeFrame(
      progress: f.progress,
      kneeFlexionDeg: f.kneeFlexionDeg,
      hipFlexionDeg: f.hipFlexionDeg,
      trunkLeanDeg: f.trunkLeanDeg,
      posture: f.posture,
      mechanics: Mechanics(assumptions: const [], joints: [
        for (final (i, m) in moments.indexed)
          JointMoment(joint: Joint.values[i], resisting: 'x', moment: m),
      ]),
      groundN: f.groundN,
    );

void main() {
  final golden = squatGolden().frames.first;

  test('is the largest magnitude over every joint of every frame', () {
    final frames = [
      _withMoments(golden, [10, -20, 30, 5]),
      _withMoments(golden, [120, 40, -60, 0]),
    ];
    expect(maxAbsMoment(frames), 120);
  });

  test('a negative moment counts by its size', () {
    expect(
        maxAbsMoment([
          _withMoments(golden, [10, -250, 30, 5])
        ]),
        250);
  });

  test('over the golden is its largest |moment|', () {
    final expected = squatGoldenJson()['frames']
        .expand((f) => f['mechanics']['joints'] as List)
        .map((j) => (j['moment'] as num).toDouble().abs())
        .reduce((a, b) => a > b ? a : b);
    expect(maxAbsMoment(squatGolden().frames), expected);
  });

  test('no frames is zero', () {
    expect(maxAbsMoment(const []), 0);
  });
}
