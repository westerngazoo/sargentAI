import 'package:flutter/foundation.dart';

import 'goal.dart';
import 'sex.dart';

/// A stored user profile, parsed from the `GET`/`PUT /profile/me`
/// `ProfileResponse` body (`backend/crates/api/src/profile/handlers.rs`).
///
/// `age` is **server-derived** — read from the response, never recomputed on
/// device (SPEC-0008 §2.6).
@immutable
class Profile {
  const Profile({
    required this.userId,
    required this.dateOfBirth,
    required this.age,
    required this.heightCm,
    required this.weightKg,
    required this.goals,
    this.sex,
    this.bodyFatPercentage,
  });

  final String userId;
  final DateTime dateOfBirth;
  final int age;
  final int heightCm;
  final double weightKg;
  final Set<Goal> goals;
  final Sex? sex;
  final double? bodyFatPercentage;

  factory Profile.fromJson(Map<String, dynamic> json) {
    final sex = json['sex'] as String?;
    return Profile(
      userId: json['user_id'] as String,
      dateOfBirth: DateTime.parse(json['date_of_birth'] as String),
      age: json['age'] as int,
      heightCm: json['height_cm'] as int,
      weightKg: (json['weight_kg'] as num).toDouble(),
      sex: sex == null ? null : Sex.fromWire(sex),
      bodyFatPercentage: (json['body_fat_percentage'] as num?)?.toDouble(),
      goals: (json['goals'] as List).cast<String>().map(Goal.fromWire).toSet(),
    );
  }
}
