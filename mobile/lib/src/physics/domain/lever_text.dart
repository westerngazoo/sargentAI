// SPEC-0059 §2.3 — "La cuenta": formatting only. Every number is the
// server's (§2.2); this file does no arithmetic on them (R-0059 AC7).

import 'package:intl/intl.dart';

import '../../../l10n/app_localizations.dart';
import 'squat_range.dart';

/// The hip's sum for [frame] in [l10n]'s language and number format, e.g.
/// "824 N × 0,03 m = 23,8 N·m, + 9,7 del cuerpo = 33,5 N·m". The total is
/// the server's hip moment, not a sum made here. Null for a bodyweight-only
/// frame, which has no bar lever.
String? leverText(RangeFrame frame, AppLocalizations l10n) {
  final hip = frame.hip;
  if (hip == null) return null;
  String format(double value, int digits) => NumberFormat.decimalPatternDigits(
        locale: l10n.localeName,
        decimalDigits: digits,
      ).format(value);
  return l10n.squatLeverSum(
    format(hip.barN, 0),
    format(hip.leverM, 2),
    format(hip.barNm, 1),
    format(hip.bodyNm, 1),
    format(frame.mechanics.moment(Joint.hip), 1),
  );
}
