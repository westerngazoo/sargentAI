import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import '../../l10n/app_localizations.dart';
import '../auth/application/auth_controller.dart';
import '../auth/presentation/brand_header.dart';
import '../core/brand.dart';
import '../core/dates.dart';
import '../core/panels/show_panel.dart';
import '../core/theme/halftone.dart';
import '../core/theme/tokens.dart';
import '../measurements/presentation/log_measurement_sheet.dart';
import '../nutrition/presentation/meal_quick_log_sheet.dart';
import '../profile/application/profile_providers.dart';
import '../profile/presentation/profile_prompt.dart';
import '../program/application/schedule_provider.dart';
import '../program/models/day_state.dart';
import '../program/models/schedule.dart';
import '../workout/application/muscle_names.dart';
import '../workout/application/save_notice_text.dart';
import '../workout/application/session_driver.dart';
import '../workout/application/workouts_provider.dart';
import '../workout/presentation/session_list.dart';
import 'calendar.dart';
import 'day_action.dart';
import 'selected_day.dart';

/// The one screen (R-0061 AC1, SPEC-0061 §2.5): the calendar, the selected
/// day with its one action, and the few controls AC7 allows. Everything
/// else opens over it with [showPanel] and closes back to it.
///
/// `profileProvider`'s read is the cold-start liveness probe: a restored but
/// revoked token meets a 401, which the `AuthInterceptor` turns into a
/// logout (SPEC-0007 SAC5). It moved here from the old `HomeShell`.
///
/// The day states come from `scheduleProvider` for the month shown. While
/// that loads, or when it fails, the logs alone draw the calendar (done or
/// nothing), so the screen never blanks.
class MainScreen extends ConsumerStatefulWidget {
  const MainScreen({super.key});

  @override
  ConsumerState<MainScreen> createState() => _MainScreenState();
}

