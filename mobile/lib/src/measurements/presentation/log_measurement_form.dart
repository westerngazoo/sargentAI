// Quick "log measurement" form — weight (required) + body fat % (optional).
// Upserts today's measurement.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../l10n/app_localizations.dart';
import '../../core/network/api_error_text.dart';
import '../../core/network/api_exception.dart';
import '../services/measurement_service.dart';

/// The weigh-in form of the Medidas panel (SPEC-0061 §2.6). Saving pops the
/// panel it sits in.
class LogMeasurementForm extends ConsumerStatefulWidget {
  const LogMeasurementForm({super.key});

  @override
  ConsumerState<LogMeasurementForm> createState() => _LogMeasurementFormState();
}

class _LogMeasurementFormState extends ConsumerState<LogMeasurementForm> {
  final _weight = TextEditingController();
  final _fat = TextEditingController();
  bool _saving = false;
  String? _error;

  @override
  void dispose() {
    _weight.dispose();
    _fat.dispose();
    super.dispose();
  }

  double? get _w => double.tryParse(_weight.text);
  double? get _bf =>
      _fat.text.trim().isEmpty ? null : double.tryParse(_fat.text);
  bool get _valid => _w != null && _w! > 0;

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
      await ref.read(measurementServiceProvider).create(
            measuredOn: iso,
            weightKg: _w!,
            bodyFatPercentage: _bf,
          );
      ref.invalidate(measurementsProvider);
      if (!mounted) return;
      Navigator.of(context).pop();
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.measurementLogged)),
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
        Text(l10n.logMeasurement,
            style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 4),
        Text(
          l10n.weighInHint,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
        ),
        const SizedBox(height: 14),
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _weight,
                keyboardType:
                    const TextInputType.numberWithOptions(decimal: true),
                decoration: InputDecoration(labelText: l10n.weightKgLabel),
                onChanged: (_) => setState(() {}),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: TextField(
                controller: _fat,
                keyboardType:
                    const TextInputType.numberWithOptions(decimal: true),
                decoration:
                    InputDecoration(labelText: l10n.bodyFatPercentInputLabel),
              ),
            ),
          ],
        ),
        if (_error != null) ...[
          const SizedBox(height: 8),
          Text(_error!,
              style: TextStyle(color: Theme.of(context).colorScheme.error)),
        ],
        const SizedBox(height: 14),
        FilledButton(
          onPressed: _valid && !_saving ? _save : null,
          child: _saving
              ? const SizedBox(
                  height: 20,
                  width: 20,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text(l10n.saveMeasurement),
        ),
      ],
    );
  }
}
