// Weekly plan progress — how many training days the user has completed this
// week against the week's target, plus a percentage and total-session
// count. Pure computation + a provider that combines the workout log, the
// active program and the schedule.

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/dates.dart';
import '../../workout/application/workouts_provider.dart';
import '../../workout/domain/workout_session.dart';
import '../models/day_state.dart';
import '../models/schedule.dart';
import 'program_providers.dart';
import 'schedule_provider.dart';

@immutable
class WeeklyProgress {
  const WeeklyProgress({
    required this.daysDone,
    required this.daysTarget,
    required this.totalSessions,
  });

  /// Distinct days with a logged workout in the current week (Mon–Sun).
  final int daysDone;

  /// The training days the projection places in this week (SPEC-0061 N6).
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

/// Pure: the week's target read from the projection (SPEC-0061 §2.1, N6),
/// not from `days_per_week`. Counts the days of the Monday-to-Sunday week
/// of [now] that the schedule places a training day on: planned, or already
/// done with a plan session. A rest day, a free workout and an empty day do
/// not count.
int weeklyTrainingTarget({required Schedule schedule, required DateTime now}) {
  final today = dateOnly(now);
  final weekStart =
      DateTime(today.year, today.month, today.day - (today.weekday - 1));
  final weekEnd = DateTime(weekStart.year, weekStart.month, weekStart.day + 6);
  bool inWeek(DateTime d) => !d.isBefore(weekStart) && !d.isAfter(weekEnd);
  bool isTraining(DayState s) => switch (s) {
        PlannedDay() => true,
        DoneDay(:final completed) => completed.isNotEmpty,
        RestDay() || NothingDay() => false,
      };
  return schedule.days.entries
      .where((e) => inWeek(e.key) && isTraining(e.value))
      .length;
}

/// Combines the workout log, the active program and this week's schedule
/// into [WeeklyProgress]. Null while any source is still loading or absent.
final weeklyProgressProvider = Provider<WeeklyProgress?>((ref) {
  final workouts = ref.watch(workoutsProvider).valueOrNull;
  final program = ref.watch(currentProgramProvider).valueOrNull;
  final now = DateTime.now();
  final schedule = ref.watch(scheduleProvider(scheduleWindow(now))).valueOrNull;
  if (workouts == null || program == null || schedule == null) return null;
  return computeWeeklyProgress(
    workouts: workouts,
    daysTarget: weeklyTrainingTarget(schedule: schedule, now: now),
    now: now,
    programChosenAt: program.chosenAt,
  );
});
