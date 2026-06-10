import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../data/workout_repository.dart';
import '../domain/workout_session.dart';

/// The home shell's recent-sessions list (the `profileProvider` idiom): resolves
/// `GET /workouts` in server order; invalidated after a create (driver) or a
/// delete (list controller).
final workoutsProvider = FutureProvider<List<WorkoutSession>>(
  (ref) => ref.read(workoutRepositoryProvider).list(),
);
