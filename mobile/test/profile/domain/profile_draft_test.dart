// SAC4/SAC5/SAC6/SAC7 -> AC4/AC5/AC6/AC7 (the validation core):
//   * dobError(today)   — age in [13, 120]; future DOB rejected
//   * heightError()     — [50, 300]
//   * weightError()     — [20.0, 500.0]
//   * bodyFatError()    — null when absent (skippable); [1.0, 75.0] when present
//   * bodyStatsValid    — all three required fields ok
//   * goalsValid        — goals.isNotEmpty (>=1 required)
//   * toRequest()       — TOTAL: ProfileRequest? (null until required fields
//     validate, no precondition throw — architect finding 4); skipped optionals
//     are OMITTED from the JSON (SAC7).
//
// Ranges mirror `backend/crates/core/src/profile.rs` EXACTLY (age 13-120,
// height 50-300, weight 20-500, body-fat 1-75); these boundary cases are the
// client mirror of the backend's source-of-truth validators.
//
// RED until package:fitai/src/profile/domain/profile_draft.dart defines
// ProfileDraft (immutable + copyWith + validators) and ProfileRequest.

import 'package:fitai/src/profile/domain/goal.dart';
import 'package:fitai/src/profile/domain/profile_draft.dart';
import 'package:fitai/src/profile/domain/sex.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  // A fixed "today" makes the age-from-DOB boundaries deterministic.
  final today = DateTime(2026, 6, 6);

  // DOB exactly N years before `today`.
  DateTime dobForAge(int years) =>
      DateTime(today.year - years, today.month, today.day);

  ProfileDraft validBodyStats() => ProfileDraft(
        dateOfBirth: dobForAge(30),
        heightCm: 180,
        weightKg: 82.5,
      );

  group('SAC4 dobError — age in [13, 120], future rejected', () {
    test('a valid adult DOB passes (null)', () {
      expect(validBodyStats().dobError(today), isNull);
    });

    test('exactly 13 (lower bound) passes', () {
      final d = ProfileDraft(dateOfBirth: dobForAge(13));
      expect(d.dobError(today), isNull);
    });

    test('age 12 (just under the floor) is rejected with a message', () {
      final d = ProfileDraft(dateOfBirth: dobForAge(12));
      expect(d.dobError(today), isNotNull);
    });

    test('exactly 120 (upper bound) passes', () {
      final d = ProfileDraft(dateOfBirth: dobForAge(120));
      expect(d.dobError(today), isNull);
    });

    test('age 121 (just over the ceiling) is rejected', () {
      final d = ProfileDraft(dateOfBirth: dobForAge(121));
      expect(d.dobError(today), isNotNull);
    });

    test('a future DOB is rejected', () {
      final d = ProfileDraft(dateOfBirth: today.add(const Duration(days: 1)));
      expect(d.dobError(today), isNotNull);
    });

    test('an empty/absent DOB is rejected (required field)', () {
      const d = ProfileDraft();
      expect(d.dobError(DateTime(2026, 6, 6)), isNotNull);
    });
  });

  group('SAC4 heightError — [50, 300]', () {
    test('180 (in range) passes', () {
      expect(validBodyStats().heightError(), isNull);
    });

    test('50 and 300 (bounds) pass', () {
      expect(const ProfileDraft(heightCm: 50).heightError(), isNull);
      expect(const ProfileDraft(heightCm: 300).heightError(), isNull);
    });

    test('49 and 301 (just out of range) are rejected', () {
      expect(const ProfileDraft(heightCm: 49).heightError(), isNotNull);
      expect(const ProfileDraft(heightCm: 301).heightError(), isNotNull);
    });

    test('an absent height is rejected (required field)', () {
      expect(const ProfileDraft().heightError(), isNotNull);
    });
  });

  group('SAC4 weightError — [20.0, 500.0]', () {
    test('82.5 (in range) passes', () {
      expect(validBodyStats().weightError(), isNull);
    });

    test('20.0 and 500.0 (bounds) pass', () {
      expect(const ProfileDraft(weightKg: 20.0).weightError(), isNull);
      expect(const ProfileDraft(weightKg: 500.0).weightError(), isNull);
    });

    test('19.9 and 500.1 (just out of range) are rejected', () {
      expect(const ProfileDraft(weightKg: 19.9).weightError(), isNotNull);
      expect(const ProfileDraft(weightKg: 500.1).weightError(), isNotNull);
    });

    test('an absent weight is rejected (required field)', () {
      expect(const ProfileDraft().weightError(), isNotNull);
    });
  });

  group('SAC4 bodyStatsValid — all three required fields ok', () {
    test('true when DOB, height, and weight all validate', () {
      expect(validBodyStats().bodyStatsValidOn(today), isTrue);
    });

    test('false when any required field is missing or invalid', () {
      expect(const ProfileDraft().bodyStatsValidOn(today), isFalse);
      expect(
        validBodyStats().copyWith(heightCm: 1000).bodyStatsValidOn(today),
        isFalse,
      );
    });
  });

  group('SAC5 goalsValid — >=1 required', () {
    test('false with zero goals', () {
      expect(const ProfileDraft().goalsValid, isFalse);
    });

    test('true with one goal', () {
      const d = ProfileDraft(goals: {Goal.buildMuscle});
      expect(d.goalsValid, isTrue);
    });

    test('true with several goals', () {
      const d = ProfileDraft(goals: {Goal.loseFat, Goal.gainStrength});
      expect(d.goalsValid, isTrue);
    });
  });

  group('SAC6 bodyFatError — skippable; [1.0, 75.0] when present', () {
    test('absent body-fat is allowed (null = ok, not an error)', () {
      expect(const ProfileDraft().bodyFatError(), isNull);
    });

    test('1.0 and 75.0 (bounds) pass', () {
      expect(const ProfileDraft(bodyFatPercentage: 1.0).bodyFatError(), isNull);
      expect(
          const ProfileDraft(bodyFatPercentage: 75.0).bodyFatError(), isNull);
    });

    test('0.9 and 75.1 (just out of range) are rejected', () {
      expect(
        const ProfileDraft(bodyFatPercentage: 0.9).bodyFatError(),
        isNotNull,
      );
      expect(
        const ProfileDraft(bodyFatPercentage: 75.1).bodyFatError(),
        isNotNull,
      );
    });
  });

  group('SAC3 copyWith preserves untouched fields (draft survives edits)', () {
    test('overriding one field keeps the rest', () {
      final base = validBodyStats().copyWith(goals: {Goal.recomp});
      final next = base.copyWith(weightKg: 90.0);
      expect(next.weightKg, 90.0);
      expect(next.heightCm, base.heightCm);
      expect(next.dateOfBirth, base.dateOfBirth);
      expect(next.goals, base.goals);
    });
  });

  group('SAC7 toRequest — total, omits skipped optionals', () {
    test('returns null while required fields are invalid (no throw)', () {
      // Missing body stats and goals.
      expect(const ProfileDraft().toRequest(today), isNull);
    });

    test('returns null when body stats are valid but no goal is chosen', () {
      expect(validBodyStats().toRequest(today), isNull);
    });

    test('builds a request once required fields validate', () {
      final draft = validBodyStats().copyWith(goals: {Goal.buildMuscle});
      final req = draft.toRequest(today);
      expect(req, isNotNull);
    });

    test('JSON omits sex and body_fat_percentage when skipped', () {
      final draft = validBodyStats().copyWith(goals: {Goal.buildMuscle});
      final json = draft.toRequest(today)!.toJson();
      expect(json.containsKey('sex'), isFalse,
          reason: 'skipped optional omitted, not null');
      expect(json.containsKey('body_fat_percentage'), isFalse);
      // Required fields are present with the backend keys.
      expect(json['date_of_birth'], '1996-06-06');
      expect(json['height_cm'], 180);
      expect(json['weight_kg'], 82.5);
      expect(json['goals'], contains('build_muscle'));
    });

    test('JSON includes optionals with LOWERCASE sex token when present', () {
      final draft = validBodyStats().copyWith(
        goals: {Goal.buildMuscle},
        sex: Sex.female,
        bodyFatPercentage: 22.0,
      );
      final json = draft.toRequest(today)!.toJson();
      expect(json['sex'], 'female', reason: 'lowercase, not snake_case');
      expect(json['body_fat_percentage'], 22.0);
    });

    test('a present-but-invalid body-fat blocks the request (returns null)',
        () {
      final draft = validBodyStats().copyWith(
        goals: {Goal.buildMuscle},
        bodyFatPercentage: 90.0, // out of [1,75]
      );
      expect(draft.toRequest(today), isNull);
    });

    test('serializes date_of_birth as YYYY-MM-DD (ISO NaiveDate)', () {
      final draft = ProfileDraft(
        dateOfBirth: DateTime(2001, 2, 3),
        heightCm: 175,
        weightKg: 70.0,
        goals: const {Goal.maintain},
      );
      final json = draft.toRequest(today)!.toJson();
      expect(json['date_of_birth'], '2001-02-03');
    });
  });
}
