import 'package:flutter/foundation.dart';

import 'goal.dart';
import 'sex.dart';

// Validation ranges mirror `backend/crates/core/src/profile.rs` EXACTLY (AC4/AC6);
// the backend stays the source of truth — these only fail the UI fast.
const int _minAge = 13;
const int _maxAge = 120;
const int _minHeight = 50;
const int _maxHeight = 300;
const double _minWeight = 20.0;
const double _maxWeight = 500.0;
const double _minBodyFat = 1.0;
const double _maxBodyFat = 75.0;

/// The validated `PUT /profile/me` payload. Built only from a draft whose
/// required fields validate; skipped optionals are **omitted** from the JSON.
@immutable
class ProfileRequest {
  const ProfileRequest({
    required this.dateOfBirth,
    required this.heightCm,
    required this.weightKg,
    required this.goals,
    this.sex,
    this.bodyFatPercentage,
  });

  final DateTime dateOfBirth;
  final int heightCm;
  final double weightKg;
  final Set<Goal> goals;
  final Sex? sex;
  final double? bodyFatPercentage;

  Map<String, dynamic> toJson() => <String, dynamic>{
        'date_of_birth': _isoDate(dateOfBirth),
        'height_cm': heightCm,
        'weight_kg': weightKg,
        'goals': goals.map((g) => g.wire).toList(),
        if (sex != null) 'sex': sex!.wire,
        if (bodyFatPercentage != null) 'body_fat_percentage': bodyFatPercentage,
      };
}

/// The in-progress onboarding input. Immutable with [copyWith] so it survives
/// step navigation; per-field validators mirror the backend and return `null`
/// when valid or a user-safe message otherwise.
@immutable
class ProfileDraft {
  const ProfileDraft({
    this.dateOfBirth,
    this.heightCm,
    this.weightKg,
    this.goals = const {},
    this.sex,
    this.bodyFatPercentage,
  });

  final DateTime? dateOfBirth;
  final int? heightCm;
  final double? weightKg;
  final Set<Goal> goals;
  final Sex? sex;
  final double? bodyFatPercentage;

  ProfileDraft copyWith({
    DateTime? dateOfBirth,
    int? heightCm,
    double? weightKg,
    Set<Goal>? goals,
    Sex? sex,
    double? bodyFatPercentage,
    bool clearSex = false,
    bool clearBodyFat = false,
  }) =>
      ProfileDraft(
        dateOfBirth: dateOfBirth ?? this.dateOfBirth,
        heightCm: heightCm ?? this.heightCm,
        weightKg: weightKg ?? this.weightKg,
        goals: goals ?? this.goals,
        sex: clearSex ? null : (sex ?? this.sex),
        bodyFatPercentage:
            clearBodyFat ? null : (bodyFatPercentage ?? this.bodyFatPercentage),
      );

  String? dobError(DateTime today) {
    final dob = dateOfBirth;
    if (dob == null) return 'enter your date of birth';
    if (dob.isAfter(today)) return 'date of birth cannot be in the future';
    final age = _ageOn(dob, today);
    if (age < _minAge || age > _maxAge) {
      return 'age must be between $_minAge and $_maxAge';
    }
    return null;
  }

  String? heightError() {
    final h = heightCm;
    if (h == null) return 'enter your height';
    if (h < _minHeight || h > _maxHeight) {
      return 'height must be between $_minHeight and $_maxHeight cm';
    }
    return null;
  }

  String? weightError() {
    final w = weightKg;
    if (w == null) return 'enter your weight';
    if (w < _minWeight || w > _maxWeight) {
      return 'weight must be between ${_minWeight.toStringAsFixed(0)} '
          'and ${_maxWeight.toStringAsFixed(0)} kg';
    }
    return null;
  }

  /// `null` when absent (skippable, AC6) or in range; a message otherwise.
  String? bodyFatError() {
    final b = bodyFatPercentage;
    if (b == null) return null;
    if (b < _minBodyFat || b > _maxBodyFat) {
      return 'body fat must be between $_minBodyFat and $_maxBodyFat %';
    }
    return null;
  }

  bool bodyStatsValidOn(DateTime today) =>
      dobError(today) == null && heightError() == null && weightError() == null;

  bool get goalsValid => goals.isNotEmpty;

  /// Total — the validated request, or `null` when the required fields don't
  /// validate or a present optional is out of range (no precondition throw).
  ProfileRequest? toRequest(DateTime today) {
    if (!bodyStatsValidOn(today) || !goalsValid || bodyFatError() != null) {
      return null;
    }
    return ProfileRequest(
      dateOfBirth: dateOfBirth!,
      heightCm: heightCm!,
      weightKg: weightKg!,
      goals: goals,
      sex: sex,
      bodyFatPercentage: bodyFatPercentage,
    );
  }

  @override
  bool operator ==(Object other) =>
      other is ProfileDraft &&
      other.dateOfBirth == dateOfBirth &&
      other.heightCm == heightCm &&
      other.weightKg == weightKg &&
      setEquals(other.goals, goals) &&
      other.sex == sex &&
      other.bodyFatPercentage == bodyFatPercentage;

  @override
  int get hashCode => Object.hash(
        dateOfBirth,
        heightCm,
        weightKg,
        Object.hashAllUnordered(goals),
        sex,
        bodyFatPercentage,
      );
}

int _ageOn(DateTime dob, DateTime today) {
  var age = today.year - dob.year;
  if (today.month < dob.month ||
      (today.month == dob.month && today.day < dob.day)) {
    age--;
  }
  return age;
}

String _isoDate(DateTime d) => '${d.year.toString().padLeft(4, '0')}-'
    '${d.month.toString().padLeft(2, '0')}-'
    '${d.day.toString().padLeft(2, '0')}';
