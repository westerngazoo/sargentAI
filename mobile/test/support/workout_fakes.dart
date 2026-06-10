// Shared test doubles + helpers for the R-0009 live-workout-logger suite.
//
// Authored by the qa agent in step 3 (test planning) — BEFORE any
// `lib/src/workout` production code exists. These imports resolve to the exact
// class/provider surface SPEC-0009 §2/§3/§6 names, so the suite is RED until
// step-5 implementation creates them, then turns GREEN with no test edits.
//
// Targeted production symbols (all under package:fitai/src/workout/...):
//   domain/muscle_group.dart    -> MuscleGroup (chest/back/shoulders/arms/legs/core)
//   domain/set_draft.dart       -> SetDraft + repsError()/weightError()/rpeError()/valid
//   domain/exercise_draft.dart  -> ExerciseDraft + nameError() (trimmed runes ≤ 100)
//   domain/session_draft.dart   -> SessionDraft, SessionRequest (total toRequest)
//   domain/workout_session.dart -> WorkoutSession.fromJson (the R-0004 aggregate)
//   data/workout_api.dart       -> WorkoutApi(Dio): list()/create()/delete()
//   data/workout_repository.dart-> WorkoutRepository, workoutRepositoryProvider
//
// Dio-level error doubles are NOT duplicated here: reuse
// support/profile_fakes.dart `dioErrorFlat`/`dioTransport` with
// `path: '/workouts'` — the backend error body is the same flat
// `{"error":"<kind>","field":"<name>"}` shape (SPEC-0008 §2.7).

import 'package:fitai/src/workout/data/workout_api.dart';
import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:fitai/src/workout/domain/exercise_draft.dart';
import 'package:fitai/src/workout/domain/session_draft.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:fitai/src/workout/domain/workout_session.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

/// Mock typed workout client (SPEC-0009 §2.4): list()/create()/delete().
class MockWorkoutApi extends Mock implements WorkoutApi {}

/// Mock repository the workouts provider, session driver, and list controller
/// depend on.
class MockWorkoutRepository extends Mock implements WorkoutRepository {}

/// Registers the non-primitive fallback `mocktail` needs to match
/// `create(any())` — a real [SessionRequest] built from a finishable draft.
void registerWorkoutFallbacks() {
  registerFallbackValue(
    SessionDraft(
      exercises: [
        ExerciseDraft(name: 'Bench press', sets: const [SetDraft(reps: 5)]),
      ],
    ).toRequest(DateTime(2026, 6, 10))!,
  );
}

/// `'YYYY-MM-DD'`, zero-padded — the backend `NaiveDate` wire format.
String isoDate(DateTime d) => '${d.year.toString().padLeft(4, '0')}-'
    '${d.month.toString().padLeft(2, '0')}-'
    '${d.day.toString().padLeft(2, '0')}';

/// A `WorkoutSet`-shaped response JSON. Keys mirror
/// `backend/crates/core/src/workout.rs::WorkoutSet` serialization exactly;
/// absent optionals serialize as `null` (serde `None`), so the keys are
/// always present.
Map<String, dynamic> setResponseJson({
  String id = 'set-1',
  int position = 1,
  int reps = 8,
  double? weightKg,
  double? rpe,
}) =>
    <String, dynamic>{
      'id': id,
      'position': position,
      'reps': reps,
      'weight_kg': weightKg,
      'rpe': rpe,
    };

/// A `WorkoutExercise`-shaped response JSON (same key authority as above).
Map<String, dynamic> exerciseResponseJson({
  String id = 'ex-1',
  int position = 1,
  String name = 'Bench press',
  String? muscleGroup,
  List<Map<String, dynamic>>? sets,
}) =>
    <String, dynamic>{
      'id': id,
      'position': position,
      'name': name,
      'muscle_group': muscleGroup,
      'sets': sets ?? [setResponseJson()],
    };

/// A `WorkoutSession`-shaped response JSON — the `GET /workouts` element and
/// the `POST /workouts` 201 body.
Map<String, dynamic> sessionResponseJson({
  String id = 'sess-1',
  String userId = 'user-123',
  String performedOn = '2026-06-10',
  List<Map<String, dynamic>>? exercises,
  String createdAt = '2026-06-10T12:00:00Z',
  String updatedAt = '2026-06-10T12:00:00Z',
}) =>
    <String, dynamic>{
      'id': id,
      'user_id': userId,
      'performed_on': performedOn,
      'exercises': exercises ?? [exerciseResponseJson()],
      'created_at': createdAt,
      'updated_at': updatedAt,
    };

/// Parsed `WorkoutSession` convenience for provider-override / mock returns.
WorkoutSession sampleSession({
  String id = 'sess-1',
  String performedOn = '2026-06-10',
  List<Map<String, dynamic>>? exercises,
}) =>
    WorkoutSession.fromJson(
      sessionResponseJson(
        id: id,
        performedOn: performedOn,
        exercises: exercises,
      ),
    );

/// Case-insensitive substring [Text] finder — AC8 pins the *phrase*
/// ("no workouts yet"), not its casing.
Finder textIgnoringCase(String needle) => find.byWidgetPredicate(
      (w) =>
          w is Text &&
          (w.data ?? '').toLowerCase().contains(needle.toLowerCase()),
    );
