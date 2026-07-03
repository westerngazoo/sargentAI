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
      throw ApiException.fromDio(e);
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
      throw ApiException.fromDio(e);
    }
  }

  /// `POST /auth/google` → 200 `{token,user_id,expires_at}`. 401 → [ApiException].
  Future<AuthToken> loginWithGoogle(String idToken) async {
    try {
      final res = await _dio.post<dynamic>(
        '/auth/google',
        data: <String, String>{'id_token': idToken},
      );
      final body = res.data as Map<String, dynamic>;
      return AuthToken(
        jwt: body['token'] as String,
        userId: body['user_id'] as String,
        expiresAt: DateTime.parse(body['expires_at'] as String),
      );
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }

  /// `GET /auth/me` → 200 `{user_id}` (requires a Bearer token).
  Future<String> me() async {
    try {
      final res = await _dio.get<dynamic>('/auth/me');
      final body = res.data as Map<String, dynamic>;
      return body['user_id'] as String;
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }
}
