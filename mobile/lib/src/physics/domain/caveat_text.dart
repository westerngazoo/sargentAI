// SPEC-0059 §2.4 (R-0059 AC5) — the caveats, in words: the key lines under
// the figure and the assumptions closing the card. Formatting only: every
// assumption is the server's; none is added or dropped here, and a token
// this client cannot name still gets a line.

import '../../../l10n/app_localizations.dart';
import 'squat_range.dart';

/// The caveats of a frame: [key] lines read first, [assumptions] in full.
typedef Caveats = ({List<String> key, List<String> assumptions});

/// The caveats for [frame] of [result].
///
/// [Caveats.key] is, in order, the lengths that came from the table (when
/// `result.tableLengths` names any), how the model places the arms, and the
/// legs' convention; the last two are always there. [Caveats.assumptions]
/// is the range's assumptions then the frame's, each line once, without a
/// line [Caveats.key] already says: `joints_advance_together` is the
/// convention line, and `table_derived_lengths` the table line when there
/// is one.
Caveats caveatLines(
  SquatRangeResult result,
  RangeFrame frame,
  AppLocalizations l10n,
) {
  final table = result.tableLengths.isEmpty
      ? null
      : l10n.squatTableLengths(
          result.tableLengths.map(l10n.squatLength).join(', '));
  final key = [
    if (table != null) table,
    l10n.squatCaveatArms,
    l10n.squatCaveatConvention,
  ];
  String line(Assumption a) => switch (a.assumption) {
        'joints_advance_together' => l10n.squatCaveatConvention,
        'table_derived_lengths' => table ?? l10n.squatAssumption(a.assumption),
        final token => l10n.squatAssumption(token),
      };
  final assumptions = {
    for (final a in [...result.assumptions, ...frame.mechanics.assumptions])
      line(a),
  }..removeAll(key);
  return (key: key, assumptions: assumptions.toList());
}
