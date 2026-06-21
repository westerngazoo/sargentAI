// R-0014 / SPEC-0014 §2.5.2 — ProgramProposalsScreen.
//
// Renders the top-3 archetype proposals. Cards are exclusively expandable —
// tapping one collapses any currently-expanded card. The "Choose this program"
// button is only visible in the expanded card; tapping it calls
// [ProgramService.chooseProgram] and navigates to [ProgramDetailScreen].

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/network/api_exception.dart';
import '../models/program_proposal.dart';
import '../services/program_service.dart';

class ProgramProposalsScreen extends ConsumerStatefulWidget {
  const ProgramProposalsScreen({super.key, required this.sessionId});

  final String sessionId;

  @override
  ConsumerState<ProgramProposalsScreen> createState() =>
      _ProgramProposalsScreenState();
}

class _ProgramProposalsScreenState
    extends ConsumerState<ProgramProposalsScreen> {
  // Async state
  ProposalsResponse? _proposals;
  Object? _error;
  bool _loading = true;

  // Which card index is expanded (null = none).
  int? _expandedIndex;

  // Per-card "choose in-flight" state.
  int? _choosingIndex;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final result =
          await ref.read(programServiceProvider).getProposals(widget.sessionId);
      if (mounted) {
        setState(() {
          _proposals = result;
          _loading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = e;
          _loading = false;
        });
      }
    }
  }

  Future<void> _choose(int index, ProgramProposal proposal) async {
    setState(() => _choosingIndex = index);
    try {
      await ref
          .read(programServiceProvider)
          .chooseProgram(widget.sessionId, proposal.archetypeId);
      if (!mounted) return;
      context.go('/programs/current');
    } on ApiException catch (e) {
      if (!mounted) return;
      final msg = e.statusCode == 409
          ? 'Selection no longer available — please refresh.'
          : 'Could not choose program — please retry.';
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));
    } finally {
      if (mounted) setState(() => _choosingIndex = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Choose your program')),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    if (_loading) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text("Could not load proposals"),
            TextButton(onPressed: _load, child: const Text('Retry')),
          ],
        ),
      );
    }
    final proposals = _proposals!.proposals;
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: proposals.length,
      itemBuilder: (context, i) => _ProposalCard(
        proposal: proposals[i],
        rank: i + 1,
        expanded: _expandedIndex == i,
        choosing: _choosingIndex == i,
        onTap: () => setState(() {
          _expandedIndex = _expandedIndex == i ? null : i;
        }),
        onChoose: () => _choose(i, proposals[i]),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Individual proposal card
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
              // Header row
              Row(
                children: [
                  Expanded(
                    child: Text(
                      proposal.displayName,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                  ),
                  Chip(label: Text(_rankLabel)),
                ],
              ),
              const SizedBox(height: 4),
              Text(proposal.summary,
                  style: Theme.of(context).textTheme.bodySmall),
              const SizedBox(height: 8),
              // Collapsed summary row
              Text('${p.daysPerWeek} days/week · ${d.estimatedKcal} kcal/day'),
              // Highlight exercises chips (always visible)
              const SizedBox(height: 8),
              Wrap(
                spacing: 6,
                children: p.highlightExercises
                    .take(3)
                    .map((e) => Chip(label: Text(e)))
                    .toList(),
              ),
              // Expanded detail
              if (expanded) ...[
                const Divider(),
                Text('Intensity: ${p.intensityGuidance}'),
                const SizedBox(height: 4),
                Text('Rest: ${p.restGuidance}'),
                const SizedBox(height: 4),
                Text('Progression: ${p.progressionGuidance}'),
                const SizedBox(height: 8),
                // Macro table
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceAround,
                  children: [
                    _MacroCell('Protein', '${d.proteinG}g'),
                    _MacroCell('Carbs', '${d.carbsG}g'),
                    _MacroCell('Fat', '${d.fatG}g'),
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

class _MacroCell extends StatelessWidget {
  const _MacroCell(this.label, this.value);

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
