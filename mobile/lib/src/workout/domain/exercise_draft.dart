import 'package:flutter/foundation.dart';

import 'draft_rejection.dart';
import 'muscle_group.dart';
import 'set_draft.dart';

/// The name bound, mirrored by the rejection message so they cannot drift.
abstract final class ExerciseLimits {
  static const int maxNameScalars = 100;
}

/// One exercise within a session: a name, an optional muscle group, and ordered
/// sets. `nameError` mirrors `core::workout::ExerciseName::try_new` exactly —
/// trimmed first, then non-empty and counted in **Unicode scalars**
/// (`runes.length`, the backend's `chars()`), not UTF-16 `String.length`.
@immutable
class ExerciseDraft {
  const ExerciseDraft({
    required this.name,
    this.muscleGroup,
    this.sets = const [],
  });

  final String name;
  final MuscleGroup? muscleGroup;
  final List<SetDraft> sets;

  DraftRejection? nameError() {
    final trimmed = name.trim();
    if (trimmed.isEmpty) return DraftRejection.nameEmpty;
    if (trimmed.runes.length > ExerciseLimits.maxNameScalars) {
      return DraftRejection.nameTooLong;
    }
    return null;
  }

  ExerciseDraft copyWith({List<SetDraft>? sets}) => ExerciseDraft(
        name: name,
        muscleGroup: muscleGroup,
        sets: sets ?? this.sets,
      );

  /// Wire shape: trimmed `name`, `muscle_group` omitted when absent, ordered
  /// `sets`.
  Map<String, dynamic> toJson() => <String, dynamic>{
        'name': name.trim(),
        if (muscleGroup != null) 'muscle_group': muscleGroup!.wire,
        'sets': sets.map((s) => s.toJson()).toList(),
      };
}
