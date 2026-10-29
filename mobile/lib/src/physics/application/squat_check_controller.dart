// SPEC-0059 §2.3 — the squat check's inputs, the server's answer, and the
// playback between the frames it returned (display only, R-0059 AC7).

import 'dart:ui' show lerpDouble;

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/network/dio_provider.dart';
import '../../profile/application/profile_providers.dart';
import '../data/squat_check_api.dart';
import '../domain/squat_range.dart';

final squatCheckApiProvider =
    Provider<SquatCheckApi>((ref) => SquatCheckApi(ref.read(dioProvider)));

/// The lifter's ankle dorsiflexion, degrees. No default (SPEC-0045
/// §2.6.1.1): null until the lifter types it, then kept for the session
/// only, until R-0058 stores it.
final squatDorsiflexionProvider = StateProvider<double?>((_) => null);

/// The bar load chosen for the check, kg; null is bodyweight only.
final squatLoadKgProvider = StateProvider<double?>((_) => null);

final squatCheckControllerProvider =
    AsyncNotifierProvider<SquatCheckController, SquatRangeResult?>(
  SquatCheckController.new,
);

/// What playback draws at a [progress]: the [posture] interpolated between
/// the two returned frames around it, and the returned [frame] nearest to
/// it, whose numbers the check shows unchanged.
@immutable
class PlaybackFrame {
  const PlaybackFrame(this.progress, this.posture, this.frame);

  final double progress;
  final Posture posture;
  final RangeFrame frame;
}

/// Asks the server for the range once the inputs are known: height and mass
/// from the profile (`height_cm / 100`), limb lengths null until R-0058 (the
/// table fills them), [squatDorsiflexionProvider] and [squatLoadKgProvider].
///
/// The state is null while the dorsiflexion or the profile is missing, and an
/// error carrying the [ApiException] when the server refuses the request.
class SquatCheckController extends AsyncNotifier<SquatRangeResult?> {
  @override
  Future<SquatRangeResult?> build() async {
    final dorsiflexion = ref.watch(squatDorsiflexionProvider);
    final barKg = ref.watch(squatLoadKgProvider);
    final profile = await ref.watch(profileProvider.future);
    if (dorsiflexion == null || profile == null) return null;
    return ref.read(squatCheckApiProvider).range(SquatRangeRequest(
          heightM: profile.heightCm / 100,
          massKg: profile.weightKg,
          dorsiflexionDeg: dorsiflexion,
          barKg: barKg,
        ));
  }

  /// The playback frame at [progress], clamped to the returned frames: it
  /// never extrapolates, and at a returned frame's progress it is that
  /// frame exactly. Null when there are no frames to play.
  PlaybackFrame? frameAt(double progress) {
    final frames = state.valueOrNull?.frames ?? const [];
    if (frames.isEmpty) return null;
    final p = progress.clamp(frames.first.progress, frames.last.progress);
    var i = 0;
    while (i < frames.length - 1 && frames[i + 1].progress <= p) {
      i++;
    }
    final a = frames[i];
    if (a.progress == p) return PlaybackFrame(p, a.posture, a);
    final b = frames[i + 1];
    final t = (p - a.progress) / (b.progress - a.progress);
    return PlaybackFrame(
        p, _lerpPosture(a.posture, b.posture, t), t < 0.5 ? a : b);
  }
}

Point _lerpPoint(Point a, Point b, double t) =>
    Point(lerpDouble(a.x, b.x, t)!, lerpDouble(a.y, b.y, t)!);

Posture _lerpPosture(Posture a, Posture b, double t) {
  final (la, lb) = (a.load, b.load);
  return Posture(
    ankle: _lerpPoint(a.ankle, b.ankle, t),
    knee: _lerpPoint(a.knee, b.knee, t),
    hip: _lerpPoint(a.hip, b.hip, t),
    shoulder: _lerpPoint(a.shoulder, b.shoulder, t),
    load: la == null || lb == null
        ? null
        : PostureLoad(kg: la.kg, at: _lerpPoint(la.at, lb.at, t)),
  );
}
