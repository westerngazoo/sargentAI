// SPEC-0059 §2.2/§2.3 — the `POST /biomech/squat/range` wire types, parsed
// once. The server computes every number; nothing here derives one.

import 'package:flutter/foundation.dart';

/// The request body. Lengths and the bar are optional: a null length comes
/// from the anthropometric table (the result lists it in `tableLengths`), a
/// null bar is bodyweight only, a null [frames] is the server's default.
@immutable
class SquatRangeRequest {
  const SquatRangeRequest({
    required this.heightM,
    required this.massKg,
    required this.dorsiflexionDeg,
    this.thighM,
    this.shankM,
    this.trunkM,
    this.barKg,
    this.frames,
  });

  final double heightM;
  final double massKg;
  final double? thighM;
  final double? shankM;
  final double? trunkM;
  final double dorsiflexionDeg;
  final double? barKg;
  final int? frames;

  Map<String, dynamic> toJson() => {
        'height_m': heightM,
        'mass_kg': massKg,
        'thigh_m': thighM,
        'shank_m': shankM,
        'trunk_m': trunkM,
        'dorsiflexion_deg': dorsiflexionDeg,
        'bar_kg': barKg,
        'frames': frames,
      };
}

double _d(Object? v) => (v as num).toDouble();

/// A joint of the sagittal model. Throws a [StateError] on an unknown token,
/// like every closed vocabulary on the wire.
enum Joint {
  hip,
  knee,
  ankle,
  lumbar;

  static Joint fromWire(String token) =>
      values.firstWhere((j) => j.name == token);
}

/// A point in metres: `x` forward, `y` up, origin at the mid-foot.
@immutable
class Point {
  const Point(this.x, this.y);

  factory Point.fromJson(Map<String, dynamic> json) =>
      Point(_d(json['x']), _d(json['y']));

  final double x;
  final double y;
}

/// The bar: its mass and where it sits.
@immutable
class PostureLoad {
  const PostureLoad({required this.kg, required this.at});

  factory PostureLoad.fromJson(Map<String, dynamic> json) => PostureLoad(
      kg: _d(json['kg']),
      at: Point.fromJson(json['at'] as Map<String, dynamic>));

  final double kg;
  final Point at;
}

/// The figure's joint centres; [load] is null for bodyweight only.
@immutable
class Posture {
  const Posture({
    required this.ankle,
    required this.knee,
    required this.hip,
    required this.shoulder,
    this.load,
  });

  factory Posture.fromJson(Map<String, dynamic> json) {
    Point p(String k) => Point.fromJson(json[k] as Map<String, dynamic>);
    final load = json['load'] as Map<String, dynamic>?;
    return Posture(
      ankle: p('ankle'),
      knee: p('knee'),
      hip: p('hip'),
      shoulder: p('shoulder'),
      load: load == null ? null : PostureLoad.fromJson(load),
    );
  }

  final Point ankle;
  final Point knee;
  final Point hip;
  final Point shoulder;
  final PostureLoad? load;
}

/// A stated modelling assumption: a token, and the posture it applies `at`
/// when it names one. Kept as tokens so a new one never breaks the parse.
@immutable
class Assumption {
  const Assumption(this.assumption, {this.at});

  factory Assumption.fromJson(Map<String, dynamic> json) =>
      Assumption(json['assumption'] as String, at: json['at'] as String?);

  final String assumption;
  final String? at;
}

List<Assumption> _assumptions(Object? list) => (list as List<dynamic>)
    .map((a) => Assumption.fromJson(a as Map<String, dynamic>))
    .toList();

/// One joint's net moment (N·m) and the muscle group resisting it.
@immutable
class JointMoment {
  const JointMoment({
    required this.joint,
    required this.resisting,
    required this.moment,
    required this.uncertainty,
  });

  factory JointMoment.fromJson(Map<String, dynamic> json) => JointMoment(
        joint: Joint.fromWire(json['joint'] as String),
        resisting: json['resisting'] as String,
        moment: _d(json['moment']),
        uncertainty: _d(json['uncertainty']),
      );

  final Joint joint;
  final String resisting;
  final double moment;
  final double uncertainty;
}

/// A frame's mechanics: the four moments, the load basis, the assumptions.
@immutable
class Mechanics {
  const Mechanics({
    required this.joints,
    required this.barKg,
    required this.assumptions,
  });

  factory Mechanics.fromJson(Map<String, dynamic> json) => Mechanics(
        joints: (json['joints'] as List<dynamic>)
            .map((j) => JointMoment.fromJson(j as Map<String, dynamic>))
            .toList(),
        barKg:
            ((json['load'] as Map<String, dynamic>)['kg'] as num?)?.toDouble(),
        assumptions: _assumptions(json['assumptions']),
      );

  final List<JointMoment> joints;

  /// The bar's mass, or null when the basis is bodyweight only.
  final double? barKg;
  final List<Assumption> assumptions;

  /// The server's moment at [joint]. Throws a [StateError] if the frame
  /// carries none, which the server never sends.
  double moment(Joint joint) =>
      joints.firstWhere((j) => j.joint == joint).moment;
}

/// The hip lever the check shows, computed on the server (SPEC-0059 §2.2):
/// `barNm = barN × leverM`, and `barNm + bodyNm` is the hip moment.
@immutable
class HipLever {
  const HipLever({
    required this.leverM,
    required this.barN,
    required this.barNm,
    required this.bodyNm,
  });

