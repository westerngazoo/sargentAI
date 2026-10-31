// R-0032 (slice 1) — minimal meal quick-log sheet over POST /nutrition.
//
// Opened from + Comida. Full nutrition logger UI remains R-0010 scope.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../l10n/app_localizations.dart';
import '../../core/network/api_error_text.dart';
import '../../core/network/api_exception.dart';
import '../../core/panels/show_panel.dart';
import '../domain/preset_meals.dart';
import '../services/nutrition_service.dart';

/// Opens the Comida panel (SPEC-0061 §2.6); resolves once dismissed.
Future<void> showMealQuickLogSheet(
  BuildContext context, {
  double? proteinG,
  double? carbsG,
  double? fatG,
}) =>
    showPanel<void>(
      context,
      title: AppLocalizations.of(context).logAMeal,
      body: (_) => MealQuickLogForm(
        initialProteinG: proteinG,
        initialCarbsG: carbsG,
        initialFatG: fatG,
      ),
    );

/// The Comida panel's body: presets, the three macros and the save button.
/// Saving pops the panel it sits in.
class MealQuickLogForm extends ConsumerStatefulWidget {
  const MealQuickLogForm({
    super.key,
    this.initialProteinG,
    this.initialCarbsG,
    this.initialFatG,
  });

  /// Protein to prefill, as dictated; null leaves the field empty.
  final double? initialProteinG;

  /// Carbs to prefill, as dictated; null leaves the field empty.
  final double? initialCarbsG;

  /// Fat to prefill, as dictated; null leaves the field empty.
  final double? initialFatG;

  @override
  ConsumerState<MealQuickLogForm> createState() => _MealQuickLogFormState();
}

class _MealQuickLogFormState extends ConsumerState<MealQuickLogForm> {
  late final TextEditingController _protein;
  late final TextEditingController _carbs;
  late final TextEditingController _fat;
  bool _saving = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _protein = TextEditingController(text: _fmt(widget.initialProteinG));
    _carbs = TextEditingController(text: _fmt(widget.initialCarbsG));
    _fat = TextEditingController(text: _fmt(widget.initialFatG));
  }

  @override
  void dispose() {
    _protein.dispose();
    _carbs.dispose();
    _fat.dispose();
    super.dispose();
  }

  static String _fmt(double? v) =>
      v == null ? '' : (v == v.roundToDouble() ? v.toStringAsFixed(0) : '$v');

  double? get _p => double.tryParse(_protein.text);
  double? get _c => double.tryParse(_carbs.text);
  double? get _f => double.tryParse(_fat.text);

  bool get _valid =>
      _p != null &&
      _c != null &&
      _f != null &&
      _p! >= 0 &&
      _c! >= 0 &&
      _f! >= 0;

  double get _kcal => 4 * (_p ?? 0) + 4 * (_c ?? 0) + 9 * (_f ?? 0);

  Future<void> _save() async {
    final l10n = AppLocalizations.of(context);
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      final today = DateTime.now();
      final iso = '${today.year.toString().padLeft(4, '0')}-'
          '${today.month.toString().padLeft(2, '0')}-'
          '${today.day.toString().padLeft(2, '0')}';
      await ref.read(nutritionServiceProvider).create(
            performedOn: iso,
            proteinG: _p!,
            carbsG: _c!,
            fatG: _f!,
          );
      if (!mounted) return;
      Navigator.of(context).pop();
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.mealLogged)),
      );
    } on ApiException catch (e) {
      if (!mounted) return;
      setState(() => _error = apiErrorText(e, l10n));
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Preset meals — tap to prefill, tweak, save.
        SizedBox(
          height: 40,
          child: ListView(
            scrollDirection: Axis.horizontal,
            children: [
              for (final meal in presetMeals)
                Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: ActionChip(
                    label: Text(meal.name),
                    onPressed: () => setState(() {
                      _protein.text = _fmt(meal.proteinG);
                      _carbs.text = _fmt(meal.carbsG);
                      _fat.text = _fmt(meal.fatG);
                    }),
                  ),
                ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(child: _macroField(l10n.proteinGramsLabel, _protein)),
            const SizedBox(width: 8),
            Expanded(child: _macroField(l10n.carbsGramsLabel, _carbs)),
            const SizedBox(width: 8),
            Expanded(child: _macroField(l10n.fatGramsLabel, _fat)),
          ],
        ),
        const SizedBox(height: 8),
        Text(
          l10n.kcalLoggedForToday(_kcal.toStringAsFixed(0)),
          style: Theme.of(context).textTheme.bodySmall,
        ),
        if (_error != null) ...[
          const SizedBox(height: 8),
          Text(
            _error!,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        ],
        const SizedBox(height: 12),
        FilledButton(
          onPressed: _valid && !_saving ? _save : null,
          child: _saving
              ? const SizedBox(
                  height: 20,
                  width: 20,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text(l10n.saveMeal),
        ),
      ],
    );
  }

  Widget _macroField(String label, TextEditingController controller) =>
      TextField(
        controller: controller,
        keyboardType: const TextInputType.numberWithOptions(decimal: true),
        decoration: InputDecoration(labelText: label),
        onChanged: (_) => setState(() {}),
      );
}
