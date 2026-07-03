// R-0032 — wire model for `GET /nutrition/foods` (USDA lookup).

/// One food with macros per 100 g.
class FoodInfo {
  const FoodInfo({
    required this.name,
    required this.proteinGPer100g,
    required this.carbsGPer100g,
    required this.fatGPer100g,
    required this.kcalPer100g,
  });

  factory FoodInfo.fromJson(Map<String, dynamic> json) => FoodInfo(
        name: json['name'] as String,
        proteinGPer100g: (json['protein_g_per_100g'] as num).toDouble(),
        carbsGPer100g: (json['carbs_g_per_100g'] as num).toDouble(),
        fatGPer100g: (json['fat_g_per_100g'] as num).toDouble(),
        kcalPer100g: (json['kcal_per_100g'] as num).toDouble(),
      );

  final String name;
  final double proteinGPer100g;
  final double carbsGPer100g;
  final double fatGPer100g;
  final double kcalPer100g;

  /// Some catalog entries carry no nutrient data — skip those when picking.
  bool get hasData => kcalPer100g > 0 || proteinGPer100g > 0;
}
