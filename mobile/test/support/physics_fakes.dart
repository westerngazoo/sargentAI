// Shared doubles for the SPEC-0059 squat check (slice 59-b).

import 'dart:convert';
import 'dart:io';

import 'package:fitai/src/physics/data/squat_check_api.dart';
import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:mocktail/mocktail.dart';

/// The backend's golden `POST /biomech/squat/range` body (59-a): the real
/// wire, read from the one copy so the client cannot drift from it.
Map<String, dynamic> squatGoldenJson() => jsonDecode(
    File('../backend/crates/api/tests/fixtures/biomech_squat_range_golden.json')
        .readAsStringSync()) as Map<String, dynamic>;

/// The golden, parsed.
SquatRangeResult squatGolden() => SquatRangeResult.fromJson(squatGoldenJson());

class MockSquatCheckApi extends Mock implements SquatCheckApi {}

/// The SPEC-0059 §2.2 example request.
const sampleSquatRequest = SquatRangeRequest(
  heightM: 1.78,
  massKg: 77.0,
  dorsiflexionDeg: 29.0,
  barKg: 84.0,
);

/// Registers the mocktail fallback for `range(any())`.
void registerSquatFallbacks() => registerFallbackValue(sampleSquatRequest);

/// The golden's frames as a bodyweight-only range: no bar in the posture and
/// no hip lever, as the server sends for `bar_kg: null`. A test double for
/// drawing only; the numbers are the golden's.
List<RangeFrame> squatGoldenBodyweightFrames() => [
      for (final f in squatGolden().frames)
        RangeFrame(
          progress: f.progress,
          kneeFlexionDeg: f.kneeFlexionDeg,
          hipFlexionDeg: f.hipFlexionDeg,
          trunkLeanDeg: f.trunkLeanDeg,
          posture: Posture(
            ankle: f.posture.ankle,
            knee: f.posture.knee,
            hip: f.posture.hip,
            shoulder: f.posture.shoulder,
          ),
          mechanics: f.mechanics,
          groundN: f.groundN,
        ),
    ];

/// A result with [end] over [frames] (the golden's when omitted).
SquatRangeResult squatResult(End end, {List<RangeFrame>? frames}) =>
    SquatRangeResult(
      end: end,
      tableLengths: const ['trunk'],
      frames: frames ?? squatGolden().frames,
      assumptions: const [],
    );
