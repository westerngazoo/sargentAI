import 'package:flutter/foundation.dart';

import 'exercise_draft.dart';

/// The validated `POST /workouts` payload. Built only from a finishable draft.
@immutable
class SessionRequest {
  const SessionRequest({required this.performedOn, required this.exercises});

  final DateTime performedOn;
  final List<ExerciseDraft> exercises;

  Map<String, dynamic> toJson() => <String, dynamic>{
        'performed_on': _isoDate(performedOn),
        'exercises': exercises.map((e) => e.toJson()).toList(),
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

  SessionRequest? toRequest(DateTime today) {
    if (!canFinish) return null;
    return SessionRequest(performedOn: today, exercises: exercises);
  }
}

String _isoDate(DateTime d) => '${d.year.toString().padLeft(4, '0')}-'
    '${d.month.toString().padLeft(2, '0')}-'
    '${d.day.toString().padLeft(2, '0')}';
