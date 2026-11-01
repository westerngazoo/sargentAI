// R-0059 AC4 -> SPEC-0059 §2.4 (slice 59-c2a): one bar per group; a
// positive moment runs right of the zero line heated by its size, a negative
// one (the load assisting extension) runs left of it with no heat.

import 'package:fitai/src/physics/presentation/moment_bars.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('a positive moment starts at zero and is heated by its size', () {
    final bar = momentBar(220, 440);
    expect(bar.left, momentZero);
    expect(bar.width, closeTo((1 - momentZero) / 2, 1e-12));
    expect(bar.heat, closeTo(0.5, 1e-12));
  });

  test('the largest moment fills the track', () {
    final bar = momentBar(-440, 440);
    expect(bar.left + bar.width, momentZero);
    expect(bar.left, 0);
    expect(bar.heat, isNull);
  });

  test('a negative moment ends at zero, left of it, with no heat', () {
    final bar = momentBar(-71.49, 438.45);
    expect(bar.left + bar.width, closeTo(momentZero, 1e-12));
    expect(bar.width, greaterThan(0));
    expect(bar.heat, isNull);
  });

  test('a zero scale draws nothing rather than dividing by it', () {
    final bar = momentBar(0, 0);
    expect(bar.width, 0);
  });
}
