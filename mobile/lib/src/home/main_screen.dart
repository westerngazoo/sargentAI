import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import '../../l10n/app_localizations.dart';
import '../auth/application/auth_controller.dart';
import '../auth/presentation/brand_header.dart';
import '../core/brand.dart';
import '../core/panels/show_panel.dart';
import '../core/theme/halftone.dart';
import '../core/theme/tokens.dart';
import '../measurements/presentation/log_measurement_sheet.dart';
import '../nutrition/presentation/meal_quick_log_sheet.dart';
import '../profile/application/profile_providers.dart';
import '../profile/presentation/profile_prompt.dart';
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
class MainScreen extends ConsumerStatefulWidget {
  const MainScreen({super.key});

  @override
  ConsumerState<MainScreen> createState() => _MainScreenState();
}

class _MainScreenState extends ConsumerState<MainScreen> {
  CalendarMode _phoneMode = CalendarMode.week;
  late DateTime _anchor = ref.read(selectedDayProvider);

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final today = dateOnly(DateTime.now());
    final selected = ref.watch(selectedDayProvider);
    final profile = ref.watch(profileProvider);
    final dismissed = ref.watch(onboardingDismissedProvider);
    final sessions = ref.watch(workoutsProvider).valueOrNull ?? const [];
    final wide = MediaQuery.sizeOf(context).width >= panelBreakpoint;
    final mode = wide ? CalendarMode.month : _phoneMode;

    final doneDays = {for (final s in sessions) dateOnly(s.performedOn)};
    DayState stateOf(DateTime d) =>
        doneDays.contains(dateOnly(d)) ? const DoneDay() : const NothingDay();

    final calendar = CalendarView(
      mode: mode,
      anchor: _anchor,
      today: today,
      selected: selected,
      stateOf: stateOf,
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
      loading: ref.watch(workoutsProvider).isLoading,
      prompt: prompt,
      onStart: (performedOn) {
        ref
            .read(sessionDriverProvider.notifier)
            .start(performedOn: performedOn == today ? null : performedOn);
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
/// profile, one primary action, and what was logged (R-0061 AC5).
class _DayRegion extends StatelessWidget {
  const _DayRegion({
    required this.day,
    required this.today,
    required this.state,
    required this.loading,
    required this.prompt,
    required this.onStart,
  });

  final DateTime day;
  final DateTime today;
  final DayState state;
  final bool loading;
  final Widget prompt;
  final ValueChanged<DateTime> onStart;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final locale = Localizations.localeOf(context).toString();
    final text = Theme.of(context).textTheme;
    final t = Theme.of(context).extension<FbfTokens>() ?? FbfTokens.light;
    final action = primaryAction(day: day, today: today, state: state);
    final isToday = dateOnly(day) == today;
    final title = capitalizeFirst(DateFormat.MMMMEEEEd(locale).format(day));

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
            NothingDay() => l10n.dayFreeHeadline,
          },
          style: text.headlineMedium,
        ),
        prompt,
        const SizedBox(height: 12),
        switch (action) {
          StartSession() => FilledButton.icon(
              key: const Key('primary-action'),
              icon: const Icon(Icons.play_arrow),
              label: Text(l10n.startSession),
              onPressed: () => onStart(day),
            ),
          LogLate() => FilledButton.icon(
              key: const Key('primary-action'),
              icon: const Icon(Icons.history),
              label: Text(l10n.logLate),
              onPressed: () => onStart(day),
            ),
          ViewLog() => OutlinedButton.icon(
              key: const Key('primary-action'),
              icon: const Icon(Icons.list_alt),
              label: Text(l10n.viewLogged),
              onPressed: () => showPanel<void>(
                context,
                title: title,
                body: (_) => SessionList(day: day),
              ),
            ),
          NoAction() => const SizedBox.shrink(),
        },
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
