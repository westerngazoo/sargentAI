import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../auth/application/auth_controller.dart';
import '../data/workout_repository.dart';
import '../domain/workout_session.dart';

/// The home shell's recent-sessions list (the `profileProvider` idiom): resolves
/// `GET /workouts` in server order; invalidated after a create (driver) or a
/// delete (list controller).
final workoutsProvider = FutureProvider<List<WorkoutSession>>((ref) {
  ref.watch(authUserIdProvider); // account switch drops the cache
  return ref.read(workoutRepositoryProvider).list();
});
