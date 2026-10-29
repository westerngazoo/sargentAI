// R-0059 AC2, AC7 -> SPEC-0059 §2.3 (slice 59-b): the controller builds the
// request from the profile (`height_cm / 100`), the session's dorsiflexion
// (no default) and the chosen load; exposes the result as an AsyncValue,
// surfacing a 400 as an error; and delegates playback to playbackAt.

import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/physics/application/squat_check_controller.dart';
import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:fitai/src/profile/application/profile_providers.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/physics_fakes.dart';
import '../../support/profile_fakes.dart';

void main() {
  setUpAll(registerSquatFallbacks);

  late MockSquatCheckApi api;

  ProviderContainer makeContainer({double? dorsiflexion, double? barKg}) {
    final c = ProviderContainer(overrides: [
      squatCheckApiProvider.overrideWithValue(api),
      // 180 cm, 82.5 kg.
      profileProvider.overrideWith((_) async => sampleProfile()),
    ]);
    addTearDown(c.dispose);
    c.read(squatDorsiflexionProvider.notifier).state = dorsiflexion;
    c.read(squatLoadKgProvider.notifier).state = barKg;
    return c;
  }

  setUp(() {
    api = MockSquatCheckApi();
    when(() => api.range(any())).thenAnswer((_) async => squatGolden());
  });

  group('the request', () {
    test('without a dorsiflexion there is nothing to ask the server', () async {
      final c = makeContainer();
      expect(await c.read(squatCheckControllerProvider.future), isNull);
      verifyNever(() => api.range(any()));
    });

    test('comes from the profile, the session dorsiflexion and the load',
        () async {
      final c = makeContainer(dorsiflexion: 29.0, barKg: 84.0);

      final r = await c.read(squatCheckControllerProvider.future);

      expect(r!.frames, hasLength(2));
      final req = verify(() => api.range(captureAny())).captured.single
          as SquatRangeRequest;
      expect(req.heightM, 1.8);
      expect(req.massKg, 82.5);
      expect(req.dorsiflexionDeg, 29.0);
      expect(req.barKg, 84.0);
      expect([req.thighM, req.shankM, req.trunkM], [null, null, null],
          reason: 'limb lengths wait for R-0058');
    });

    test('no chosen load is bodyweight only', () async {
      final c = makeContainer(dorsiflexion: 29.0);
      await c.read(squatCheckControllerProvider.future);
      final req = verify(() => api.range(captureAny())).captured.single
          as SquatRangeRequest;
      expect(req.barKg, isNull);
    });

    test('a 400 validation is surfaced as the error', () async {
      when(() => api.range(any())).thenThrow(const ApiException(
          ApiErrorKind.badRequest,
          statusCode: 400,
          field: 'dorsiflexion_deg'));
      final c = makeContainer(dorsiflexion: 29.0);

      await expectLater(
          c.read(squatCheckControllerProvider.future), throwsA(anything));

      final state = c.read(squatCheckControllerProvider);
      expect(
          state.error,
          isA<ApiException>()
              .having((e) => e.field, 'field', 'dorsiflexion_deg'));
    });
  });

  group('playback', () {
    test('delegates to playbackAt over the current result', () async {
      final c = makeContainer(dorsiflexion: 29.0, barKg: 84.0);
      await c.read(squatCheckControllerProvider.future);
      final last = squatGolden().frames.last;
      final p = c.read(squatCheckControllerProvider.notifier).frameAt(1.0)!;
      expect(p.frame.progress, last.progress);
      expect(p.posture.hip.y, last.posture.hip.y);
    });

    test('there is no frame before a result', () async {
      final c = makeContainer();
      await c.read(squatCheckControllerProvider.future);
      expect(c.read(squatCheckControllerProvider.notifier).frameAt(0), isNull);
    });
  });
}
