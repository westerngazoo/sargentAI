// R-0059 AC7 -> SPEC-0059 §2.3 (slice 59-b): playback interpolates postures
// only between the frames the server returned, shows a returned frame's
// numbers unchanged, and never extrapolates. Pure: no mocks.

import 'package:fitai/src/physics/domain/playback.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/physics_fakes.dart';

void main() {
  final frames = squatGolden().frames; // progress 0 and 1
  final [a, b] = frames;

  test('at a returned frame is that frame, exactly', () {
    for (final frame in frames) {
      final p = playbackAt(frames, frame.progress)!;
      expect(identical(p.frame, frame), isTrue);
      expect(identical(p.posture, frame.posture), isTrue);
      expect(p.progress, frame.progress);
    }
  });

  test('between two frames the posture lies between them', () {
    final p = playbackAt(frames, 0.25)!;
    expect(p.progress, 0.25);
    expect(
        p.posture.hip.y,
        closeTo(
            a.posture.hip.y + (b.posture.hip.y - a.posture.hip.y) / 4, 1e-12));
    expect(
        p.posture.load!.at.x,
        closeTo(
            a.posture.load!.at.x +
                (b.posture.load!.at.x - a.posture.load!.at.x) / 4,
            1e-12));
    expect(identical(p.frame, a), isTrue,
        reason: 'the numbers shown are the nearest returned frame');
    expect(identical(playbackAt(frames, 0.75)!.frame, b), isTrue);
  });

  test('never extrapolates past the returned frames', () {
    final late = playbackAt(frames, 1.5)!;
    expect(late.progress, b.progress);
    expect(identical(late.posture, b.posture), isTrue);
    final early = playbackAt(frames, -1)!;
    expect(early.progress, a.progress);
    expect(identical(early.posture, a.posture), isTrue);
  });

  test('a stopped range plays only up to its last frame', () {
    final p = playbackAt([a], 0.9)!;
    expect(identical(p.frame, a), isTrue);
  });

  test('no frames, nothing to play', () {
    expect(playbackAt(const [], 0.5), isNull);
  });
}
