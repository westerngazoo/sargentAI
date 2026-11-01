// SPEC-0059 §2.4 (R-0059 AC5) — the caveats under the figure, in words.
// Formatting only: every assumption is the server's; none is added or
// dropped here, and a token this client cannot name still gets a line.

import '../../../l10n/app_localizations.dart';
import 'squat_range.dart';

/// The caveat lines for [frame] of [result], each once and in order: the
/// range's assumptions, then the frame's, then how the model places the
/// arms and the legs' convention, which are always shown.
/// `table_derived_lengths` names the lengths in `result.tableLengths`, and
/// `joints_advance_together` is the convention line itself.
List<String> caveatLines(
  SquatRangeResult result,
  RangeFrame frame,
  AppLocalizations l10n,
) {
  final table = result.tableLengths.isEmpty
      ? null
      : l10n.squatTableLengths(
          result.tableLengths.map(l10n.squatLength).join(', '));
  String line(Assumption a) => switch (a.assumption) {
        'joints_advance_together' => l10n.squatCaveatConvention,
        'table_derived_lengths' => table ?? l10n.squatAssumption(a.assumption),
        final token => l10n.squatAssumption(token),
      };
  return {
    for (final a in [...result.assumptions, ...frame.mechanics.assumptions])
      line(a),
    if (table != null) table,
    l10n.squatCaveatArms,
    l10n.squatCaveatConvention,
  }.toList();
}
