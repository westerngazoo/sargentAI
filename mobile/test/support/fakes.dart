// Shared test doubles + helpers for the R-0007 auth-shell suite.
//
// Authored by the qa agent in step 3 (test planning) — BEFORE any `lib/src`
// production code exists. These imports resolve to the exact class/provider
// surface SPEC-0007 §2/§3/§6 names, so the suite is RED until step-5
// implementation creates them, then turns GREEN with no test edits.
//
// Targeted production symbols (all under package:fitai/src/...):
//   core/network/api_exception.dart   -> ApiException
//   core/storage/token_store.dart     -> AuthToken, TokenStore, tokenStoreProvider
//   auth/domain/session.dart          -> Session
//   auth/data/auth_api.dart           -> AuthApi
//   auth/data/auth_repository.dart    -> authRepositoryProvider
//   auth/application/auth_controller.dart -> authControllerProvider

import 'package:dio/dio.dart';
import 'package:fitai/src/auth/data/auth_api.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

/// Mock HTTP client — lets a test stub register/login/me at the transport edge.
class MockDio extends Mock implements Dio {}

/// Mock secure-storage wrapper. SPEC-0007 §3.1: read/write/clear.
class MockTokenStore extends Mock implements TokenStore {}

/// Mock typed auth client. SPEC-0007 §3.2: register/login/me.
class MockAuthApi extends Mock implements AuthApi {}

/// Mock repository orchestrating api + token-store (SPEC-0007 §2.5 seams).
class MockAuthRepository extends Mock implements AuthRepository {}

/// A canned valid token used across the suite.
AuthToken sampleToken({
  String jwt = 'jwt.header.payload',
  String userId = 'user-123',
  DateTime? expiresAt,
}) =>
    AuthToken(
      jwt: jwt,
      userId: userId,
      expiresAt: expiresAt ?? DateTime.utc(2099, 1, 1),
    );

/// A 401 DioException with the optional backend `{"error":{...}}` body.
DioException dioError(
  int status, {
  String path = '/auth/login',
  Map<String, dynamic>? body,
}) {
  final req = RequestOptions(path: path);
  return DioException(
    requestOptions: req,
    type: DioExceptionType.badResponse,
    response: Response<dynamic>(
      requestOptions: req,
      statusCode: status,
      data: body,
    ),
  );
}

/// A transport/timeout DioException (no response) — drives the AC9 retry path.
DioException dioTimeout({String path = '/auth/login'}) {
  final req = RequestOptions(path: path);
  return DioException(
    requestOptions: req,
    type: DioExceptionType.connectionTimeout,
  );
}

/// Registers mocktail fallbacks for non-primitive argument types.
void registerFallbacks() {
  registerFallbackValue(sampleToken());
  registerFallbackValue(RequestOptions(path: '/'));
}

/// Convenience: an ApiException matcher by status code.
Matcher isApiExceptionWithStatus(int? status) =>
    isA<ApiException>().having((e) => e.statusCode, 'statusCode', status);
