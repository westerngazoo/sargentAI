// The Progress screen — body-composition and strength trends over time.
// "Fat down, muscle up, strength up" made visible.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../workout/application/workouts_provider.dart';
import '../../workout/domain/muscle_group.dart';
import '../../workout/domain/workout_session.dart';
import '../application/strength_trend.dart';
import '../models/measurement.dart';
import '../services/measurement_service.dart';
import 'log_measurement_sheet.dart';
import 'trend_chart.dart';

class ProgressScreen extends ConsumerWidget {
  const ProgressScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final measurements = ref.watch(measurementsProvider);
    final workouts = ref.watch(workoutsProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Progress'),
        leading: BackButton(onPressed: () => context.go('/home')),
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => showLogMeasurementSheet(context),
        icon: const Icon(Icons.straighten),
        label: const Text('Log measurement'),
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 96),
        children: [
          Text('Body composition',
              style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          measurements.when(
            loading: () =>
                const Card(child: SizedBox(height: 180, child: _Loading())),
            error: (_, __) => const _ErrorCard(),
            data: (list) => _BodyComposition(measurements: list),
          ),
          const SizedBox(height: 24),
          Text('Strength', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          workouts.when(
            loading: () =>
                const Card(child: SizedBox(height: 180, child: _Loading())),
            error: (_, __) => const _ErrorCard(),
            data: (list) => _StrengthCard(workouts: list),
          ),
          workouts.maybeWhen(
            data: (list) {
              final balance = computeMuscleVolume(list);
              if (balance.isEmpty) return const SizedBox.shrink();
              return Padding(
                padding: const EdgeInsets.only(top: 24),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Training balance',
                        style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 8),
                    _MuscleBalance(balance: balance),
                  ],
                ),
              );
            },
            orElse: () => const SizedBox.shrink(),
          ),
        ],
      ),
    );
  }
}

class _Loading extends StatelessWidget {
  const _Loading();
  @override
  Widget build(BuildContext context) =>
      const Center(child: CircularProgressIndicator());
}

class _ErrorCard extends StatelessWidget {
  const _ErrorCard();
  @override
  Widget build(BuildContext context) => const Card(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Text('Could not load — pull to retry.'),
        ),
      );
}

class _BodyComposition extends StatefulWidget {
  const _BodyComposition({required this.measurements});
  final List<Measurement> measurements;

  @override
  State<_BodyComposition> createState() => _BodyCompositionState();
}

enum _Metric { fat, lean, weight }

class _BodyCompositionState extends State<_BodyComposition> {
  _Metric _selected = _Metric.fat;

  @override
  Widget build(BuildContext context) {
    final m = widget.measurements;
    if (m.length < 2) {
      return Card(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            children: [
              Icon(Icons.show_chart,
                  size: 48,
                  color: Theme.of(context).colorScheme.outlineVariant),
              const SizedBox(height: 10),
              const Text(
                'Log a couple of measurements to see your trend.',
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      );
    }

    final cs = Theme.of(context).colorScheme;
    // Good direction: fat down, lean up, weight is neutral (show as info).
    final fat = m.map((e) => e.bodyFatPercentage).toList();
    final lean = m.map((e) => e.leanMassKg).toList();
    final weight = m.map((e) => e.weightKg).toList();
    final hasFat = fat.every((v) => v != null);
    final hasLean = lean.every((v) => v != null);

    final colorFat = const Color(0xFFE0704A); // warm — fat
    final colorLean = cs.primary; // olive — lean/muscle
    final colorWeight = cs.tertiary;

    final selectedSeries = switch (_selected) {
      _Metric.fat => ChartSeries(
          label: 'Body fat %',
          color: colorFat,
          values: fat.whereType<double>().toList(),
          unit: '%'),
      _Metric.lean => ChartSeries(
          label: 'Lean mass',
          color: colorLean,
          values: lean.whereType<double>().toList(),
          unit: 'kg'),
      _Metric.weight => ChartSeries(
          label: 'Weight', color: colorWeight, values: weight, unit: 'kg'),
    };

    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                if (hasFat)
                  _MetricTile(
                    label: 'Body fat',
                    values: fat.whereType<double>().toList(),
                    unit: '%',
                    color: colorFat,
                    goodWhenDown: true,
                    selected: _selected == _Metric.fat,
                    onTap: () => setState(() => _selected = _Metric.fat),
                  ),
                if (hasLean)
                  _MetricTile(
                    label: 'Lean mass',
                    values: lean.whereType<double>().toList(),
                    unit: 'kg',
                    color: colorLean,
                    goodWhenDown: false,
                    selected: _selected == _Metric.lean,
                    onTap: () => setState(() => _selected = _Metric.lean),
                  ),
                _MetricTile(
                  label: 'Weight',
                  values: weight,
                  unit: 'kg',
                  color: colorWeight,
                  goodWhenDown: null,
                  selected: _selected == _Metric.weight,
                  onTap: () => setState(() => _selected = _Metric.weight),
                ),
              ],
            ),
            const SizedBox(height: 14),
            Row(
              children: [
                Container(
                  width: 10,
                  height: 10,
                  decoration: BoxDecoration(
                      color: selectedSeries.color, shape: BoxShape.circle),
                ),
                const SizedBox(width: 6),
                Text(
                  '${selectedSeries.label} over time',
                  style: Theme.of(context).textTheme.labelMedium,
                ),
              ],
            ),
            const SizedBox(height: 8),
            TrendChart(series: [selectedSeries]),
          ],
        ),
      ),
    );
  }
}

