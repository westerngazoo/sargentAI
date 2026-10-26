// SPEC-0061 §2.5 — the schedule the calendar and the Progreso target read.

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../auth/application/auth_controller.dart';
import '../../core/dates.dart';
import '../../workout/application/workouts_provider.dart';
import '../models/schedule.dart';
import '../services/program_service.dart';
import 'program_providers.dart';

/// An inclusive range of local dates: the key of [scheduleProvider]. A value
/// type, so two ranges with the same dates share one cache entry.
@immutable
class DateRange {
  const DateRange(this.from, this.to);

  /// The first day of the range, inclusive.
  final DateTime from;

  /// The last day of the range, inclusive.
  final DateTime to;

  @override
  bool operator ==(Object other) =>
      other is DateRange && other.from == from && other.to == to;

  @override
  int get hashCode => Object.hash(from, to);
}

/// The window to request for the calendar around [anchor]: its month plus
/// six days either side, which covers any six-week month grid and every
/// week strip inside that month. At most 43 days, under the endpoint's 62,
/// and the same key for every day of the month, so moving between weeks
/// costs no request.
DateRange scheduleWindow(DateTime anchor) => DateRange(
      DateTime(anchor.year, anchor.month, 1 - 6),
      DateTime(anchor.year, anchor.month + 1, 0 + 6),
    );

/// `GET /programs/me/schedule` over a [DateRange], with `today` as the local
/// date at fetch time.
///
/// It re-reads when the projection may have moved: when the workout list
/// changes (a saved or deleted session) and when the current program
/// changes, both watched here so the workout and program features never
/// reach into this one. App resume invalidates it from `MainScreen`
/// (SPEC-0061 §2.5), and an account switch drops the cache like every
/// user-scoped provider.
final scheduleProvider =
    FutureProvider.family<Schedule, DateRange>((ref, range) {
  ref.watch(authUserIdProvider);
  ref.watch(workoutsProvider.select((w) => w.valueOrNull));
  ref.watch(currentProgramProvider.select((p) => p.valueOrNull?.id));
  return ref.watch(programServiceProvider).getSchedule(
        from: range.from,
        to: range.to,
        today: dateOnly(DateTime.now()),
      );
});