class _MainScreenState extends ConsumerState<MainScreen>
    with WidgetsBindingObserver {
  CalendarMode _phoneMode = CalendarMode.week;
  late DateTime _anchor = ref.read(selectedDayProvider);

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  /// Coming back to the app re-reads the projection: a day may have passed,
  /// or another device may have logged (SPEC-0061 §2.5).
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) ref.invalidate(scheduleProvider);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final today = dateOnly(DateTime.now());
    // A finished session lands on its day (SPEC-0061 §2.7); the schedule
    // re-reads on its own, since it watches the workout list. Saved as free
    // after a refused plan link, it says so.
    ref.listen<SessionDriverState>(sessionDriverProvider, (previous, next) {
      if (!next.done || (previous?.done ?? false)) return;
      ref.read(selectedDayProvider.notifier).state = next.performedOn ?? today;
      final notice = next.notice;
      if (notice == null) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(saveNoticeText(notice, l10n))),
      );
    });
    final selected = ref.watch(selectedDayProvider);
    final profile = ref.watch(profileProvider);
    final dismissed = ref.watch(onboardingDismissedProvider);
    final workouts = ref.watch(workoutsProvider);
    final sessions = workouts.valueOrNull ?? const [];
    final scheduleAsync = ref.watch(scheduleProvider(scheduleWindow(_anchor)));
    final schedule = scheduleAsync.valueOrNull;
    final wide = MediaQuery.sizeOf(context).width >= panelBreakpoint;
    final mode = wide ? CalendarMode.month : _phoneMode;

    final doneDays = {for (final s in sessions) dateOnly(s.performedOn)};
    DayState stateOf(DateTime d) =>
        schedule?.stateOn(d) ??
        (doneDays.contains(dateOnly(d)) ? const DoneDay() : const NothingDay());
    String? labelOf(DateTime d) => switch (stateOf(d)) {
          PlannedDay(:final cycleIndex) => switch (
                schedule?.trainingDay(cycleIndex)) {
              null => null,
              final planned => focusName(planned.focus, l10n),
            },
          _ => null,
        };

    final calendar = CalendarView(
      mode: mode,
      anchor: _anchor,
      today: today,
      selected: selected,
      stateOf: stateOf,
      labelOf: labelOf,
      onSelect: (d) => ref.read(selectedDayProvider.notifier).state = d,
      onShift: (n) => setState(() {
        _anchor = mode == CalendarMode.week
            ? addDays(_anchor, 7 * n)
            : DateTime(_anchor.year, _anchor.month + n, 1);
      }),
      onToggleMode: wide
          ? null
          : () => setState(() {
                _phoneMode = _phoneMode == CalendarMode.week
                    ? CalendarMode.month
                    : CalendarMode.week;
                // Back to the week strip: show the selected day's week, not
                // the month's first.
                if (_phoneMode == CalendarMode.week) _anchor = selected;
              }),
    );

    final prompt = profile.when(
      loading: () => const SizedBox.shrink(),
      error: (_, __) => const SizedBox.shrink(),
      data: (p) => (p == null && !dismissed)
          ? ProfilePrompt(
              onStart: () => context.push('/onboarding'),
              onDismiss: () =>
                  ref.read(onboardingDismissedProvider.notifier).state = true,
            )
          : const SizedBox.shrink(),
    );

    final day = _DayRegion(
      day: selected,
      today: today,
      state: stateOf(selected),
      schedule: schedule,
      loading: workouts.isLoading || scheduleAsync.isLoading,
      prompt: prompt,
      onStart: (day, planDayIndex) {
        ref.read(sessionDriverProvider.notifier).start(
              seed: planDayIndex == null
                  ? null
                  : schedule?.planSeed(planDayIndex),
              performedOn: day == today ? null : day,
            );
        context.push('/session');
      },
    );

    final nutrition = _NutritionCard(
      onAddMeal: () => showMealQuickLogSheet(context),
    );

    return Scaffold(
      appBar: AppBar(
        titleSpacing: 12,
        title: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            const BrandBadge(size: 30, shadow: false),
            const SizedBox(width: 10),
            Flexible(
              child: Text(
                Brand.appName,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.titleLarge?.copyWith(
                      fontFamily: AppFonts.headline,
                      letterSpacing: 0.5,
                    ),
              ),
            ),
          ],
        ),
        actions: [
          PopupMenuButton<_Menu>(
            key: const Key('profile-menu'),
            tooltip: l10n.profileMenu,
            icon: const Icon(Icons.account_circle_outlined),
            onSelected: _open,
            itemBuilder: (_) => [
              PopupMenuItem(
                  value: _Menu.program, child: Text(l10n.menuProgram)),
              PopupMenuItem(
                  value: _Menu.measurements,
                  child: Text(l10n.menuMeasurements)),
              PopupMenuItem(
                  value: _Menu.progress, child: Text(l10n.menuProgress)),
              const PopupMenuDivider(),
              PopupMenuItem(value: _Menu.signOut, child: Text(l10n.signOut)),
            ],
          ),
          const SizedBox(width: 4),
        ],
      ),
      floatingActionButton: FloatingActionButton(
        key: const Key('mic'),
        tooltip: l10n.dictate,
        onPressed: () => context.push('/hub'),
        child: const Icon(Icons.mic),
      ),
      body: HalftoneBackground(
        child: SafeArea(
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 1200),
              child: wide
                  ? SingleChildScrollView(
                      padding: const EdgeInsets.all(16),
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          SizedBox(
                            // §2.5: a 316 px rail, 340 from 1180.
                            width: MediaQuery.sizeOf(context).width >= 1180
                                ? 340
                                : 316,
                            child: Column(
                              children: [
                                _Panel(child: calendar),
                                const SizedBox(height: 16),
                                _Panel(child: nutrition),
                              ],
                            ),
                          ),
                          const SizedBox(width: 16),
                          Expanded(child: _Panel(child: day)),
                        ],
                      ),
                    )
                  : ListView(
                      padding: const EdgeInsets.fromLTRB(16, 12, 16, 96),
                      children: [
                        _Panel(child: calendar),
                        const SizedBox(height: 12),
                        _Panel(child: day),
                        const SizedBox(height: 12),
                        _Panel(child: nutrition),
                      ],
                    ),
            ),
          ),
        ),
      ),
    );
  }

  /// Until slice 3 gives each its panel, Programa and Progreso push the
  /// existing screens, so back returns here and nothing is lost (AC2).
  void _open(_Menu item) {
    switch (item) {
      case _Menu.program:
        context.push('/programs/current');
      case _Menu.measurements:
        showLogMeasurementSheet(context);
      case _Menu.progress:
        context.push('/progress');
      case _Menu.signOut:
        ref.read(authControllerProvider.notifier).logout();
    }
  }
}

enum _Menu { program, measurements, progress, signOut }

/// A card in the reels' grammar: hard border, hard shadow, no radius to
/// speak of.
class _Panel extends StatelessWidget {
  const _Panel({required this.child});
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).extension<FbfTokens>() ?? FbfTokens.light;
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: t.panel,
        border: Border.all(color: t.line, width: 2),
        boxShadow: [BoxShadow(color: t.line, offset: const Offset(4, 4))],
      ),
      child: child,
    );
  }
}

/// The selected day: its headline, the onboarding prompt when there is no
/// profile, one primary action, and what was logged or what the plan puts
/// there (R-0061 AC5). A planned day is named by its focus and lists its
/// exercises; a free workout on a plan day keeps that day startable below
/// the log (SPEC-0061 §2.2, N1).
class _DayRegion extends StatelessWidget {
  const _DayRegion({
    required this.day,
    required this.today,
    required this.state,
    required this.schedule,
    required this.loading,
    required this.prompt,
    required this.onStart,
  });

