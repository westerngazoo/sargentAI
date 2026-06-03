import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../auth/application/auth_controller.dart';
import '../config/app_config.dart';

/// The shared HTTP client, pointed at [AppConfig.apiBaseUrl] (AC7) with the
/// [AuthInterceptor] installed (AC4/AC5).
final dioProvider = Provider<Dio>((ref) {
  final dio = Dio(
    BaseOptions(
      baseUrl: AppConfig.apiBaseUrl,
      connectTimeout: const Duration(seconds: 5),
      receiveTimeout: const Duration(seconds: 5),
    ),
  );
  dio.interceptors.add(AuthInterceptor(ref));
  return dio;
});

/// Attaches the Bearer token on the way out (AC4) and, on a `401` for any
/// authed call, logs the user out (AC5). The `/auth/login` and `/auth/register`
/// paths are EXEMPT — their 401 is "bad credentials" (a message), not session
/// expiry, so they must not trigger a logout loop (SPEC-0007 §2.6).
class AuthInterceptor extends Interceptor {
  AuthInterceptor(this._ref);

  final Ref _ref;

  static const _exemptPaths = {'/auth/login', '/auth/register'};

  @override
  void onRequest(RequestOptions options, RequestInterceptorHandler handler) {
    final token = _ref.read(authControllerProvider.notifier).token;
    if (token != null) {
      options.headers['Authorization'] = 'Bearer ${token.jwt}';
    }
    handler.next(options);
  }

  @override
  void onError(DioException err, ErrorInterceptorHandler handler) {
    final isUnauthorized = err.response?.statusCode == 401;
    final isExempt = _exemptPaths.contains(err.requestOptions.path);
    if (isUnauthorized && !isExempt) {
      _ref.read(authControllerProvider.notifier).logout();
    }
    handler.next(err);
  }
}
