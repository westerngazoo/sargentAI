import 'package:flutter/foundation.dart';

import 'draft_rejection.dart';

/// Ranges mirror `core::workout::{Reps,LoadKg,Rpe}` EXACTLY (AC3); the backend
/// stays the source of truth — these only fail the UI fast. Public so the
/// rejection messages quote the same numbers.
abstract final class SetLimits {
  static const int minReps = 1;
  static const int maxReps = 10000;

  /// The lower bound is EXCLUSIVE (> 0).
  static const double maxWeightKg = 1000.0;
  static const double minRpe = 6.0;
  static const double maxRpe = 10.0;
}

/// One logged set. `reps` is required; `weightKg` and `rpe` are optional. Each
/// validator returns `null` when valid or the typed rejection (the
/// `ProfileDraft`/`SetDraft` idiom).
@immutable
class SetDraft {
  const SetDraft({this.reps, this.weightKg, this.rpe});

  final int? reps;
  final double? weightKg;
  final double? rpe;

  DraftRejection? repsError() {
    final r = reps;
    if (r == null) return DraftRejection.repsMissing;
    if (r < SetLimits.minReps || r > SetLimits.maxReps) {
      return DraftRejection.repsOutOfRange;
    }
    return null;
  }

  /// Optional; when present must be finite and in `(0, 1000]` (0 exclusive).
  DraftRejection? weightError() {
    final w = weightKg;
    if (w == null) return null;
    if (!w.isFinite || w <= 0 || w > SetLimits.maxWeightKg) {
      return DraftRejection.weightOutOfRange;
    }
    return null;
  }

  /// Optional; when present must be finite, in `[6, 10]`, on the exact 0.5 grid.
  DraftRejection? rpeError() {
    final r = rpe;
    if (r == null) return null;
    final onGrid = r.isFinite && (r * 2) == (r * 2).truncateToDouble();
    if (!r.isFinite ||
        r < SetLimits.minRpe ||
        r > SetLimits.maxRpe ||
        !onGrid) {
      return DraftRejection.rpeOutOfRange;
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
