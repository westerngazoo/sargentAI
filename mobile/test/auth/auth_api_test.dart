// SAC1/SAC2/SAC4 (data layer): AuthApi maps the R-0002 wire shapes and
// translates DioException -> ApiException.
//   register: POST /auth/register -> 201 {user_id};  400/409 -> ApiException
//   login:    POST /auth/login    -> 200 {token,user_id,expires_at}; 401 -> ApiException
//   me:       GET  /auth/me       -> 200 {user_id,email}
//   timeout/no-response -> ApiException(statusCode: null) — the AC9 retry text.
//
// RED until package:fitai/src/auth/data/auth_api.dart defines AuthApi(Dio) with
// register/login/me.

import 'package:dio/dio.dart';
import 'package:fitai/src/auth/data/auth_api.dart';
import 'package:fitai/src/auth/domain/me.dart';
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

    test('400 bad email -> field-aware ApiException (FLAT backend body)',
        () async {
      // The backend sends the FLAT shape `{"error":"validation","field":...}`
      // (`backend/crates/api/src/error.rs`); the corrected shared
      // `ApiException.fromDio` (SPEC-0008 §2.7) reads `data["field"]` directly,
      // so `field` is now populated instead of the always-null R-0007 bug.
      when(() => dio.post<dynamic>('/auth/register', data: any(named: 'data')))
          .thenThrow(dioError(
        400,
        path: '/auth/register',
        body: {'error': 'validation', 'field': 'email'},
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
    test('200 {user_id,email} returns both (SPEC-0061 slice 3c)', () async {
      when(() => dio.get<dynamic>('/auth/me')).thenAnswer((_) async =>
          _ok('/auth/me', 200, {'user_id': 'me-1', 'email': 'me@b.com'}));
      final me = await api.me();
      expect(me, isA<Me>());
      expect(me.userId, 'me-1');
      expect(me.email, 'me@b.com');
    });

    test('401 -> ApiException(401)', () async {
      when(() => dio.get<dynamic>('/auth/me'))
          .thenThrow(dioError(401, path: '/auth/me'));
      await expectLater(api.me(), throwsA(isApiExceptionWithStatus(401)));
    });
  });

  // R-0060 -> AC7 (data layer, SPEC-0060 §2.7):
  //   forgotPassword: POST /auth/password/forgot {email}          -> 202
  //   resetPassword:  POST /auth/password/reset  {token,password} -> 204;
  //                   401 -> ApiException(401) (link no longer valid);
  //                   400 -> ApiException(400, field: password).
  group('R-0060 forgotPassword', () {
    test('posts {email} to /auth/password/forgot and completes on 202',
        () async {
      when(() => dio.post<dynamic>('/auth/password/forgot',
              data: any(named: 'data')))
          .thenAnswer((_) async => _ok('/auth/password/forgot', 202, null));

      await expectLater(api.forgotPassword('a@b.com'), completes);

      verify(() => dio.post<dynamic>('/auth/password/forgot',
          data: <String, String>{'email': 'a@b.com'})).called(1);
    });

    test('timeout -> ApiException with null statusCode (retryable)', () async {
      when(() => dio.post<dynamic>('/auth/password/forgot',
              data: any(named: 'data')))
          .thenThrow(dioTimeout(path: '/auth/password/forgot'));
      await expectLater(
        api.forgotPassword('a@b.com'),
        throwsA(isApiExceptionWithStatus(null)),
      );
    });
  });

  group('R-0060 resetPassword', () {
    test('posts {token,password} to /auth/password/reset and completes on 204',
        () async {
      when(() => dio.post<dynamic>('/auth/password/reset',
              data: any(named: 'data')))
          .thenAnswer((_) async => _ok('/auth/password/reset', 204, null));

      await expectLater(
        api.resetPassword(token: 'abc', password: 'newpassword1'),
        completes,
      );

      verify(() =>
          dio.post<dynamic>('/auth/password/reset', data: <String, String>{
            'token': 'abc',
            'password': 'newpassword1',
          })).called(1);
    });

    test('401 -> ApiException(401): the link is no longer valid', () async {
      when(() => dio.post<dynamic>('/auth/password/reset',
          data: any(named: 'data'))).thenThrow(dioError(
        401,
        path: '/auth/password/reset',
        body: {'error': 'unauthorized'},
      ));
      await expectLater(
        api.resetPassword(token: 'dead', password: 'newpassword1'),
        throwsA(isApiExceptionWithStatus(401)),
      );
    });

    test('400 short password -> field-aware ApiException(400, password)',
        () async {
      when(() => dio.post<dynamic>('/auth/password/reset',
          data: any(named: 'data'))).thenThrow(dioError(
        400,
        path: '/auth/password/reset',
        body: {'error': 'validation', 'field': 'password'},
      ));
      await expectLater(
        api.resetPassword(token: 'abc', password: 'short'),
        throwsA(isA<ApiException>()
            .having((e) => e.statusCode, 'statusCode', 400)
            .having((e) => e.field, 'field', 'password')),
      );
    });
  });
}
