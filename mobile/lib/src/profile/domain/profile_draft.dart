import 'package:flutter/foundation.dart';

import 'goal.dart';
import 'profile_field_error.dart';
import 'sex.dart';

/// Validation ranges. They mirror `backend/crates/core/src/profile.rs`
/// EXACTLY (AC4/AC6); the backend stays the source of truth — these only fail
/// the UI fast. Public so the field-error messages can quote them.
abstract final class ProfileLimits {
  static const int minAge = 13;
  static const int maxAge = 120;
  static const int minHeight = 50;
  static const int maxHeight = 300;
  static const double minWeight = 20.0;
  static const double maxWeight = 500.0;
  static const double minBodyFat = 1.0;
  static const double maxBodyFat = 75.0;
}

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
/// when valid or the [ProfileFieldError] kind otherwise.
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

  ProfileFieldError? dobError(DateTime today) {
    final dob = dateOfBirth;
    if (dob == null) return ProfileFieldError.dobMissing;
    if (dob.isAfter(today)) return ProfileFieldError.dobInFuture;
    final age = _ageOn(dob, today);
    if (age < ProfileLimits.minAge || age > ProfileLimits.maxAge) {
      return ProfileFieldError.ageOutOfRange;
    }
    return null;
  }

  ProfileFieldError? heightError() {
    final h = heightCm;
    if (h == null) return ProfileFieldError.heightMissing;
    if (h < ProfileLimits.minHeight || h > ProfileLimits.maxHeight) {
      return ProfileFieldError.heightOutOfRange;
    }
    return null;
  }

  ProfileFieldError? weightError() {
    final w = weightKg;
    if (w == null) return ProfileFieldError.weightMissing;
    if (w < ProfileLimits.minWeight || w > ProfileLimits.maxWeight) {
      return ProfileFieldError.weightOutOfRange;
    }
    return null;
  }

  /// `null` when absent (skippable, AC6) or in range; the kind otherwise.
  ProfileFieldError? bodyFatError() {
    final b = bodyFatPercentage;
    if (b == null) return null;
    if (b < ProfileLimits.minBodyFat || b > ProfileLimits.maxBodyFat) {
      return ProfileFieldError.bodyFatOutOfRange;
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
