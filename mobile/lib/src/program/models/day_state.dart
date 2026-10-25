/// A calendar day's state, as `GET /programs/me/schedule` projects it
/// (SPEC-0061 §2.2, §2.4). Indices point into the program's cycle and every
/// one of them comes from the server: the client never replays the plan.
///
/// Mirrors `core::plan::DayState`, serialized with `#[serde(tag = "state")]`.
library;

/// One of the four shapes a day can take (R-0061 AC4).
sealed class DayState {
  const DayState();

  /// Parses a `days[]` entry (or just its tagged state fields).
  ///
  /// Throws a [FormatException] on a `state` token the client does not
  /// know; a missing field of a known state throws a [TypeError], as every
  /// wire model here does.
  factory DayState.fromJson(Map<String, dynamic> json) =>
      switch (json['state'] as String) {
        'done' => DoneDay(
            completed: (json['completed'] as List<dynamic>).cast<int>(),
            pending: json['pending'] as int?,
          ),
        'planned' => PlannedDay(json['cycle_index'] as int),
        'rest' => RestDay(json['cycle_index'] as int),
        'nothing' => NothingDay(pending: json['pending'] as int?),
        final other => throw FormatException('unknown day state: $other'),
      };
}

/// At least one workout is logged that day.
final class DoneDay extends DayState {
  const DoneDay({this.completed = const [], this.pending});

  /// The cycle indices completed that day; empty when only free workouts
  /// were logged.
  final List<int> completed;

  /// With free workouts only: the training day still pending (N1), which
  /// today's secondary *Empezar sesión* starts.
  final int? pending;
}

/// A training day of the plan projected onto today or later.
final class PlannedDay extends DayState {
  const PlannedDay(this.cycleIndex);
  final int cycleIndex;
}

/// A rest day of the plan projected onto today or later.
final class RestDay extends DayState {
  const RestDay(this.cycleIndex);
  final int cycleIndex;
}

/// A past day with nothing logged, or any day for a user with no program.
final class NothingDay extends DayState {
  const NothingDay({this.pending});

  /// The training day a late log would complete, if any (null before a
  /// later plan log, N2: a late log there is a free workout).
  final int? pending;
}
