// Radio-protocol units: "over" terminates, "out" signs off, and neither
// false-positives on ordinary words.

import 'package:fitai/src/hub/voice_protocol.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('endsWithOver', () {
    test('detects a trailing over (with punctuation and case)', () {
      expect(endsWithOver('10 reps at 100 kilos over'), isTrue);
      expect(endsWithOver('10 reps, Over.'), isTrue);
      expect(endsWithOver('over'), isTrue);
    });

    test('does not fire mid-sentence or inside words', () {
      expect(endsWithOver('over 100 kilos'), isFalse);
      expect(endsWithOver('moreover'), isFalse);
      expect(endsWithOver('10 reps'), isFalse);
    });
  });

  group('stripOver', () {
    test('removes the terminator, keeps the command', () {
      expect(stripOver('10 reps at 100 kilos, over.'), '10 reps at 100 kilos');
      expect(stripOver('log a meal over'), 'log a meal');
      expect(stripOver('over'), '');
    });

    test('is a no-op without a terminator', () {
      expect(stripOver('10 reps at 100 kilos'), '10 reps at 100 kilos');
    });
  });

  group('isOut', () {
    test('"out" and "over and out" sign off', () {
      expect(isOut('out'), isTrue);
      expect(isOut('over and out'), isTrue);
      expect(isOut('roger out'), isTrue);
    });

    test('"workout" never signs off', () {
      expect(isOut('start workout'), isFalse);
      expect(isOut('finish workout'), isFalse);
    });
  });
}
