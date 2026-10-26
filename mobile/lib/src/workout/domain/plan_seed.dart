import 'package:flutter/foundation.dart';

/// What a plan session starts from (SPEC-0061 §2.7): the current program,
/// the cycle [index] of one of its training days, and that day's exercises
/// by name. The caller builds it from the schedule it already holds; the
/// session driver never reads the schedule itself (second review, N12).
@immutable
class PlanSeed {
  const PlanSeed({
    required this.userProgramId,
    required this.index,
    required this.exercises,
  });

  /// The program the day belongs to: `user_program_id` on save.
  final String userProgramId;

  /// The training day's position in the program's cycle: `plan_day_index`
  /// on save. The server supplied it, so it names a training day.
  final int index;

  /// The day's exercises by name, in the plan's order.
  final List<String> exercises;

  @override
  bool operator ==(Object other) =>
      other is PlanSeed &&
      other.userProgramId == userProgramId &&
      other.index == index &&
      listEquals(other.exercises, exercises);

  @override
  int get hashCode =>
      Object.hash(userProgramId, index, Object.hashAll(exercises));
}
