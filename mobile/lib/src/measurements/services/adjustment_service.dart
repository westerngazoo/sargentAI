// R-0017 — typed client over /adjustments.

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../auth/application/auth_controller.dart';
import '../../core/network/api_exception.dart';
import '../../core/network/dio_provider.dart';
import '../models/coach_suggestion.dart';

class AdjustmentService {
  const AdjustmentService(this._dio);

  final Dio _dio;

  /// `GET /adjustments` — the coach's current suggestions.
  Future<CoachAdvice> fetch() async {
    try {
      final res = await _dio.get<Map<String, dynamic>>('/adjustments');
      return CoachAdvice.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }
}

final adjustmentServiceProvider = Provider<AdjustmentService>(
  (ref) => AdjustmentService(ref.read(dioProvider)),
);

/// The caller's coach advice. Drops on account switch.
final coachAdviceProvider = FutureProvider<CoachAdvice>((ref) {
  ref.watch(authUserIdProvider);
  return ref.read(adjustmentServiceProvider).fetch();
});
