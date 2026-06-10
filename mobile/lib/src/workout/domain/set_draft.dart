import 'package:flutter/foundation.dart';

// Ranges mirror `core::workout::{Reps,LoadKg,Rpe}` EXACTLY (AC3); the backend
// stays the source of truth — these only fail the UI fast.
const int _minReps = 1;
const int _maxReps = 10000;
const double _maxWeight = 1000.0; // lower bound is EXCLUSIVE (> 0)
const double _minRpe = 6.0;
const double _maxRpe = 10.0;

/// One logged set. `reps` is required; `weightKg` and `rpe` are optional. Each
/// validator returns `null` when valid or a user-safe message (the
/// `ProfileDraft`/`SetDraft` idiom).
@immutable
class SetDraft {
  const SetDraft({this.reps, this.weightKg, this.rpe});

  final int? reps;
  final double? weightKg;
  final double? rpe;

  String? repsError() {
    final r = reps;
    if (r == null) return 'enter the reps';
    if (r < _minReps || r > _maxReps) {
      return 'reps must be between $_minReps and $_maxReps';
    }
    return null;
  }

  /// Optional; when present must be finite and in `(0, 1000]` (0 exclusive).
  String? weightError() {
    final w = weightKg;
    if (w == null) return null;
    if (!w.isFinite || w <= 0 || w > _maxWeight) {
      return 'weight must be over 0 and at most ${_maxWeight.toStringAsFixed(0)} kg';
    }
    return null;
  }

  /// Optional; when present must be finite, in `[6, 10]`, on the exact 0.5 grid.
  String? rpeError() {
    final r = rpe;
    if (r == null) return null;
    final onGrid = r.isFinite && (r * 2) == (r * 2).truncateToDouble();
    if (!r.isFinite || r < _minRpe || r > _maxRpe || !onGrid) {
      return 'RPE must be between $_minRpe and $_maxRpe in 0.5 steps';
    }
    return null;
  }

  bool get valid =>
      repsError() == null && weightError() == null && rpeError() == null;

  /// Wire shape: `reps` always present; absent optionals are OMITTED, not null
  /// (the `#[serde(default)]` backend DTO).
  Map<String, dynamic> toJson() => <String, dynamic>{
        'reps': reps,
        if (weightKg != null) 'weight_kg': weightKg,
        if (rpe != null) 'rpe': rpe,
      };
}
