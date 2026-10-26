/// What the selected day offers (R-0061 AC5), as a pure function of the day,
/// today and the day's state. Every index it needs comes from the server, so
/// the client never replays the plan (SPEC-0061 §2.5, §3).
library;

import '../core/dates.dart';
import '../program/models/day_state.dart';

/// The one primary action of the selected day.
sealed class DayAction {
  const DayAction();
}

/// Today: start a session. The cycle index of a planned day, or null for a
/// free session (no program, or a rest day).
final class StartSession extends DayAction {
  const StartSession(this.planDayIndex);
  final int? planDayIndex;
}

/// Today, with only a free workout logged: the primary action shows the
/// log, and a secondary *Empezar sesión* starts the plan day still
/// [pending] (SPEC-0061 §2.2, N1).
final class ViewLogWithPending extends DayAction {
  const ViewLogWithPending(this.pending);
  final int pending;
}

/// A past day with nothing logged: log it late. The training day that log
/// completes, or null for a free late log (N2).
final class LogLate extends DayAction {
  const LogLate(this.planDayIndex);
  final int? planDayIndex;
}

/// A done day: see what was logged.
final class ViewLog extends DayAction {
  const ViewLog();
}

/// A future day: its projection and nothing to do.
final class NoAction extends DayAction {
  const NoAction();
}

/// The primary action of [day] seen from [today], given its [state]. The
/// time of day never matters.
DayAction primaryAction({
  required DateTime day,
  required DateTime today,
  required DayState state,
}) {
  final cmp = dateOnly(day).compareTo(dateOnly(today));
  return switch ((cmp, state)) {
    (0, DoneDay(pending: final p?)) => ViewLogWithPending(p),
    (_, DoneDay()) => const ViewLog(),
    (0, PlannedDay(:final cycleIndex)) => StartSession(cycleIndex),
    (0, _) => const StartSession(null),
    (< 0, NothingDay(:final pending)) => LogLate(pending),
    _ => const NoAction(),
  };
}
