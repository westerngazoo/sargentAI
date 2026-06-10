// SAC2 -> AC2 (wire mapping): the client MuscleGroup emits EXACTLY the six
// snake_case tokens `core::workout::MuscleGroup` accepts
// (chest/back/shoulders/arms/legs/core) — anything else is a backend-contract
// break (AC12: no backend change), so the tokens are pinned directly, with the
// round-trip parse (the Goal/Sex idiom from R-0008).
//
// RED until package:fitai/src/workout/domain/muscle_group.dart defines the
// enum with its wire tokens and round-trip parse.

import 'package:fitai/src/workout/domain/muscle_group.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('SAC2 MuscleGroup wire tokens (mirrors core::workout::MuscleGroup)',
      () {
    const expected = <MuscleGroup, String>{
      MuscleGroup.chest: 'chest',
      MuscleGroup.back: 'back',
      MuscleGroup.shoulders: 'shoulders',
      MuscleGroup.arms: 'arms',
      MuscleGroup.legs: 'legs',
      MuscleGroup.core: 'core',
    };

    test('the six groups serialize to their backend tokens', () {
      for (final entry in expected.entries) {
        expect(entry.key.wire, entry.value);
      }
    });

    test('every backend token parses back to its MuscleGroup (round-trip)', () {
      for (final entry in expected.entries) {
        expect(MuscleGroup.fromWire(entry.value), entry.key);
      }
    });

    test('the controlled set is exactly the six R-0004 groups', () {
      expect(MuscleGroup.values.length, 6);
    });
  });
}
