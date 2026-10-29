import 'package:dio/dio.dart';

import '../../core/network/api_exception.dart';
import '../domain/squat_range.dart';

/// Typed client over `POST /biomech/squat/range` (SPEC-0059 §2.2).
class SquatCheckApi {
  const SquatCheckApi(this._dio);

  final Dio _dio;

  /// The range of balanced postures for [request]. A `200` always parses,
  /// including a stopped or rejected range. Throws an [ApiException] on any
  /// transport or HTTP failure; a `400` names the offending `field`.
  Future<SquatRangeResult> range(SquatRangeRequest request) async {
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/biomech/squat/range',
        data: request.toJson(),
      );
      return SquatRangeResult.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }
}
