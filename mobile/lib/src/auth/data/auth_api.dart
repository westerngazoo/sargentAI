import 'package:dio/dio.dart';

import '../../core/network/api_exception.dart';
import '../../core/storage/token_store.dart';

/// Typed client over the R-0002 auth endpoints (SPEC-0007 §3.2). Every
/// `DioException` is translated to an [ApiException] so no transport error ever
/// escapes the data layer.
class AuthApi {
  const AuthApi(this._dio);

  final Dio _dio;

  /// `POST /auth/register` → 201 `{user_id}` (no token). 400/409 → [ApiException].
  Future<void> register(String email, String password) async {
    try {
      await _dio.post<dynamic>(
        '/auth/register',
        data: <String, String>{'email': email, 'password': password},
      );
    } on DioException catch (e) {
      throw _toApiException(e);
    }
  }

  /// `POST /auth/login` → 200 `{token,user_id,expires_at}`. 401 → [ApiException].
  Future<AuthToken> login(String email, String password) async {
    try {
      final res = await _dio.post<dynamic>(
        '/auth/login',
        data: <String, String>{'email': email, 'password': password},
      );
      final body = res.data as Map<String, dynamic>;
      return AuthToken(
        jwt: body['token'] as String,
        userId: body['user_id'] as String,
        expiresAt: DateTime.parse(body['expires_at'] as String),
      );
    } on DioException catch (e) {
      throw _toApiException(e);
    }
  }

  /// `GET /auth/me` → 200 `{user_id}` (requires a Bearer token).
  Future<String> me() async {
    try {
      final res = await _dio.get<dynamic>('/auth/me');
      final body = res.data as Map<String, dynamic>;
      return body['user_id'] as String;
    } on DioException catch (e) {
      throw _toApiException(e);
    }
  }

  ApiException _toApiException(DioException e) {
    final response = e.response;
    if (response == null) {
      // No HTTP response → transport/timeout: the AC9 retryable case.
      return const ApiException("can't reach the server — retry");
    }
    final status = response.statusCode;
    final data = response.data;
    if (data is Map && data['error'] is Map) {
      final error = data['error'] as Map;
      return ApiException(
        (error['message'] as String?) ?? _defaultMessage(status),
        statusCode: status,
        field: error['field'] as String?,
      );
    }
    return ApiException(_defaultMessage(status), statusCode: status);
  }

  String _defaultMessage(int? status) => switch (status) {
        400 => 'please check your details',
        401 => 'invalid email or password',
        409 => 'that email is already registered',
        _ => 'something went wrong — please try again',
      };
}
