// R-0059 AC1, AC3 -> SPEC-0059 §2.2/§2.3 (slice 59-b): the client parses the
// `POST /biomech/squat/range` body exactly as the server writes it — the
// golden is the real wire — with `end` as a sealed class and bodyweight
// frames carrying no `hip`.

import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/physics_fakes.dart';

Map<String, dynamic> _with(Map<String, dynamic> end, {List<dynamic>? frames}) =>
    squatGoldenJson()
      ..['end'] = end
      ..['frames'] = frames ?? squatGoldenJson()['frames'];

void main() {
  group('SquatRangeResult.fromJson — the golden', () {
    test('parses the envelope', () {
      final r = squatGolden();
      expect(r.end, isA<Bottom>());
      expect(r.tableLengths, ['trunk']);
      expect(r.assumptions.map((a) => a.assumption),
          ['joints_advance_together', 'straight_body_upright']);
      expect(r.frames, hasLength(2));
    });

    test('parses a frame: angles, posture, the hip lever and ground force', () {
      final f = squatGolden().frames.first;
      expect(f.progress, 0.0);
      expect(f.shankDeg, closeTo(3.4028, 1e-4));
      expect(f.kneeFlexionDeg, 0.0);
      expect(f.hipFlexionDeg, 0.0);
      expect(f.trunkLeanDeg, closeTo(3.4028, 1e-4));
      expect(f.posture.hip.x, closeTo(-0.018707, 1e-6));
      expect(f.posture.shoulder.y, closeTo(1.449622, 1e-6));
      expect(f.posture.load!.kg, 84.0);
      expect(f.posture.load!.at.y, closeTo(1.424036, 1e-6));
      expect(f.hip!.leverM, closeTo(0.028906, 1e-6));
      expect(f.hip!.barN, closeTo(824.04, 1e-9));
      expect(f.hip!.barNm, closeTo(23.820002, 1e-6));
      expect(f.hip!.bodyNm, closeTo(9.697683, 1e-6));
      expect(f.groundN, closeTo(1579.41, 1e-9));
    });

    test('parses the mechanics: four moments, the load and its assumptions',
        () {
      final m = squatGolden().frames.first.mechanics;
      expect(m.joints.map((j) => j.joint),
          [Joint.hip, Joint.knee, Joint.ankle, Joint.lumbar]);
      expect(m.moment(Joint.hip), closeTo(33.517684, 1e-6));
      expect(m.moment(Joint.knee), closeTo(-71.494678, 1e-6),
          reason: 'a negative moment survives the parse');
      expect(m.joints.first.resisting, 'hip_extensors');
      expect(m.joints.last.uncertainty, closeTo(2.049756, 1e-6));
      expect(m.barKg, 84.0);
      final single =
          m.assumptions.firstWhere((a) => a.assumption == 'single_posture');
      expect(single.at, 'upright');
    });
  });

  group('end', () {
    test('stopped carries how far the lifter got and the bottom rejection', () {
      final r = SquatRangeResult.fromJson(_with({
        'kind': 'stopped',
        'reached': 0.565,
        'bottom': {
          'reason': 'depth_not_reachable',
          'limiting': 'ankle',
          'needed_deg': 31.5,
          'available_deg': 25.0,
        },
      }));
      final end = r.end as Stopped;
      expect(end.reached, 0.565);
      expect(end.bottom!.reason, RejectionReason.depthNotReachable);
      expect(end.bottom!.limiting, Joint.ankle);
      expect(end.bottom!.neededDeg, 31.5);
      expect(end.bottom!.availableDeg, 25.0);
    });

    test('stopped by the convention alone has no bottom rejection', () {
      final r = SquatRangeResult.fromJson(
          _with({'kind': 'stopped', 'reached': 0.9, 'bottom': null}));
      expect((r.end as Stopped).bottom, isNull);
    });

    test('rejected carries the typed reason and no frames', () {
      final r = SquatRangeResult.fromJson(squatGoldenJson()
        ..['end'] = {
          'kind': 'rejected',
          'rejection': {'reason': 'does_not_balance', 'unknown': 'trunk_angle'},
        }
        ..['frames'] = <dynamic>[]
        ..['assumptions'] = <dynamic>[]);
      final end = r.end as Rejected;
      expect(end.reason.reason, RejectionReason.doesNotBalance);
      expect(end.reason.limiting, isNull);
      expect(r.frames, isEmpty);
      expect(r.assumptions, isEmpty);
    });

    test('an unknown end kind is a FormatException', () {
      expect(() => SquatRangeResult.fromJson(_with({'kind': 'sideways'})),
          throwsFormatException);
    });
  });

  test('a bodyweight frame has no hip lever and no bar', () {
    final frame = Map<String, dynamic>.of(
        (squatGoldenJson()['frames'] as List).first as Map<String, dynamic>)
      ..remove('hip');
    (frame['posture'] as Map<String, dynamic>)['load'] = null;
    (frame['mechanics'] as Map<String, dynamic>)['load'] = {
      'basis': 'bodyweight_only'
    };
    final f =
        SquatRangeResult.fromJson(_with({'kind': 'bottom'}, frames: [frame]))
            .frames
            .single;
    expect(f.hip, isNull);
    expect(f.posture.load, isNull);
    expect(f.mechanics.barKg, isNull);
  });

  test('the request serializes with the wire names; nulls stay null', () {
    expect(sampleSquatRequest.toJson(), {
      'height_m': 1.78,
      'mass_kg': 77.0,
      'thigh_m': null,
      'shank_m': null,
      'trunk_m': null,
      'dorsiflexion_deg': 29.0,
      'bar_kg': 84.0,
      'frames': null,
    });
  });
}