/// A tappable metric tile: current value, change since start (coloured by
/// whether the direction is good), and a sparkline.
class _MetricTile extends StatelessWidget {
  const _MetricTile({
    required this.label,
    required this.values,
    required this.unit,
    required this.color,
    required this.goodWhenDown,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final List<double> values;
  final String unit;
  final Color color;

  /// true = down is good (fat), false = up is good (lean), null = neutral.
  final bool? goodWhenDown;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final current = values.last;
    final delta = values.last - values.first;
    final up = delta >= 0;
    final good =
        goodWhenDown == null ? null : (goodWhenDown! ? delta < 0 : delta > 0);
    final deltaColor =
        good == null ? cs.onSurfaceVariant : (good ? cs.primary : cs.error);

    return Expanded(
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 10),
          decoration: BoxDecoration(
            color: selected ? cs.surfaceContainerHigh : null,
            borderRadius: BorderRadius.circular(12),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(label.toUpperCase(),
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                        color: cs.onSurfaceVariant,
                        letterSpacing: 0.6,
                      )),
              const SizedBox(height: 4),
              Text(
                '${_fmt(current)}$unit',
                style: Theme.of(context)
                    .textTheme
                    .titleMedium
                    ?.copyWith(fontWeight: FontWeight.w800),
              ),
              const SizedBox(height: 2),
              Row(
                children: [
                  Icon(up ? Icons.arrow_upward : Icons.arrow_downward,
                      size: 12, color: deltaColor),
                  Text(
                    '${_fmt(delta.abs())}$unit',
                    style: Theme.of(context).textTheme.labelSmall?.copyWith(
                        color: deltaColor, fontWeight: FontWeight.w700),
                  ),
                ],
              ),
              const SizedBox(height: 6),
              Sparkline(values: values, color: color, width: double.infinity),
            ],
          ),
        ),
      ),
    );
  }

  static String _fmt(double v) =>
      v == v.roundToDouble() ? v.toStringAsFixed(0) : v.toStringAsFixed(1);
}

class _StrengthCard extends StatefulWidget {
  const _StrengthCard({required this.workouts});
  final List<WorkoutSession> workouts;

  @override
  State<_StrengthCard> createState() => _StrengthCardState();
}

class _StrengthCardState extends State<_StrengthCard> {
  /// null = "All lifts" (best overall); otherwise the selected lift name.
  String? _lift;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final overall = computeStrengthTrend(widget.workouts);
    final lifts = computePerLiftTrends(widget.workouts);

