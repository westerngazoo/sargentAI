// SAC1/SAC7/SAC8 -> AC1/AC7/AC8 (data layer): ProfileApi maps the R-0003 wire.
//   getMe: GET /profile/me -> 200 Profile ; 404 -> null (no profile yet, NOT an
//          error) ; other DioException -> ApiException.fromDio.
//   putMe: PUT /profile/me -> 200 Profile ; 400 -> ApiException carrying
//          statusCode + field ; transport -> null statusCode (retryable) ;
//          401 -> ApiException(401) (the interceptor handles the session sink).
//
// RED until package:fitai/src/profile/data/profile_api.dart defines
// ProfileApi(Dio) with getMe()/putMe(), using ApiException.fromDio.

import 'package:dio/dio.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/profile/data/profile_api.dart';
import 'package:fitai/src/profile/domain/goal.dart';
import 'package:fitai/src/profile/domain/profile.dart';
import 'package:fitai/src/profile/domain/profile_draft.dart';
import 'package:fitai/src/profile/domain/sex.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/fakes.dart';
import '../../support/profile_fakes.dart';

Response<T> _resp<T>(String path, int status, T data) {
  final req = RequestOptions(path: path);
  return Response<T>(requestOptions: req, statusCode: status, data: data);
}

ProfileRequest _validRequest() => ProfileDraft(
      dateOfBirth: DateTime(1996, 6, 6),
      heightCm: 180,
      weightKg: 82.5,
      goals: const {Goal.buildMuscle},
      sex: Sex.male,
    ).toRequest(DateTime(2026, 6, 6))!;

void main() {
  setUpAll(registerFallbacks);

  late MockDio dio;
  late ProfileApi api;

  setUp(() {
    dio = MockDio();
    api = ProfileApi(dio);
  });

  group('SAC1 getMe', () {
    test('200 parses the body into a Profile', () async {
      when(() => dio.get<Map<String, dynamic>>('/profile/me')).thenAnswer(
        (_) async =>
            _resp('/profile/me', 200, profileResponseJson(userId: 'u-9')),
      );
      final Profile? p = await api.getMe();
      expect(p, isNotNull);
      expect(p!.userId, 'u-9');
    });

    test('404 maps to null (no profile yet — NOT an error)', () async {
      when(() => dio.get<Map<String, dynamic>>('/profile/me'))
          .thenThrow(dioErrorFlat(404, error: 'not_found'));
      expect(await api.getMe(), isNull);
    });

    test('a non-404 error becomes an ApiException (carries statusCode)',
        () async {
      when(() => dio.get<Map<String, dynamic>>('/profile/me'))
          .thenThrow(dioErrorFlat(401, error: 'unauthorized'));
      await expectLater(
        api.getMe(),
        throwsA(isApiExceptionWithStatus(401)),
      );
    });

    test('a transport error becomes a retryable ApiException (null status)',
        () async {
      when(() => dio.get<Map<String, dynamic>>('/profile/me'))
          .thenThrow(dioTransport());
      await expectLater(
        api.getMe(),
        throwsA(isApiExceptionWithStatus(null)),
      );
    });
  });

  group('SAC7/SAC8 putMe', () {
    test('200 parses the saved profile from the response', () async {
      when(() => dio.put<Map<String, dynamic>>('/profile/me',
          data: any(named: 'data'))).thenAnswer(
        (_) async =>
            _resp('/profile/me', 200, profileResponseJson(userId: 'u-saved')),
      );
      final p = await api.putMe(_validRequest());
      expect(p.userId, 'u-saved');
    });

    test('a 400 carries statusCode 400 AND the offending field (AC8)',
        () async {
      when(() => dio.put<Map<String, dynamic>>('/profile/me',
              data: any(named: 'data')))
          .thenThrow(dioErrorFlat(400, field: 'weight_kg'));
      await expectLater(
        api.putMe(_validRequest()),
        throwsA(isA<ApiException>()
            .having((e) => e.statusCode, 'statusCode', 400)
            .having((e) => e.field, 'field', 'weight_kg')),
      );
    });

    test('a transport error -> retryable ApiException (null statusCode)',
        () async {
      when(() => dio.put<Map<String, dynamic>>('/profile/me',
          data: any(named: 'data'))).thenThrow(dioTransport());
      await expectLater(
        api.putMe(_validRequest()),
        throwsA(isApiExceptionWithStatus(null)),
      );
    });

    test('a 401 -> ApiException(401) (the interceptor handles the sink)',
        () async {
      when(() => dio.put<Map<String, dynamic>>('/profile/me',
              data: any(named: 'data')))
          .thenThrow(dioErrorFlat(401, error: 'unauthorized'));
      await expectLater(
        api.putMe(_validRequest()),
        throwsA(isApiExceptionWithStatus(401)),
      );
    });
  });
}
