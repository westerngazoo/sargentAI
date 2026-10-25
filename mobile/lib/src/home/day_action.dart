/// What the selected day offers (R-0061 AC5), as a pure function of the day,
/// today and the day's state. Every index it needs comes from the server, so
/// the client never replays the plan (SPEC-0061 §2.5).
///
/// Slice 1f knows two states, from the logs alone: a day with sessions and a
/// day without. Slice 2d adds the planned and rest states and the indices.
library;

/// A calendar day's state as the client shows it.
sealed class DayState {
  const DayState();
}

/// At least one workout is logged that day.
final class DoneDay extends DayState {
  const DoneDay();
}

/// Nothing is logged that day, and no plan is projected onto it yet.
final class NothingDay extends DayState {
  const NothingDay();
}

/// The one primary action of the selected day.
sealed class DayAction {
  const DayAction();
}

/// Today: start a session. A null index starts a free session.
final class StartSession extends DayAction {
  const StartSession(this.planDayIndex);
  final int? planDayIndex;
}

/// A past day with nothing logged: log it late.
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

/// Strips the time, so two `DateTime`s on the same calendar day compare equal.
DateTime dateOnly(DateTime d) => DateTime(d.year, d.month, d.day);

DayAction primaryAction({
  required DateTime day,
  required DateTime today,
  required DayState state,
}) {
  final cmp = dateOnly(day).compareTo(dateOnly(today));
  return switch ((cmp, state)) {
    (_, DoneDay()) => const ViewLog(),
    (0, NothingDay()) => const StartSession(null),
    (< 0, NothingDay()) => const LogLate(null),
    _ => const NoAction(),
  };
}
