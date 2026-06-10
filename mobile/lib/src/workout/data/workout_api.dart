import 'package:dio/dio.dart';

import '../../core/network/api_exception.dart';
import '../domain/session_draft.dart';
import '../domain/workout_session.dart';

/// Typed client over `/workouts` (R-0004). Every transport/HTTP failure goes
/// through the shared [ApiException.fromDio] (flat backend body) — a raw
/// `DioException` never escapes the data layer.
class WorkoutApi {
  const WorkoutApi(this._dio);

  final Dio _dio;

  /// `GET /workouts` → sessions in server order (newest first; no client sort).
  Future<List<WorkoutSession>> list() async {
    try {
      final res = await _dio.get<List<dynamic>>('/workouts');
      return (res.data ?? const <dynamic>[])
          .map((e) => WorkoutSession.fromJson(e as Map<String, dynamic>))
          .toList();
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }

  Future<WorkoutSession> create(SessionRequest req) async {
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/workouts',
        data: req.toJson(),
      );
      return WorkoutSession.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e); // carries statusCode + field for AC6
    }
  }

  Future<void> delete(String id) async {
    try {
      await _dio.delete<void>('/workouts/$id');
    } on DioException catch (e) {
      throw ApiException.fromDio(e); // 404 → ApiException(404)
    }
  }
}
