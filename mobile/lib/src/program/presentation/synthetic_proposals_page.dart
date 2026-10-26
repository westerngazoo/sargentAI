// R-0030 — Proposals for the synthetic (no-photo) path.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../l10n/app_localizations.dart';
import '../../core/dates.dart';
import '../../core/network/api_error_text.dart';
import '../../core/network/api_exception.dart';
import '../../core/panels/panel_page_header.dart';
import '../application/program_providers.dart';
import '../models/program_proposal.dart';
import '../models/synthetic_match.dart';
import '../services/program_service.dart';

/// The last page of the Programa panel's flow (SPEC-0061 §2.6): the
/// proposals [BodyTypePickerPage] got for [shape] and [fatBand]. Choosing
/// one calls `POST /programs/synthetic` with today as `starts_on` and returns
/// the flow to its first page.
class SyntheticProposalsPage extends ConsumerStatefulWidget {
  const SyntheticProposalsPage({
    super.key,
    required this.proposals,
    required this.shape,
    required this.fatBand,
  });

  /// The proposals, ranked best first.
  final List<ProgramProposal> proposals;

  /// The picker's choices, sent back with the chosen archetype.
  final BodyShape shape;
  final FatBand fatBand;

  @override
  ConsumerState<SyntheticProposalsPage> createState() =>
      _SyntheticProposalsPageState();
}

class _SyntheticProposalsPageState
    extends ConsumerState<SyntheticProposalsPage> {
  int? _expandedIndex;
  int? _choosingIndex;

  Future<void> _choose(int index, ProgramProposal proposal) async {
    setState(() => _choosingIndex = index);
    try {
      await ref.read(programServiceProvider).chooseSyntheticProgram(
            proposal.archetypeId,
            widget.shape,
            widget.fatBand,
            startsOn: dateOnly(DateTime.now()),
          );
      if (!mounted) return;
      // The detail page, first in the flow, re-reads the program; the
      // schedule follows through its watch on it (SPEC-0061 §2.6).
      ref.invalidate(currentProgramProvider);
      Navigator.of(context).popUntil((route) => route.isFirst);
    } on ApiException catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
          content: Text(apiErrorText(e, AppLocalizations.of(context)))));
    } finally {
      if (mounted) setState(() => _choosingIndex = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        PanelPageHeader(title: AppLocalizations.of(context).chooseYourProgram),
        Expanded(
          child: ListView.builder(
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
        ),
      ],
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

  String _rankLabel(AppLocalizations l10n) => switch (rank) {
        1 => l10n.rankBestMatch,
        2 => l10n.rankCloseMatch,
        _ => l10n.rankGoodOption,
      };

  @override
  Widget build(BuildContext context) {
    final p = proposal.program;
    final d = proposal.diet;
    final l10n = AppLocalizations.of(context);
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
                  Chip(label: Text(_rankLabel(l10n))),
                ],
              ),
              const SizedBox(height: 4),
              Text(proposal.summary,
                  style: Theme.of(context).textTheme.bodySmall),
              const SizedBox(height: 8),
              Text(l10n.proposalSummaryLine(p.daysPerWeek, d.estimatedKcal)),
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
                Text(l10n.intensityLine(p.intensityGuidance)),
                const SizedBox(height: 4),
                Text(l10n.restLine(p.restGuidance)),
                const SizedBox(height: 4),
                Text(l10n.progressionLine(p.progressionGuidance)),
                const SizedBox(height: 8),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceAround,
                  children: [
                    _Macro(l10n.protein, l10n.grams(d.proteinG)),
                    _Macro(l10n.carbs, l10n.grams(d.carbsG)),
                    _Macro(l10n.fat, l10n.grams(d.fatG)),
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
                        : Text(l10n.chooseThisProgram),
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
