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
