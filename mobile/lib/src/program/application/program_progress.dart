// Weekly plan progress — how many training days the user has completed this
// week against the program's target, plus a percentage and total-session
// count. Pure computation + a provider that combines the workout log with
// the active program.

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../workout/application/workouts_provider.dart';
import '../../workout/domain/workout_session.dart';
import 'program_providers.dart';

@immutable
class WeeklyProgress {
  const WeeklyProgress({
    required this.daysDone,
    required this.daysTarget,
    required this.totalSessions,
  });

  /// Distinct days with a logged workout in the current week (Mon–Sun).
  final int daysDone;

  /// The program's `daysPerWeek` target.
  final int daysTarget;

  /// All-time sessions logged since the program was chosen.
  final int totalSessions;

  /// Completion of this week's target, clamped to 0..1.
  double get ratio {
    if (daysTarget <= 0) return 0;
    return (daysDone / daysTarget).clamp(0.0, 1.0);
  }

  int get percent => (ratio * 100).round();

  /// True once the weekly target is met (or exceeded).
  bool get weekComplete => daysDone >= daysTarget && daysTarget > 0;
}

/// Pure: derive weekly progress from the workout log. [now] is injected so
/// the week boundary is testable.
WeeklyProgress computeWeeklyProgress({
  required List<WorkoutSession> workouts,
  required int daysTarget,
  required DateTime now,
  DateTime? programChosenAt,
}) {
  final today = DateTime(now.year, now.month, now.day);
  final weekStart = today.subtract(Duration(days: now.weekday - 1));
  DateTime dayOf(DateTime d) => DateTime(d.year, d.month, d.day);

  final daysDone = workouts
      .where((w) => !dayOf(w.performedOn).isBefore(weekStart))
      .map((w) => dayOf(w.performedOn))
      .toSet()
      .length;

  final total = programChosenAt == null
      ? workouts.length
      : workouts.where((w) => !w.performedOn.isBefore(programChosenAt)).length;

  return WeeklyProgress(
    daysDone: daysDone,
    daysTarget: daysTarget,
    totalSessions: total,
  );
}

/// Combines the workout log and the active program into [WeeklyProgress].
/// Null while either source is still loading/absent.
final weeklyProgressProvider = Provider<WeeklyProgress?>((ref) {
  final workouts = ref.watch(workoutsProvider).valueOrNull;
  final program = ref.watch(currentProgramProvider).valueOrNull;
  if (workouts == null || program == null) return null;
  return computeWeeklyProgress(
    workouts: workouts,
    daysTarget: program.program.daysPerWeek,
    now: DateTime.now(),
    programChosenAt: program.chosenAt,
  );
});
