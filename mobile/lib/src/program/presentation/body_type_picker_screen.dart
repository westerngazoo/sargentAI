// R-0030 — Visual body-type picker: shape grid + fat-band chips → proposals.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/network/api_exception.dart';
import '../models/synthetic_match.dart';
import '../services/program_service.dart';
import 'synthetic_proposals_screen.dart';

class BodyTypePickerScreen extends ConsumerStatefulWidget {
  const BodyTypePickerScreen({super.key});

  @override
  ConsumerState<BodyTypePickerScreen> createState() =>
      _BodyTypePickerScreenState();
}

class _BodyTypePickerScreenState extends ConsumerState<BodyTypePickerScreen> {
  BodyShape? _shape;
  FatBand? _fatBand;
  bool _loading = false;
  String? _error;

  bool get _canConfirm => _shape != null && _fatBand != null && !_loading;

  Future<void> _confirm() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final result = await ref
          .read(programServiceProvider)
          .syntheticMatch(_shape!, _fatBand!);
      if (!mounted) return;
      await Navigator.of(context).push(
        MaterialPageRoute<void>(
          builder: (_) => SyntheticProposalsScreen(
            proposals: result.proposals,
            shape: _shape!,
            fatBand: _fatBand!,
          ),
        ),
      );
    } on ApiException catch (e) {
      if (!mounted) return;
      setState(() => _error = e.message);
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Choose your body type'),
        leading: BackButton(onPressed: () => context.go('/home')),
      ),
      body: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Expanded(
              child: SingleChildScrollView(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Text('Body shape',
                        style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 8),
                    _ShapeGrid(
                      selected: _shape,
                      onSelect: (s) => setState(() => _shape = s),
                    ),
                    if (_shape != null) ...[
                      const SizedBox(height: 24),
                      Text('Body fat level',
                          style: Theme.of(context).textTheme.titleMedium),
                      const SizedBox(height: 8),
                      _FatBandChips(
                        selected: _fatBand,
                        onSelect: (b) => setState(() => _fatBand = b),
                      ),
                    ],
                    if (_error != null) ...[
                      const SizedBox(height: 12),
                      Text(
                        _error!,
                        style: TextStyle(
                            color: Theme.of(context).colorScheme.error),
                      ),
                    ],
                  ],
                ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.all(16),
              child: FilledButton(
                onPressed: _canConfirm ? _confirm : null,
                child: _loading
                    ? const SizedBox(
                        height: 20,
                        width: 20,
                        child: CircularProgressIndicator(
                            strokeWidth: 2, color: Colors.white),
                      )
                    : const Text('Find my program'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Shape grid — 3 cards, one per body shape
// ---------------------------------------------------------------------------

class _ShapeGrid extends StatelessWidget {
  const _ShapeGrid({required this.selected, required this.onSelect});

  final BodyShape? selected;
  final ValueChanged<BodyShape> onSelect;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: BodyShape.values
          .map((s) => _ShapeCard(
                shape: s,
                isSelected: selected == s,
                onTap: () => onSelect(s),
              ))
          .toList(),
    );
  }
}

class _ShapeCard extends StatelessWidget {
  const _ShapeCard({
    required this.shape,
    required this.isSelected,
    required this.onTap,
  });

  final BodyShape shape;
  final bool isSelected;
  final VoidCallback onTap;

  IconData get _icon => switch (shape) {
        BodyShape.ectomorph => Icons.straighten,
        BodyShape.mesomorph => Icons.fitness_center,
        BodyShape.endomorph => Icons.circle_outlined,
      };

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      color: isSelected ? cs.primaryContainer : null,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: isSelected
            ? BorderSide(color: cs.primary, width: 2)
            : BorderSide.none,
      ),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Icon(_icon,
                  size: 40,
                  color: isSelected ? cs.primary : cs.onSurfaceVariant),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(shape.label,
                        style: Theme.of(context)
                            .textTheme
                            .titleSmall
                            ?.copyWith(
                                color: isSelected ? cs.primary : null,
                                fontWeight: FontWeight.w600)),
                    const SizedBox(height: 2),
                    Text(shape.description,
                        style: Theme.of(context).textTheme.bodySmall),
                  ],
                ),
              ),
              if (isSelected) Icon(Icons.check_circle, color: cs.primary),
            ],
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Fat-band chips — 3 chips shown after shape is selected
// ---------------------------------------------------------------------------

class _FatBandChips extends StatelessWidget {
  const _FatBandChips({required this.selected, required this.onSelect});

  final FatBand? selected;
  final ValueChanged<FatBand> onSelect;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: FatBand.values
          .map((b) => _FatBandTile(
                band: b,
                isSelected: selected == b,
                onTap: () => onSelect(b),
              ))
          .toList(),
    );
  }
}

class _FatBandTile extends StatelessWidget {
  const _FatBandTile({
    required this.band,
    required this.isSelected,
    required this.onTap,
  });

  final FatBand band;
  final bool isSelected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return ChoiceChip(
      label: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(band.label,
              style: const TextStyle(fontWeight: FontWeight.w600)),
          Text(band.sublabel,
              style: Theme.of(context).textTheme.bodySmall),
        ],
      ),
      selected: isSelected,
      onSelected: (_) => onTap(),
      selectedColor: cs.primaryContainer,
      showCheckmark: false,
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: isSelected
            ? BorderSide(color: cs.primary, width: 1.5)
            : BorderSide(color: cs.outline),
      ),
    );
  }
}
