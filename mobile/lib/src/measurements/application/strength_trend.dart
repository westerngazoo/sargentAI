// Strength progression from the workout log — no backend needed. Per session
// we take the best estimated 1RM (Epley: weight × (1 + reps/30)) across all
// weighted sets, and the total volume (Σ reps × weight). Both trend up as the
// user gets stronger.

import 'package:flutter/foundation.dart';

import '../../workout/domain/muscle_group.dart';
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

/// One lift's progression, with its current best estimated 1RM and whether
/// the latest session set a new personal record.
@immutable
class LiftTrend {
  const LiftTrend({
    required this.name,
    required this.best1rm,
    required this.sessions,
  });

  final String name;

  /// Best estimated 1RM per session that included this lift, oldest first.
  final List<TrendPoint> best1rm;

  /// Number of sessions this lift appeared in (with weight).
  final int sessions;

  double get currentE1rm => best1rm.isEmpty ? 0 : best1rm.last.value;

  double get peakE1rm =>
      best1rm.isEmpty ? 0 : best1rm.map((p) => p.value).reduce(_max);

  /// True when the latest session equals the all-time peak (a fresh PR).
  bool get isPr => best1rm.length >= 2 && (currentE1rm - peakE1rm).abs() < 1e-6;

  /// Gain from first to latest recorded session (kg).
  double get gain =>
      best1rm.length < 2 ? 0 : best1rm.last.value - best1rm.first.value;
}

double _max(double a, double b) => a > b ? a : b;

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

/// Per-lift estimated-1RM trends, keyed by exercise name (case-insensitive,
/// display form preserved). Only lifts with ≥2 weighted sessions are kept
/// (a trend needs two points); ordered by most sessions, then biggest gain.
List<LiftTrend> computePerLiftTrends(List<WorkoutSession> workouts) {
  final sorted = [...workouts]
    ..sort((a, b) => a.performedOn.compareTo(b.performedOn));

  // name(lower) -> (display name, points)
  final byLift = <String, ({String display, List<TrendPoint> pts})>{};
  for (final s in sorted) {
    // Best e1RM for this lift *within* the session (across its sets).
    final sessionBest = <String, double>{};
    final display = <String, String>{};
    for (final ex in s.exercises) {
      final key = ex.name.trim().toLowerCase();
      if (key.isEmpty) continue;
      display.putIfAbsent(key, () => ex.name.trim());
      for (final set in ex.sets) {
        final w = set.weightKg;
        if (w == null || w <= 0) continue;
        final e = _epley(set.reps, w);
        final prev = sessionBest[key];
        if (prev == null || e > prev) sessionBest[key] = e;
      }
    }
    for (final entry in sessionBest.entries) {
      final b = byLift.putIfAbsent(
        entry.key,
        () => (display: display[entry.key]!, pts: <TrendPoint>[]),
      );
      b.pts.add(TrendPoint(s.performedOn, entry.value));
    }
  }

  final out = <LiftTrend>[];
  for (final e in byLift.entries) {
    if (e.value.pts.length < 2) continue;
    out.add(LiftTrend(
      name: e.value.display,
      best1rm: e.value.pts,
      sessions: e.value.pts.length,
    ));
  }
  out.sort((a, b) {
    final s = b.sessions.compareTo(a.sessions);
    return s != 0 ? s : b.gain.compareTo(a.gain);
  });
  return out;
}

/// Total training volume (Σ reps × weight) per muscle group, for a training-
/// balance view. Untagged exercises are skipped. Highest first.
List<({MuscleGroup group, double volume})> computeMuscleVolume(
    List<WorkoutSession> workouts) {
  final totals = <MuscleGroup, double>{};
  for (final s in workouts) {
    for (final ex in s.exercises) {
      final g = ex.muscleGroup;
      if (g == null) continue;
      for (final set in ex.sets) {
        final w = set.weightKg;
        if (w == null || w <= 0) continue;
        totals[g] = (totals[g] ?? 0) + w * set.reps;
      }
    }
  }
  final out = totals.entries
      .map((e) => (group: e.key, volume: e.value))
      .toList()
    ..sort((a, b) => b.volume.compareTo(a.volume));
  return out;
}
