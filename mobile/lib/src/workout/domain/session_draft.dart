import 'package:flutter/foundation.dart';

import 'exercise_draft.dart';
import 'plan_seed.dart';

/// The validated `POST /workouts` payload. Built only from a finishable draft.
@immutable
class SessionRequest {
  const SessionRequest({
    required this.performedOn,
    required this.exercises,
    this.plan,
  });

  final DateTime performedOn;
  final List<ExerciseDraft> exercises;

  /// The plan day this session completes, or null for a free workout. One
  /// value carries both wire fields, so they come both or neither
  /// (SPEC-0061 §2.4).
  final PlanSeed? plan;

  /// The same session with the plan link dropped: what is sent when the
  /// server refuses the link (§2.4).
  SessionRequest asFree() =>
      SessionRequest(performedOn: performedOn, exercises: exercises);

  Map<String, dynamic> toJson() => <String, dynamic>{
        'performed_on': _isoDate(performedOn),
        'exercises': exercises.map((e) => e.toJson()).toList(),
        if (plan != null) 'user_program_id': plan!.userProgramId,
        if (plan != null) 'plan_day_index': plan!.index,
      };
}

/// The in-progress session. `canFinish` mirrors the backend's structural rules
/// (`WorkoutError::{ExercisesEmpty,SetsEmpty}`); `toRequest` is **total**
/// (`null` until finishable, no precondition throw).
@immutable
class SessionDraft {
  const SessionDraft({this.exercises = const []});

  final List<ExerciseDraft> exercises;

  bool get canFinish =>
      exercises.isNotEmpty && exercises.every((e) => e.sets.isNotEmpty);

  /// Whether leaving would lose logged work (SPEC-0061 §2.7): a seeded
  /// session with no set yet has nothing to lose.
  bool get hasSets => exercises.any((e) => e.sets.isNotEmpty);

  /// The request for the session performed on [today], linked to [plan]
  /// when it is a plan session.
  SessionRequest? toRequest(DateTime today, {PlanSeed? plan}) {
    if (!canFinish) return null;
    return SessionRequest(performedOn: today, exercises: exercises, plan: plan);
  }
}

String _isoDate(DateTime d) => '${d.year.toString().padLeft(4, '0')}-'
    '${d.month.toString().padLeft(2, '0')}-'
    '${d.day.toString().padLeft(2, '0')}';
