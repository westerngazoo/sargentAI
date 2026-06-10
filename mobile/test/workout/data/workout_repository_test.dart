// SAC10 -> AC10 (layering): WorkoutRepository is the seam the providers, the
// session driver, and the list controller depend on — one hop from transport,
// mirroring ProfileRepository. It delegates verbatim; no logic lives here.
//
// RED until package:fitai/src/workout/data/workout_repository.dart defines
// WorkoutRepository(WorkoutApi) and workoutRepositoryProvider.

import 'package:fitai/src/workout/data/workout_repository.dart';
import 'package:fitai/src/workout/domain/exercise_draft.dart';
import 'package:fitai/src/workout/domain/session_draft.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/workout_fakes.dart';

void main() {
  setUpAll(registerWorkoutFallbacks);

  late MockWorkoutApi api;
  late WorkoutRepository repo;

  setUp(() {
    api = MockWorkoutApi();
    repo = WorkoutRepository(api);
  });

  test('list() delegates to the api and passes the sessions through', () async {
    when(() => api.list()).thenAnswer((_) async => [sampleSession(id: 's-1')]);
    final sessions = await repo.list();
    expect(sessions.single.id, 's-1');
    verify(() => api.list()).called(1);
  });

  test('create() delegates the SAME request instance to the api', () async {
    final req = SessionDraft(exercises: [
      ExerciseDraft(name: 'Squat', sets: const [SetDraft(reps: 5)]),
    ]).toRequest(DateTime(2026, 6, 10))!;
    when(() => api.create(req)).thenAnswer((_) async => sampleSession());

    await repo.create(req);

    verify(() => api.create(req)).called(1);
  });

  test('delete() delegates the id to the api', () async {
    when(() => api.delete('s-9')).thenAnswer((_) async {});
    await repo.delete('s-9');
    verify(() => api.delete('s-9')).called(1);
  });
}
