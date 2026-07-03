// R-0034 — typed client over /measurements.

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../auth/application/auth_controller.dart';
import '../../core/network/api_exception.dart';
import '../../core/network/dio_provider.dart';
import '../models/measurement.dart';

class MeasurementService {
  const MeasurementService(this._dio);

  final Dio _dio;

  /// `GET /measurements` — oldest first, for charting.
  Future<List<Measurement>> list() async {
    try {
      final res = await _dio.get<List<dynamic>>('/measurements');
      return res.data!
          .map((e) => Measurement.fromJson(e as Map<String, dynamic>))
          .toList();
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }

  /// `POST /measurements` — upsert one row for the day.
  Future<Measurement> create({
    required String measuredOn,
    required double weightKg,
    double? bodyFatPercentage,
  }) async {
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/measurements',
        data: {
          'measured_on': measuredOn,
          'weight_kg': weightKg,
          if (bodyFatPercentage != null)
            'body_fat_percentage': bodyFatPercentage,
        },
      );
      return Measurement.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }
}

final measurementServiceProvider = Provider<MeasurementService>(
  (ref) => MeasurementService(ref.read(dioProvider)),
);

/// The caller's measurement history, oldest first. Drops on account switch.
final measurementsProvider = FutureProvider<List<Measurement>>((ref) {
  ref.watch(authUserIdProvider);
  return ref.read(measurementServiceProvider).list();
});
