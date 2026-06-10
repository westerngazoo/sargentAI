import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/network/dio_provider.dart';
import '../domain/session_draft.dart';
import '../domain/workout_session.dart';
import 'workout_api.dart';

/// The seam the providers, session driver, and list controller depend on — one
/// hop from transport, mirroring `ProfileRepository`. Delegates verbatim.
class WorkoutRepository {
  const WorkoutRepository(this._api);

  final WorkoutApi _api;

  Future<List<WorkoutSession>> list() => _api.list();

  Future<WorkoutSession> create(SessionRequest req) => _api.create(req);

  Future<void> delete(String id) => _api.delete(id);
}

final workoutApiProvider =
    Provider<WorkoutApi>((ref) => WorkoutApi(ref.read(dioProvider)));

final workoutRepositoryProvider = Provider<WorkoutRepository>(
  (ref) => WorkoutRepository(ref.read(workoutApiProvider)),
);
