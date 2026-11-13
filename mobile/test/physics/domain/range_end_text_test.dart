// R-0059 AC3 -> SPEC-0059 §2.4 (slice 59-c2a, owner 2026-11-01): a range
// stopped by a joint's limit gives the degrees the bottom needs and the
// degrees the lifter has; the server's numbers, rounded for reading.

import 'package:fitai/src/physics/domain/range_end_text.dart';
import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/l10n.dart';

void main() {
  test('an ankle limit names the degrees needed and available', () {
    const end = Stopped(
      0.57,
      Rejection(
        reason: RejectionReason.depthNotReachable,
        limiting: Joint.ankle,
        neededDeg: 38.2,
        availableDeg: 29,
      ),
    );
    expect(
      stoppedText(end, null, l10n),
      'Sin carga, tu tobillo te detiene al 57 % de la bajada. '
      'El fondo pide 38° y tienes 29°.',
    );
  });

  test('no degrees when the server sends none', () {
    expect(
      stoppedText(const Stopped(0.8, null), null, l10n),
      'Sin carga, la convención del modelo te detiene al 80 % de la bajada.',
    );
  });
}
