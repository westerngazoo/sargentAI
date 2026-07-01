// R-0032 (slice 1) — wire model for the R-0005 nutrition endpoints.

/// One day's macro log as returned by `POST /nutrition` / `GET /nutrition`.
class NutritionLog {
  const NutritionLog({
    required this.id,
    required this.performedOn,
    required this.proteinG,
    required this.carbsG,
    required this.fatG,
    required this.calories,
  });

  factory NutritionLog.fromJson(Map<String, dynamic> json) => NutritionLog(
        id: json['id'] as String,
        performedOn: json['performed_on'] as String,
        proteinG: (json['protein_g'] as num).toDouble(),
        carbsG: (json['carbs_g'] as num).toDouble(),
        fatG: (json['fat_g'] as num).toDouble(),
        calories: (json['calories'] as num).toDouble(),
      );

  final String id;

  /// ISO date (`yyyy-MM-dd`) the log applies to.
  final String performedOn;
  final double proteinG;
  final double carbsG;
  final double fatG;

  /// Derived server-side: 4·protein + 4·carbs + 9·fat.
  final double calories;
}
