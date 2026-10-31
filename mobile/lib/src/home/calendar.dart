import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

import '../../l10n/app_localizations.dart';
import '../core/dates.dart';
import '../core/theme/tokens.dart';
import '../program/models/day_state.dart';

/// The in-house calendar (R-0061 AC4, SPEC-0061 §2.5): a 7-column grid of
/// [DayCell]s showing one state per day, with today marked and the selected
/// day filled. A week strip on phones, the month on wide windows; the parent
/// decides which (`mode`).
class CalendarView extends StatelessWidget {
  const CalendarView({
    super.key,
    required this.mode,
    required this.anchor,
    required this.today,
    required this.selected,
    required this.stateOf,
    required this.onSelect,
    required this.onShift,
    this.labelOf,
    this.onToggleMode,
  });

  final CalendarMode mode;

  /// Any day inside the week or month being shown.
  final DateTime anchor;
  final DateTime today;
  final DateTime selected;
  final DayState Function(DateTime day) stateOf;

  /// The name of a planned day (its focus, R-0061 AC4), or null when the day
  /// has none; the cell carries it as a tooltip and in its semantics.
  final String? Function(DateTime day)? labelOf;
  final ValueChanged<DateTime> onSelect;

  /// Move one week or one month: `-1` or `+1`.
  final ValueChanged<int> onShift;

  /// Phones only: swap between the week strip and the month.
  final VoidCallback? onToggleMode;

  @override
  Widget build(BuildContext context) {
    final locale = Localizations.localeOf(context).toString();
    final firstDay = _firstDayOfWeek(context);
    final days = mode == CalendarMode.week
        ? _week(anchor, firstDay)
        : _monthGrid(anchor, firstDay);
    final title = mode == CalendarMode.week
        ? _weekTitle(days.first, days.last, locale)
        : capitalizeFirst(DateFormat.yMMMM(locale).format(anchor));
    final materialL10n = MaterialLocalizations.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(title,
                  key: const Key('calendar-title'),
                  style: Theme.of(context).textTheme.titleLarge),
            ),
            IconButton(
              key: const Key('calendar-prev'),
              icon: const Icon(Icons.chevron_left),
              tooltip: materialL10n.previousMonthTooltip,
              onPressed: () => onShift(-1),
            ),
            IconButton(
              key: const Key('calendar-next'),
              icon: const Icon(Icons.chevron_right),
              tooltip: materialL10n.nextMonthTooltip,
              onPressed: () => onShift(1),
            ),
            if (onToggleMode != null)
              IconButton(
                key: const Key('calendar-toggle'),
                icon: Icon(mode == CalendarMode.week
                    ? Icons.calendar_view_month
                    : Icons.view_week),
                tooltip: mode == CalendarMode.week
                    ? AppLocalizations.of(context).showMonth
                    : AppLocalizations.of(context).showWeek,
                onPressed: onToggleMode,
              ),
          ],
        ),
        const SizedBox(height: 6),
        GridView.count(
          crossAxisCount: 7,
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          childAspectRatio: 1.15,
          children: [
            for (var i = 0; i < 7; i++)
              Center(
                child: Text(
                  _weekdayInitial(days[i], locale),
                  style: Theme.of(context).textTheme.labelSmall,
                ),
              ),
            for (final day in days)
              DayCell(
                key: ValueKey('day-${dateOnly(day).toIso8601String()}'),
                day: day,
                state: stateOf(day),
                label: labelOf?.call(day),
                isToday: dateOnly(day) == dateOnly(today),
                isSelected: dateOnly(day) == dateOnly(selected),
                inMonth: mode == CalendarMode.week || day.month == anchor.month,
                onTap: () => onSelect(dateOnly(day)),
              ),
          ],
        ),
      ],
    );
  }

  static int _firstDayOfWeek(BuildContext context) =>
      MaterialLocalizations.of(context).firstDayOfWeekIndex; // 0 = Sunday

  /// The seven days of [anchor]'s week, starting on the locale's first day.
  static List<DateTime> _week(DateTime anchor, int firstDay) {
    final a = dateOnly(anchor);
    final offset = (a.weekday % 7 - firstDay + 7) % 7;
    final start = addDays(a, -offset);
    return [for (var i = 0; i < 7; i++) addDays(start, i)];
  }

  /// Whole weeks covering [anchor]'s month.
  static List<DateTime> _monthGrid(DateTime anchor, int firstDay) {
    final first = DateTime(anchor.year, anchor.month, 1);
    final last = DateTime(anchor.year, anchor.month + 1, 0);
    final start = _week(first, firstDay).first;
    final end = _week(last, firstDay).last;
    final n = _daysBetween(start, end) + 1;
    return [for (var i = 0; i < n; i++) addDays(start, i)];
  }

  /// Calendar days from [a] to [b], counted on dates so a DST change (a 23-
  /// or 25-hour day) cannot shift the count.
  static int _daysBetween(DateTime a, DateTime b) =>
      DateTime.utc(b.year, b.month, b.day)
          .difference(DateTime.utc(a.year, a.month, a.day))
          .inDays;

  static String _weekdayInitial(DateTime d, String locale) =>
      DateFormat.E(locale).format(d).characters.first.toUpperCase();

  static String _weekTitle(DateTime a, DateTime b, String locale) {
    final day = DateFormat.d(locale);
    final month = DateFormat.MMM(locale);
    return a.month == b.month
        ? '${day.format(a)}–${day.format(b)} ${capitalizeFirst(month.format(b))}'
        : '${day.format(a)} ${capitalizeFirst(month.format(a))} – '
            '${day.format(b)} ${capitalizeFirst(month.format(b))}';
  }
}

