import 'package:flutter/foundation.dart';

import 'muscle_group.dart';

/// A stored set, parsed from the R-0004 `WorkoutSet` serialization (transparent
/// newtypes → plain scalars on the wire). An integer `weight_kg`/`rpe` arrives
/// as a Dart `int`, so parse via `num` (the `Profile.fromJson` idiom).
@immutable
class WorkoutSet {
  const WorkoutSet({
    required this.id,
    required this.position,
    required this.reps,
    this.weightKg,
    this.rpe,
  });

  final String id;
  final int position;
  final int reps;
  final double? weightKg;
  final double? rpe;

  factory WorkoutSet.fromJson(Map<String, dynamic> json) => WorkoutSet(
        id: json['id'] as String,
        position: json['position'] as int,
        reps: json['reps'] as int,
        weightKg: (json['weight_kg'] as num?)?.toDouble(),
        rpe: (json['rpe'] as num?)?.toDouble(),
      );
}

@immutable
class WorkoutExercise {
  const WorkoutExercise({
    required this.id,
    required this.position,
    required this.name,
    this.muscleGroup,
    required this.sets,
  });

  final String id;
  final int position;
  final String name;
  final MuscleGroup? muscleGroup;
  final List<WorkoutSet> sets;

  factory WorkoutExercise.fromJson(Map<String, dynamic> json) {
    final group = json['muscle_group'] as String?;
    return WorkoutExercise(
      id: json['id'] as String,
      position: json['position'] as int,
      name: json['name'] as String,
      muscleGroup: group == null ? null : MuscleGroup.fromWire(group),
      sets: (json['sets'] as List)
          .map((e) => WorkoutSet.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
  }
}

/// A stored workout session, parsed from `GET`/`POST /workouts`.
@immutable
class WorkoutSession {
  const WorkoutSession({
    required this.id,
    required this.userId,
    required this.performedOn,
    required this.exercises,
    required this.createdAt,
    required this.updatedAt,
  });

  final String id;
  final String userId;
  final DateTime performedOn;
  final List<WorkoutExercise> exercises;
  final DateTime createdAt;
  final DateTime updatedAt;

  int get setCount => exercises.fold(0, (total, e) => total + e.sets.length);

  factory WorkoutSession.fromJson(Map<String, dynamic> json) => WorkoutSession(
        id: json['id'] as String,
        userId: json['user_id'] as String,
        performedOn: DateTime.parse(json['performed_on'] as String),
        exercises: (json['exercises'] as List)
            .map((e) => WorkoutExercise.fromJson(e as Map<String, dynamic>))
            .toList(),
        createdAt: DateTime.parse(json['created_at'] as String),
        updatedAt: DateTime.parse(json['updated_at'] as String),
      );
}
