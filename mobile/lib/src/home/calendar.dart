import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

import '../../l10n/app_localizations.dart';
import '../core/theme/tokens.dart';
import 'day_action.dart';

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
    this.onToggleMode,
  });

  final CalendarMode mode;

  /// Any day inside the week or month being shown.
  final DateTime anchor;
  final DateTime today;
  final DateTime selected;
  final DayState Function(DateTime day) stateOf;
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
        : _capitalize(DateFormat.yMMMM(locale).format(anchor));
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
    final start = a.subtract(Duration(days: offset));
    return [for (var i = 0; i < 7; i++) start.add(Duration(days: i))];
  }

  /// Whole weeks covering [anchor]'s month.
  static List<DateTime> _monthGrid(DateTime anchor, int firstDay) {
    final first = DateTime(anchor.year, anchor.month, 1);
    final last = DateTime(anchor.year, anchor.month + 1, 0);
    final start = _week(first, firstDay).first;
    final end = _week(last, firstDay).last;
    final n = end.difference(start).inDays + 1;
    return [for (var i = 0; i < n; i++) start.add(Duration(days: i))];
  }

  static String _weekdayInitial(DateTime d, String locale) =>
      DateFormat.E(locale).format(d).characters.first.toUpperCase();

  static String _weekTitle(DateTime a, DateTime b, String locale) {
    final day = DateFormat.d(locale);
    final month = DateFormat.MMM(locale);
    return a.month == b.month
        ? '${day.format(a)}–${day.format(b)} ${_capitalize(month.format(b))}'
        : '${day.format(a)} ${_capitalize(month.format(a))} – '
            '${day.format(b)} ${_capitalize(month.format(b))}';
  }

  static String _capitalize(String s) =>
      s.isEmpty ? s : s[0].toUpperCase() + s.substring(1);
}

enum CalendarMode { week, month }

/// One day: its number, a mark for its state, a ring for today and a fill
/// for the selected day. Colours identify states, never good or bad.
class DayCell extends StatelessWidget {
  const DayCell({
    super.key,
    required this.day,
    required this.state,
    required this.isToday,
    required this.isSelected,
    required this.inMonth,
    required this.onTap,
  });

  final DateTime day;
  final DayState state;
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
    return Semantics(
      button: true,
      selected: isSelected,
      label: DateFormat.yMMMMEEEEd(Localizations.localeOf(context).toString())
          .format(day),
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
              _Mark(state: state, color: fg),
            ],
          ),
        ),
      ),
    );
  }
}

class _Mark extends StatelessWidget {
  const _Mark({required this.state, required this.color});
  final DayState state;
  final Color color;

  @override
  Widget build(BuildContext context) => switch (state) {
        DoneDay() => Container(
            key: const Key('mark-done'),
            width: 7,
            height: 7,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          ),
        NothingDay() => const SizedBox(height: 7),
      };
}
