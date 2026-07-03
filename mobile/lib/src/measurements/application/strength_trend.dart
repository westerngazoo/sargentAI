// Strength progression from the workout log — no backend needed. Per session
// we take the best estimated 1RM (Epley: weight × (1 + reps/30)) across all
// weighted sets, and the total volume (Σ reps × weight). Both trend up as the
// user gets stronger.

import 'package:flutter/foundation.dart';

import '../../workout/domain/workout_session.dart';

/// A single (date, value) sample for a chart line.
@immutable
class TrendPoint {
  const TrendPoint(this.date, this.value);
  final DateTime date;
  final double value;
}

@immutable
class StrengthTrend {
  const StrengthTrend({required this.best1rm, required this.volume});

  /// Best estimated 1RM per session, oldest first.
  final List<TrendPoint> best1rm;

  /// Total volume (kg) per session, oldest first.
  final List<TrendPoint> volume;

  bool get hasData => best1rm.length >= 2 || volume.length >= 2;
}

double _epley(int reps, double weightKg) => weightKg * (1 + reps / 30.0);

/// Derives strength/volume trends from the workout log (any order in → sorted
/// out). Sessions with no weighted set are skipped for 1RM but still counted
/// for volume when they carry weight.
StrengthTrend computeStrengthTrend(List<WorkoutSession> workouts) {
  final sorted = [...workouts]
    ..sort((a, b) => a.performedOn.compareTo(b.performedOn));

  final best1rm = <TrendPoint>[];
  final volume = <TrendPoint>[];
  for (final s in sorted) {
    double? sessionBest;
    var sessionVolume = 0.0;
    for (final ex in s.exercises) {
      for (final set in ex.sets) {
        final w = set.weightKg;
        if (w == null || w <= 0) continue;
        sessionVolume += w * set.reps;
        final e = _epley(set.reps, w);
        if (sessionBest == null || e > sessionBest) sessionBest = e;
      }
    }
    if (sessionBest != null) {
      best1rm.add(TrendPoint(s.performedOn, sessionBest));
    }
    if (sessionVolume > 0) {
      volume.add(TrendPoint(s.performedOn, sessionVolume));
    }
  }
  return StrengthTrend(best1rm: best1rm, volume: volume);
}