  factory HipLever.fromJson(Map<String, dynamic> json) => HipLever(
        leverM: _d(json['lever_m']),
        barN: _d(json['bar_n']),
        barNm: _d(json['bar_nm']),
        bodyNm: _d(json['body_nm']),
      );

  final double leverM;
  final double barN;
  final double barNm;
  final double bodyNm;
}

/// One balanced posture of the range.
@immutable
class RangeFrame {
  const RangeFrame({
    required this.progress,
    required this.shankDeg,
    required this.kneeFlexionDeg,
    required this.hipFlexionDeg,
    required this.trunkLeanDeg,
    required this.posture,
    required this.mechanics,
    required this.groundN,
    this.hip,
  });

  factory RangeFrame.fromJson(Map<String, dynamic> json) {
    final hip = json['hip'] as Map<String, dynamic>?;
    return RangeFrame(
      progress: _d(json['progress']),
      shankDeg: _d(json['shank_deg']),
      kneeFlexionDeg: _d(json['knee_flexion_deg']),
      hipFlexionDeg: _d(json['hip_flexion_deg']),
      trunkLeanDeg: _d(json['trunk_lean_deg']),
      posture: Posture.fromJson(json['posture'] as Map<String, dynamic>),
      mechanics: Mechanics.fromJson(json['mechanics'] as Map<String, dynamic>),
      groundN: _d(json['ground_n']),
      hip: hip == null ? null : HipLever.fromJson(hip),
    );
  }

  /// `0` upright to `1` at the bottom.
  final double progress;
  final double shankDeg;
  final double kneeFlexionDeg;
  final double hipFlexionDeg;
  final double trunkLeanDeg;
  final Posture posture;
  final Mechanics mechanics;

  /// The ground's reaction, N.
  final double groundN;

  /// Null for a bodyweight-only frame.
  final HipLever? hip;
}

/// Why the model cannot place a posture (core's `Rejection`).
enum RejectionReason {
  doesNotBalance('does_not_balance'),
  anatomicallyImplausible('anatomically_implausible'),
  depthNotReachable('depth_not_reachable');

  const RejectionReason(this.wire);

  final String wire;

  static RejectionReason fromWire(String token) =>
      values.firstWhere((r) => r.wire == token);
}

/// A typed rejection. [limiting], [neededDeg] and [availableDeg] are set for
/// [RejectionReason.depthNotReachable] only.
@immutable
class Rejection {
  const Rejection({
    required this.reason,
    this.limiting,
    this.neededDeg,
    this.availableDeg,
  });

  factory Rejection.fromJson(Map<String, dynamic> json) {
    final limiting = json['limiting'] as String?;
    return Rejection(
      reason: RejectionReason.fromWire(json['reason'] as String),
      limiting: limiting == null ? null : Joint.fromWire(limiting),
      neededDeg: (json['needed_deg'] as num?)?.toDouble(),
      availableDeg: (json['available_deg'] as num?)?.toDouble(),
    );
  }

  final RejectionReason reason;
  final Joint? limiting;
  final double? neededDeg;
  final double? availableDeg;
}

/// How the range ended (the api-owned `End`, tagged by `kind`).
sealed class End {
  const End();

  /// Throws a [FormatException] on an unknown `kind`.
  factory End.fromJson(Map<String, dynamic> json) {
    Rejection? rejection(Object? v) =>
        v == null ? null : Rejection.fromJson(v as Map<String, dynamic>);
    return switch (json['kind'] as String) {
      'bottom' => const Bottom(),
      'stopped' => Stopped(_d(json['reached']), rejection(json['bottom'])),
      'rejected' => Rejected(rejection(json['rejection'])!),
      final other => throw FormatException('unknown range end: $other'),
    };
  }
}

/// The lifter reached the bottom.
final class Bottom extends End {
  const Bottom();
}

/// The lifter stopped at [reached]; [bottom] says why the bottom itself
/// fails, or is null when only the model's convention stops the range.
final class Stopped extends End {
  const Stopped(this.reached, this.bottom);

  final double reached;
  final Rejection? bottom;
}

/// The model could not place the lifter at all; the result has no frames.
final class Rejected extends End {
  const Rejected(this.reason);

  final Rejection reason;
}

/// The `200` body of `POST /biomech/squat/range`.
@immutable
class SquatRangeResult {
  const SquatRangeResult({
    required this.end,
    required this.tableLengths,
    required this.frames,
    required this.assumptions,
  });

  /// Throws a [FormatException] on an unknown `end.kind`, a [StateError] on
  /// an unknown joint or rejection token, and a [TypeError] on a missing
  /// field, as every wire model here does.
  factory SquatRangeResult.fromJson(Map<String, dynamic> json) =>
      SquatRangeResult(
        end: End.fromJson(json['end'] as Map<String, dynamic>),
        tableLengths: (json['table_lengths'] as List<dynamic>).cast<String>(),
        frames: (json['frames'] as List<dynamic>)
            .map((f) => RangeFrame.fromJson(f as Map<String, dynamic>))
            .toList(),
        assumptions: _assumptions(json['assumptions']),
      );

  final End end;

  /// Which of `thigh`, `shank`, `trunk` came from the table.
  final List<String> tableLengths;
  final List<RangeFrame> frames;

  /// The range-level assumptions.
  final List<Assumption> assumptions;
}
