// SPEC-0059 §2.4 (R-0059 AC3) — how a squat range ended, in plain words.
// Formatting only: the reach and the reason are the server's.

import 'package:intl/intl.dart';

import '../../../l10n/app_localizations.dart';
import '../../l10n/locale_rule.dart';
import 'squat_range.dart';

/// The catalogue's token for why the model stops at [r]: the limiting joint
/// for a depth limit, `balance`, `implausible`, or `other` when there is no
/// rejection (only the model's convention stops the range).
String _cause(Rejection? r) => switch (r) {
      null => 'other',
      Rejection(reason: RejectionReason.doesNotBalance) => 'balance',
      Rejection(reason: RejectionReason.anatomicallyImplausible) =>
        'implausible',
      Rejection(limiting: final joint?) => joint.name,
      Rejection() => 'other',
    };

/// "Sin carga, tu tobillo te detiene al 57 % de la bajada." for a [Stopped]
/// range, with the load from the result's own bar ([barKg], null for
/// bodyweight only).
String stoppedText(Stopped end, double? barKg, AppLocalizations l10n) {
  final load = barKg == null
      ? l10n.squatWithoutLoad
      : l10n.squatWithLoad(
          NumberFormat.decimalPattern(numberLocale(l10n.localeName))
              .format(barKg));
  return l10n.squatStopped(
      load, _cause(end.bottom), (end.reached * 100).round());
}

/// Why the model could not place the lifter at all, for a [Rejected] range.
String rejectedText(Rejection reason, AppLocalizations l10n) =>
    l10n.squatRejected(_cause(reason));
