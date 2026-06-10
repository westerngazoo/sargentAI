// SAC7/SAC11 -> AC7/AC11 (wire mapping): the client enums emit EXACTLY the
// tokens the R-0003 backend expects — Goal is snake_case, Sex is LOWERCASE
// (`male`/`female`), per `core::profile::{Goal,Sex}`. Emitting anything else
// would be a backend-contract break (AC11: no backend change), so these are
// guarded directly.
//
// RED until package:fitai/src/profile/domain/{goal.dart,sex.dart} define the
// enums with their wire tokens and round-trip parse.

import 'package:fitai/src/profile/domain/goal.dart';
import 'package:fitai/src/profile/domain/sex.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('SAC11 Goal wire tokens (snake_case, mirrors core::profile::Goal)', () {
    const expected = <Goal, String>{
      Goal.loseFat: 'lose_fat',
      Goal.buildMuscle: 'build_muscle',
      Goal.recomp: 'recomp',
      Goal.maintain: 'maintain',
      Goal.gainStrength: 'gain_strength',
    };

    test('the five goals serialize to their snake_case tokens', () {
      for (final entry in expected.entries) {
        expect(entry.key.wire, entry.value);
      }
    });

    test('every backend token parses back to its Goal (round-trip)', () {
      for (final entry in expected.entries) {
        expect(Goal.fromWire(entry.value), entry.key);
      }
    });

    test('the controlled set is exactly five goals', () {
      expect(Goal.values.length, 5);
    });
  });

  group('SAC7/SAC11 Sex wire tokens (LOWERCASE, NOT snake_case)', () {
    test('Sex.male serializes to "male" and Sex.female to "female"', () {
      expect(Sex.male.wire, 'male');
      expect(Sex.female.wire, 'female');
    });

    test('the backend lowercase tokens parse back to Sex (round-trip)', () {
      expect(Sex.fromWire('male'), Sex.male);
      expect(Sex.fromWire('female'), Sex.female);
    });

    test('the controlled set is exactly the two R-0003 values', () {
      expect(Sex.values.length, 2);
    });
  });
}
