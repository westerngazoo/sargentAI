// SPEC-0059 §2.4, SPEC-0061 §2.10 — the one normaliser for the tension
// ramp: heat is a moment's magnitude against the range's largest. Display
// only (R-0059 AC7): it compares the server's moments, it computes none.

import 'dart:math' as math;

import 'squat_range.dart';

/// The largest joint moment magnitude, N·m, over every joint of every frame
/// in [frames]; negative moments count by their size. `0` when [frames] is
/// empty or every moment is zero, so a caller dividing by it checks first.
double maxAbsMoment(List<RangeFrame> frames) => frames
    .expand((f) => f.mechanics.joints)
    .map((j) => j.moment.abs())
    .fold(0.0, math.max);