    if (!overall.hasData) {
      return Card(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            children: [
              Icon(Icons.fitness_center, size: 48, color: cs.outlineVariant),
              const SizedBox(height: 10),
              const Text(
                'Log a few weighted sessions to see your strength climb.',
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      );
    }

    final selected =
        _lift == null ? null : lifts.firstWhere((l) => l.name == _lift);
    final e1rm =
        (selected?.best1rm ?? overall.best1rm).map((p) => p.value).toList();
    final vol = overall.volume.map((p) => p.value).toList();
    final title = selected == null
        ? 'Estimated 1RM · all lifts'
        : 'Estimated 1RM · ${selected.name}';

    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (lifts.isNotEmpty)
              SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: Row(
                  children: [
                    _LiftChip(
                      label: 'All lifts',
                      selected: _lift == null,
                      onTap: () => setState(() => _lift = null),
                    ),
                    for (final l in lifts)
                      _LiftChip(
                        label: l.name,
                        pr: l.isPr,
                        selected: _lift == l.name,
                        onTap: () => setState(() => _lift = l.name),
                      ),
                  ],
                ),
              ),
            const SizedBox(height: 6),
            if (e1rm.length >= 2) ...[
              Row(
                children: [
                  Icon(Icons.trending_up, size: 16, color: cs.primary),
                  const SizedBox(width: 6),
                  Expanded(
                    child: Text(title,
                        style: Theme.of(context).textTheme.labelMedium,
                        overflow: TextOverflow.ellipsis),
                  ),
                  if (selected?.isPr ?? false) _PrBadge(cs: cs),
                  const SizedBox(width: 8),
                  Text('${e1rm.last.round()} kg',
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.w800,
                            color: cs.primary,
                          )),
                ],
              ),
              if (selected != null && selected.gain != 0) ...[
                const SizedBox(height: 2),
                Text(
                  '${selected.gain > 0 ? '+' : ''}${selected.gain.round()} kg '
                  'since you started tracking it',
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                        color: selected.gain > 0 ? cs.primary : cs.error,
                        fontWeight: FontWeight.w700,
                      ),
                ),
              ],
              const SizedBox(height: 8),
              TrendChart(series: [
                ChartSeries(
                    label: 'e1RM', color: cs.primary, values: e1rm, unit: 'kg')
              ]),
            ],
            if (_lift == null && vol.length >= 2) ...[
              const SizedBox(height: 14),
              Row(
                children: [
                  Icon(Icons.stacked_bar_chart, size: 16, color: cs.tertiary),
                  const SizedBox(width: 6),
                  Text('Session volume',
                      style: Theme.of(context).textTheme.labelMedium),
                  const Spacer(),
                  Text('${vol.last.round()} kg',
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.w800,
                            color: cs.tertiary,
                          )),
                ],
              ),
              const SizedBox(height: 8),
              TrendChart(
                height: 120,
                series: [
                  ChartSeries(
                      label: 'Volume',
                      color: cs.tertiary,
                      values: vol,
                      unit: 'kg')
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _LiftChip extends StatelessWidget {
  const _LiftChip({
    required this.label,
    required this.selected,
    required this.onTap,
    this.pr = false,
  });

  final String label;
  final bool selected;
  final bool pr;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(right: 8),
      child: ChoiceChip(
        label: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(label),
            if (pr) ...[
              const SizedBox(width: 4),
              Icon(Icons.emoji_events, size: 13, color: cs.tertiary),
            ],
          ],
        ),
        selected: selected,
        onSelected: (_) => onTap(),
      ),
    );
  }
}

class _PrBadge extends StatelessWidget {
  const _PrBadge({required this.cs});
  final ColorScheme cs;

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.only(right: 4),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: cs.tertiary.withValues(alpha: 0.18),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.emoji_events, size: 12, color: cs.tertiary),
          const SizedBox(width: 3),
          Text('PR',
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                    color: cs.tertiary,
                    fontWeight: FontWeight.w800,
                  )),
        ],
      ),
    );
  }
}

/// Horizontal volume bars per muscle group — training-balance at a glance.
class _MuscleBalance extends StatelessWidget {
  const _MuscleBalance({required this.balance});
  final List<({MuscleGroup group, double volume})> balance;

  static const _labels = {
    MuscleGroup.chest: 'Chest',
    MuscleGroup.back: 'Back',
    MuscleGroup.shoulders: 'Shoulders',
    MuscleGroup.arms: 'Arms',
    MuscleGroup.legs: 'Legs',
    MuscleGroup.core: 'Core',
  };

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final max = balance.first.volume;
    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (final b in balance) ...[
              Row(
                children: [
                  SizedBox(
                    width: 78,
                    child: Text(_labels[b.group] ?? b.group.wire,
                        style: Theme.of(context).textTheme.labelMedium),
                  ),
                  Expanded(
                    child: ClipRRect(
                      borderRadius: BorderRadius.circular(999),
                      child: LinearProgressIndicator(
                        value: max <= 0 ? 0 : (b.volume / max),
                        minHeight: 10,
                        backgroundColor: cs.surfaceContainerHighest,
                        color: cs.primary,
                      ),
                    ),
                  ),
                  const SizedBox(width: 10),
                  SizedBox(
                    width: 62,
                    child: Text('${b.volume.round()} kg',
                        textAlign: TextAlign.right,
                        style: Theme.of(context).textTheme.labelSmall?.copyWith(
                              color: cs.onSurfaceVariant,
                            )),
                  ),
                ],
              ),
              const SizedBox(height: 10),
            ],
          ],
        ),
      ),
    );
  }
}
