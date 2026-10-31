// SPEC-0059 §2.3 — playback between the frames the server returned. Display
// only (R-0059 AC7): postures are interpolated for drawing, and the numbers
// shown are always a returned frame's, unchanged.

import 'dart:ui' show lerpDouble;

import 'package:flutter/foundation.dart';

import 'squat_range.dart';

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

/// The playback frame at [progress] over [frames], which are in increasing
/// `progress` order as the server returns them. [progress] is clamped to the
/// returned frames, so it never extrapolates; at a returned frame's progress
/// the result is that frame exactly. Null when [frames] is empty (a rejected
/// range has nothing to play).
PlaybackFrame? playbackAt(List<RangeFrame> frames, double progress) {
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
