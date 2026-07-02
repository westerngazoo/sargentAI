// R-0032 (slice 1) — typed HTTP client over the R-0005 nutrition endpoints.

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/network/api_exception.dart';
import '../../core/network/dio_provider.dart';
import '../models/food_info.dart';
import '../models/nutrition_log.dart';

/// Thin client over `POST /nutrition` and `GET /nutrition`. Transport and
/// HTTP failures surface as [ApiException] — raw [DioException]s never
/// escape, matching the `ProgramService` contract.
class NutritionService {
  const NutritionService(this._dio);

  final Dio _dio;

  /// `POST /nutrition` — log macros for a day; returns the persisted row.
  Future<NutritionLog> create({
    required String performedOn,
    required double proteinG,
    required double carbsG,
    required double fatG,
  }) async {
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/nutrition',
        data: {
          'performed_on': performedOn,
          'protein_g': proteinG,
          'carbs_g': carbsG,
          'fat_g': fatG,
        },
      );
      return NutritionLog.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }

  /// `GET /nutrition/foods?q=…` — USDA nutrient lookup, macros per 100 g.
  Future<List<FoodInfo>> searchFoods(String query) async {
    try {
      final res = await _dio.get<Map<String, dynamic>>(
        '/nutrition/foods',
        queryParameters: {'q': query},
      );
      return (res.data!['foods'] as List<dynamic>)
          .map((e) => FoodInfo.fromJson(e as Map<String, dynamic>))
          .toList();
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }

  /// `GET /nutrition` — all of the caller's logs, newest first.
  Future<List<NutritionLog>> list() async {
    try {
      final res = await _dio.get<List<dynamic>>('/nutrition');
      return res.data!
          .map((e) => NutritionLog.fromJson(e as Map<String, dynamic>))
          .toList();
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }
}

final nutritionServiceProvider = Provider<NutritionService>(
  (ref) => NutritionService(ref.read(dioProvider)),
);
