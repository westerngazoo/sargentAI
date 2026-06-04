// SAC1/SAC2/SAC4 (data layer): AuthApi maps the R-0002 wire shapes and
// translates DioException -> ApiException.
//   register: POST /auth/register -> 201 {user_id};  400/409 -> ApiException
//   login:    POST /auth/login    -> 200 {token,user_id,expires_at}; 401 -> ApiException
//   me:       GET  /auth/me       -> 200 {user_id}
//   timeout/no-response -> ApiException(statusCode: null) — the AC9 retry text.
//
// RED until package:fitai/src/auth/data/auth_api.dart defines AuthApi(Dio) with
// register/login/me.

import 'package:dio/dio.dart';
import 'package:fitai/src/auth/data/auth_api.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';

Response<T> _ok<T>(String path, int status, T data) {
  final req = RequestOptions(path: path);
  return Response<T>(requestOptions: req, statusCode: status, data: data);
}

void main() {
  setUpAll(registerFallbacks);

  late MockDio dio;
  late AuthApi api;

  setUp(() {
    dio = MockDio();
    api = AuthApi(dio);
  });

  group('SAC1 register', () {
    test('201 {user_id} completes without throwing', () async {
      when(() => dio.post<dynamic>('/auth/register', data: any(named: 'data')))
          .thenAnswer(
              (_) async => _ok('/auth/register', 201, {'user_id': 'user-1'}));
      await expectLater(api.register('a@b.com', 'pw'), completes);
    });

    test('400 bad email -> field-aware ApiException', () async {
      when(() => dio.post<dynamic>('/auth/register', data: any(named: 'data')))
          .thenThrow(dioError(
        400,
        path: '/auth/register',
        body: {
          'error': {'message': 'invalid email', 'field': 'email'}
        },
      ));
      await expectLater(
        api.register('bad', 'pw'),
        throwsA(isA<ApiException>()
            .having((e) => e.statusCode, 'statusCode', 400)
            .having((e) => e.field, 'field', 'email')),
      );
    });

    test('409 duplicate email -> ApiException(409)', () async {
      when(() => dio.post<dynamic>('/auth/register', data: any(named: 'data')))
          .thenThrow(dioError(409, path: '/auth/register'));
      await expectLater(
        api.register('dup@b.com', 'pw'),
        throwsA(isApiExceptionWithStatus(409)),
      );
    });
  });

  group('SAC2 login', () {
    test('200 parses {token,user_id,expires_at} into AuthToken', () async {
      when(() => dio.post<dynamic>('/auth/login', data: any(named: 'data')))
          .thenAnswer((_) async => _ok('/auth/login', 200, {
                'token': 'jwt.live',
                'user_id': 'user-7',
                'expires_at': '2030-01-01T00:00:00Z',
              }));
      final AuthToken token = await api.login('a@b.com', 'pw');
      expect(token.jwt, 'jwt.live');
      expect(token.userId, 'user-7');
    });

    test('401 -> non-enumerating ApiException(401)', () async {
      when(() => dio.post<dynamic>('/auth/login', data: any(named: 'data')))
          .thenThrow(dioError(401, path: '/auth/login'));
      await expectLater(
        api.login('a@b.com', 'wrong'),
        throwsA(isApiExceptionWithStatus(401)),
      );
    });

    test('timeout -> ApiException with null statusCode (retryable)', () async {
      when(() => dio.post<dynamic>('/auth/login', data: any(named: 'data')))
          .thenThrow(dioTimeout(path: '/auth/login'));
      await expectLater(
        api.login('a@b.com', 'pw'),
        throwsA(isApiExceptionWithStatus(null)),
      );
    });
  });

  group('SAC4 me', () {
    test('200 {user_id} returns the user id', () async {
      when(() => dio.get<dynamic>('/auth/me'))
          .thenAnswer((_) async => _ok('/auth/me', 200, {'user_id': 'me-1'}));
      expect(await api.me(), 'me-1');
    });
  });
}
