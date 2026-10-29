// SPEC-0061 §2.4 — the `GET /programs/me/schedule` wire type.

import 'package:flutter/foundation.dart';

import '../../core/dates.dart';
import '../../physics/domain/lift.dart';
import '../../workout/domain/muscle_group.dart';
import '../../workout/domain/plan_seed.dart';
import 'day_state.dart';

/// One entry of the program's day cycle. Mirrors `core::plan::PlanDay`,
/// serialized with `#[serde(tag = "kind")]`.
sealed class PlanDay {
  const PlanDay();

  /// Parses a `cycle[]` entry. Throws a [FormatException] on an unknown
  /// `kind`.
  factory PlanDay.fromJson(Map<String, dynamic> json) =>
      switch (json['kind'] as String) {
        'training' => TrainingPlanDay(
            focus: (json['focus'] as List<dynamic>)
                .map((t) => MuscleGroup.fromWire(t as String))
                .toList(),
            exercises: (json['exercises'] as List<dynamic>).cast<String>(),
            physics: (json['physics'] as List<dynamic>)
                .map((p) => Lift.fromWire(p as String?))
                .toList(),
          ),
        'rest' => const RestPlanDay(),
        final other => throw FormatException('unknown plan day kind: $other'),
      };
}

/// A training day: named by its [focus] on the client (SPEC-0061 §2.1), with
/// its exercises as curated English data (§4).
final class TrainingPlanDay extends PlanDay {
  const TrainingPlanDay({
    required this.focus,
    required this.exercises,
    required this.physics,
  });

  /// The muscle groups the day trains, in the record's order; never empty.
  final List<MuscleGroup> focus;

  /// The day's exercises by name, in the record's order; never empty.
  final List<String> exercises;

  /// Per exercise, in [exercises]' order: the lift whose physics check
  /// models it, or null when it has none yet (*Pronto*). The server decides
  /// (SPEC-0059 §2.5).
  final List<Lift?> physics;
}

/// A rest day of the cycle.
final class RestPlanDay extends PlanDay {
  const RestPlanDay();
}

/// The projection of the current program over a requested range, plus the
/// logged past. [programId], [startsOn] and [cycle] are null or empty when
/// the caller has no usable plan; [days] then come from the logs alone.
@immutable
class Schedule {
  const Schedule({
    required this.programId,
    required this.startsOn,
    required this.cycle,
    required this.days,
    required this.sessionIds,
  });

  final String? programId;
  final DateTime? startsOn;
  final List<PlanDay> cycle;

  /// One state per date in the requested range, keyed by local date.
  final Map<DateTime, DayState> days;

  /// The workout ids logged on each date that has any.
  final Map<DateTime, List<String>> sessionIds;

  /// Parses the `GET /programs/me/schedule` body.
  ///
  /// Throws a [FormatException] on a `kind` or `state` token the client does
  /// not know, like [DayState.fromJson]; a missing field throws a
  /// [TypeError], and an unknown muscle-group token a [StateError], as every
  /// wire model here does.
  factory Schedule.fromJson(Map<String, dynamic> json) {
    final startsOn = json['starts_on'] as String?;
    return Schedule(
      programId: json['program_id'] as String?,
      startsOn: startsOn == null ? null : DateTime.parse(startsOn),
      cycle: (json['cycle'] as List<dynamic>)
          .map((e) => PlanDay.fromJson(e as Map<String, dynamic>))
          .toList(),
      days: {
        for (final e in json['days'] as List<dynamic>)
          DateTime.parse((e as Map<String, dynamic>)['date'] as String):
              DayState.fromJson(e),
      },
      sessionIds: {
        for (final e in (json['session_ids'] as Map<String, dynamic>).entries)
          DateTime.parse(e.key): (e.value as List<dynamic>).cast<String>(),
      },
    );
  }

  /// The state of [day], or null when it lies outside the requested range.
  DayState? stateOn(DateTime day) => days[dateOnly(day)];

  /// The training entry at [index], or null when the index is out of the
  /// cycle or names a rest day.
  TrainingPlanDay? trainingDay(int index) {
    if (index < 0 || index >= cycle.length) return null;
    final entry = cycle[index];
    return entry is TrainingPlanDay ? entry : null;
  }

  /// The seed of a plan session for the training day at [index]
  /// (SPEC-0061 §2.7), or null when there is no program or [index] names no
  /// training day: the session is then free.
  PlanSeed? planSeed(int index) {
    final programId = this.programId;
    final day = trainingDay(index);
    if (programId == null || day == null) return null;
    return PlanSeed(
      userProgramId: programId,
      index: index,
      exercises: day.exercises,
    );
  }
}