  final DateTime day;
  final DateTime today;
  final DayState state;

  /// Resolves the state's cycle indices to their training days; null while
  /// the schedule loads or failed.
  final Schedule? schedule;
  final bool loading;
  final Widget prompt;

  /// Starts a session on the day, for the plan day at the index when the
  /// server named one (SPEC-0061 §2.7).
  final void Function(DateTime day, int? planDayIndex) onStart;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final locale = Localizations.localeOf(context).toString();
    final text = Theme.of(context).textTheme;
    final t = Theme.of(context).extension<FbfTokens>() ?? FbfTokens.light;
    final action = primaryAction(day: day, today: today, state: state);
    final isToday = dateOnly(day) == today;
    final title = capitalizeFirst(DateFormat.MMMMEEEEd(locale).format(day));
    final planned = switch (state) {
      PlannedDay(:final cycleIndex) => schedule?.trainingDay(cycleIndex),
      _ => null,
    };
    final pending = switch (action) {
      ViewLogWithPending(:final pending) => schedule?.trainingDay(pending),
      _ => null,
    };
    void openLog() => showPanel<void>(
          context,
          title: title,
          body: (_) => SessionList(day: day),
        );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                title,
                key: const Key('day-title'),
                style: text.labelMedium?.copyWith(color: t.muted),
              ),
            ),
            if (isToday)
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                color: t.red,
                child: Text(l10n.today,
                    style: text.labelSmall?.copyWith(color: t.onRed)),
              ),
          ],
        ),
        const SizedBox(height: 6),
        Text(
          switch (state) {
            DoneDay() => l10n.dayDoneHeadline,
            PlannedDay() => planned == null
                ? l10n.dayFreeHeadline
                : focusName(planned.focus, l10n),
            RestDay() => l10n.dayRestHeadline,
            NothingDay() => l10n.dayFreeHeadline,
          },
          style: text.headlineMedium,
        ),
        prompt,
        const SizedBox(height: 12),
        switch (action) {
          StartSession(:final planDayIndex) => FilledButton.icon(
              key: const Key('primary-action'),
              icon: const Icon(Icons.play_arrow),
              label: Text(l10n.startSession),
              onPressed: () => onStart(day, planDayIndex),
            ),
          LogLate(:final planDayIndex) => FilledButton.icon(
              key: const Key('primary-action'),
              icon: const Icon(Icons.history),
              label: Text(l10n.logLate),
              onPressed: () => onStart(day, planDayIndex),
            ),
          ViewLog() || ViewLogWithPending() => OutlinedButton.icon(
              key: const Key('primary-action'),
              icon: const Icon(Icons.list_alt),
              label: Text(l10n.viewLogged),
              onPressed: openLog,
            ),
          NoAction() => const SizedBox.shrink(),
        },
        if (action case ViewLogWithPending(pending: final pendingIndex)) ...[
          const SizedBox(height: 12),
          if (pending != null)
            Text(l10n.pendingPlanDay(focusName(pending.focus, l10n)),
                style: text.bodyMedium?.copyWith(color: t.muted)),
          const SizedBox(height: 6),
          FilledButton.icon(
            key: const Key('secondary-action'),
            icon: const Icon(Icons.play_arrow),
            label: Text(l10n.startSession),
            onPressed: () => onStart(day, pendingIndex),
          ),
        ],
        if (planned != null) ...[
          const SizedBox(height: 12),
          for (final exercise in planned.exercises)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Text(exercise, style: text.bodyLarge),
            ),
        ],
        if (loading) ...[
          const SizedBox(height: 12),
          const Center(
            child: SizedBox(
              height: 18,
              width: 18,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
          ),
        ] else if (state is NothingDay && action is! StartSession) ...[
          const SizedBox(height: 12),
          Text(l10n.nothingLogged,
              style: text.bodyMedium?.copyWith(color: t.muted)),
        ],
      ],
    );
  }
}

/// Nutrition orbits the training: one action, *+ Comida* (R-0061 AC7).
/// The protein ring arrives with the Comida panel in slice 3.
class _NutritionCard extends StatelessWidget {
  const _NutritionCard({required this.onAddMeal});
  final VoidCallback onAddMeal;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Row(
      children: [
        Expanded(
          child: Text(l10n.nutritionToday,
              style: Theme.of(context).textTheme.labelMedium),
        ),
        OutlinedButton(
          key: const Key('add-meal'),
          // The theme's buttons stretch (Size.fromHeight); in a row they
          // must not.
          style: OutlinedButton.styleFrom(minimumSize: const Size(0, 44)),
          onPressed: onAddMeal,
          child: Text(l10n.addMeal),
        ),
      ],
    );
  }
}
