// R-0034 — wire model for GET/POST /measurements. Lean mass is derived
// server-side (weight × (1 − bf%)).

class Measurement {
  const Measurement({
    required this.measuredOn,
    required this.weightKg,
    this.bodyFatPercentage,
    this.leanMassKg,
  });

  factory Measurement.fromJson(Map<String, dynamic> j) => Measurement(
        measuredOn: DateTime.parse(j['measured_on'] as String),
        weightKg: (j['weight_kg'] as num).toDouble(),
        bodyFatPercentage: (j['body_fat_percentage'] as num?)?.toDouble(),
        leanMassKg: (j['lean_mass_kg'] as num?)?.toDouble(),
      );

  final DateTime measuredOn;
  final double weightKg;
  final double? bodyFatPercentage;
  final double? leanMassKg;
}
