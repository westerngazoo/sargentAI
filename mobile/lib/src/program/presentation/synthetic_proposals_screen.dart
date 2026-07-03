// R-0030 — Proposals screen for the synthetic (no-photo) path.
//
// Receives pre-loaded proposals from [BodyTypePickerScreen] and calls
// `POST /programs/synthetic` when the user chooses. Navigates to
// `/programs/current` on success.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/network/api_exception.dart';
import '../application/program_providers.dart';
import '../models/program_proposal.dart';
import '../models/synthetic_match.dart';
import '../services/program_service.dart';

class SyntheticProposalsScreen extends ConsumerStatefulWidget {
  const SyntheticProposalsScreen({
    super.key,
    required this.proposals,
    required this.shape,
    required this.fatBand,
  });

  final List<ProgramProposal> proposals;
  final BodyShape shape;
  final FatBand fatBand;

  @override
  ConsumerState<SyntheticProposalsScreen> createState() =>
      _SyntheticProposalsScreenState();
}

class _SyntheticProposalsScreenState
    extends ConsumerState<SyntheticProposalsScreen> {
  int? _expandedIndex;
  int? _choosingIndex;

  Future<void> _choose(int index, ProgramProposal proposal) async {
    setState(() => _choosingIndex = index);
    try {
      await ref.read(programServiceProvider).chooseSyntheticProgram(
            proposal.archetypeId,
            widget.shape,
            widget.fatBand,
          );
      if (!mounted) return;
      // Invalidate the current-program cache so HomeShell picks it up.
      ref.invalidate(currentProgramProvider);
      context.go('/programs/current');
    } on ApiException catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(e.message)));
    } finally {
      if (mounted) setState(() => _choosingIndex = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Choose your program')),
      body: ListView.builder(
        padding: const EdgeInsets.all(16),
        itemCount: widget.proposals.length,
        itemBuilder: (context, i) {
          final p = widget.proposals[i];
          return _ProposalCard(
            proposal: p,
            rank: i + 1,
            expanded: _expandedIndex == i,
            choosing: _choosingIndex == i,
            onTap: () => setState(() {
              _expandedIndex = _expandedIndex == i ? null : i;
            }),
            onChoose: () => _choose(i, p),
          );
        },
      ),
    );
  }
}

// ---------------------------------------------------------------------------

class _ProposalCard extends StatelessWidget {
  const _ProposalCard({
    required this.proposal,
    required this.rank,
    required this.expanded,
    required this.choosing,
    required this.onTap,
    required this.onChoose,
  });

  final ProgramProposal proposal;
  final int rank;
  final bool expanded;
  final bool choosing;
  final VoidCallback onTap;
  final VoidCallback onChoose;

  String get _rankLabel => switch (rank) {
        1 => 'Best match',
        2 => 'Close match',
        _ => 'Good option',
      };

  @override
  Widget build(BuildContext context) {
    final p = proposal.program;
    final d = proposal.diet;
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(proposal.displayName,
                        style: Theme.of(context).textTheme.titleMedium),
                  ),
                  Chip(label: Text(_rankLabel)),
                ],
              ),
              const SizedBox(height: 4),
              Text(proposal.summary,
                  style: Theme.of(context).textTheme.bodySmall),
              const SizedBox(height: 8),
              Text('${p.daysPerWeek} days/week · ${d.estimatedKcal} kcal/day'),
              const SizedBox(height: 8),
              Wrap(
                spacing: 6,
                children: p.highlightExercises
                    .take(3)
                    .map((e) => Chip(label: Text(e)))
                    .toList(),
              ),
              if (expanded) ...[
                const Divider(),
                Text('Intensity: ${p.intensityGuidance}'),
                const SizedBox(height: 4),
                Text('Rest: ${p.restGuidance}'),
                const SizedBox(height: 4),
                Text('Progression: ${p.progressionGuidance}'),
                const SizedBox(height: 8),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceAround,
                  children: [
                    _Macro('Protein', '${d.proteinG}g'),
                    _Macro('Carbs', '${d.carbsG}g'),
                    _Macro('Fat', '${d.fatG}g'),
                  ],
                ),
                const SizedBox(height: 12),
                SizedBox(
                  width: double.infinity,
                  child: ElevatedButton(
                    onPressed: choosing ? null : onChoose,
                    child: choosing
                        ? const SizedBox(
                            height: 20,
                            width: 20,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Text('Choose this program'),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _Macro extends StatelessWidget {
  const _Macro(this.label, this.value);

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Column(
        children: [
          Text(label, style: Theme.of(context).textTheme.labelSmall),
          Text(value, style: Theme.of(context).textTheme.bodyMedium),
        ],
      );
}
