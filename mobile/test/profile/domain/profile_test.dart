// SAC1/SAC7 -> AC1/AC7 (data model): Profile.fromJson parses the
// ProfileResponse body (`backend/crates/api/src/profile/handlers.rs`). `age` is
// SERVER-derived and read from the response, never recomputed on device
// (SPEC-0008 §2.6); optional `sex`/`body_fat_percentage` may be null.
//
// RED until package:fitai/src/profile/domain/profile.dart defines
// Profile.fromJson.

import 'package:fitai/src/profile/domain/goal.dart';
import 'package:fitai/src/profile/domain/profile.dart';
import 'package:fitai/src/profile/domain/sex.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/profile_fakes.dart';

void main() {
  group('SAC7 Profile.fromJson', () {
    test('parses the full ProfileResponse body', () {
      final p = Profile.fromJson(profileResponseJson(
        userId: 'u-1',
        age: 35,
        heightCm: 180,
        weightKg: 82.5,
        sex: 'female',
        bodyFatPercentage: 18.0,
        goals: ['build_muscle', 'gain_strength'],
      ));

      expect(p.userId, 'u-1');
      expect(p.age, 35,
          reason: 'age is read from the response, server-derived');
      expect(p.heightCm, 180);
      expect(p.weightKg, 82.5);
      expect(p.sex, Sex.female);
      expect(p.bodyFatPercentage, 18.0);
      expect(p.goals, contains(Goal.buildMuscle));
      expect(p.goals, contains(Goal.gainStrength));
    });

    test('omitted optionals parse to null (sex, body_fat_percentage)', () {
      final p = Profile.fromJson(profileResponseJson());
      expect(p.sex, isNull);
      expect(p.bodyFatPercentage, isNull);
    });

    test('parses the goals list into the controlled Goal set', () {
      final p = Profile.fromJson(profileResponseJson(
        goals: ['lose_fat', 'maintain', 'recomp'],
      ));
      expect(
        p.goals,
        containsAll(<Goal>[Goal.loseFat, Goal.maintain, Goal.recomp]),
      );
    });
  });
}