/// [date] plus [n] calendar days, at midnight local time. Built from the
/// date parts, never with `Duration`, so a DST change cannot land on 23:00
/// of the same day and duplicate or skip a cell.
DateTime addDays(DateTime date, int n) =>
    DateTime(date.year, date.month, date.day + n);

/// Spanish month and weekday names come lowercase from `intl`; titles want
/// a capital.
String capitalizeFirst(String s) =>
    s.isEmpty ? s : s[0].toUpperCase() + s.substring(1);

enum CalendarMode { week, month }

/// One day: its number, a mark for its state, a ring for today and a fill
/// for the selected day. Colours identify states, never good or bad.
class DayCell extends StatelessWidget {
  const DayCell({
    super.key,
    required this.day,
    required this.state,
    this.label,
    required this.isToday,
    required this.isSelected,
    required this.inMonth,
    required this.onTap,
  });

  final DateTime day;
  final DayState state;

  /// The day's name, when the plan gives it one: "Piernas". Shown as a
  /// tooltip on the mark and read with the date, never drawn in the cell,
  /// which has no room for it on a phone.
  final String? label;
  final bool isToday;
  final bool isSelected;
  final bool inMonth;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).extension<FbfTokens>() ?? FbfTokens.light;
    final fg = isSelected
        ? t.onRed
        : inMonth
            ? t.text
            : t.muted;
    final date =
        DateFormat.yMMMMEEEEd(Localizations.localeOf(context).toString())
            .format(day);
    return Semantics(
      button: true,
      selected: isSelected,
      label: label == null ? date : '$date, $label',
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(6),
        child: Container(
          margin: const EdgeInsets.all(2),
          decoration: BoxDecoration(
            color: isSelected ? t.red : null,
            borderRadius: BorderRadius.circular(6),
            border: isToday && !isSelected
                ? Border.all(color: t.line, width: 2)
                : null,
          ),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Text(
                day.day.toString(),
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                      color: fg,
                      fontWeight: isToday ? FontWeight.w600 : null,
                    ),
              ),
              const SizedBox(height: 3),
              _Mark(state: state, color: fg, label: label),
            ],
          ),
        ),
      ),
    );
  }
}

/// The glyph of a state (SPEC-0061 §2.5): ● done, ○ planned with its name
/// as a tooltip, — rest, and nothing for an empty day. Every mark is 7 px
/// tall so the cells line up.
class _Mark extends StatelessWidget {
  const _Mark({required this.state, required this.color, this.label});
  final DayState state;
  final Color color;
  final String? label;

  @override
  Widget build(BuildContext context) => switch (state) {
        DoneDay() => Container(
            key: const Key('mark-done'),
            width: 7,
            height: 7,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          ),
        PlannedDay() => _withTooltip(
            Container(
              key: const Key('mark-planned'),
              width: 7,
              height: 7,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                border: Border.all(color: color, width: 1.5),
              ),
            ),
          ),
        RestDay() => SizedBox(
            height: 7,
            child: Center(
              child: Container(
                key: const Key('mark-rest'),
                width: 8,
                height: 2,
                color: color,
              ),
            ),
          ),
        NothingDay() => const SizedBox(height: 7),
      };

  /// The cell's semantics already read the name, so the tooltip adds none.
  Widget _withTooltip(Widget mark) => switch (label) {
        null => mark,
        final l => Tooltip(message: l, excludeFromSemantics: true, child: mark),
      };
}
