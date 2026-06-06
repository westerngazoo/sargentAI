// SAC8 -> AC8 (error-parsing authority): the shared, corrected
// `ApiException.fromDio(DioException)` reads the backend's FLAT error body
// `{"error":"<kind>","field":"<name>"}` and populates `field` for the AC8
// 400-field -> step jump. This is the single fix for the R-0007 latent bug
// (the old `_toApiException` only read `field` from a NESTED `{"error":{...}}`
// shape the backend never sends, so `field` was always null).
//
// RED until package:fitai/src/core/network/api_exception.dart adds the
// `ApiException.fromDio` factory (SPEC-0008 §2.7).

import 'package:dio/dio.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/profile_fakes.dart';

void main() {
  group('SAC8 ApiException.fromDio — flat backend body', () {
    test('a 400 validation body populates field from the FLAT shape', () {
      final e = ApiException.fromDio(
        dioErrorFlat(400, field: 'height_cm'),
      );
      expect(e.statusCode, 400);
      expect(e.field, 'height_cm', reason: 'reads data["field"], not nested');
      expect(e.message, isNotEmpty);
    });

    test('a 400 with no field leaves field null but keeps the status', () {
      final e = ApiException.fromDio(
        dioErrorFlat(400, error: 'validation'),
      );
      expect(e.statusCode, 400);
      expect(e.field, isNull);
    });

    test('a 401 carries statusCode 401 and a null field', () {
      final e = ApiException.fromDio(
        dioErrorFlat(401, path: '/profile/me', error: 'unauthorized'),
      );
      expect(e.statusCode, 401);
      expect(e.field, isNull);
    });

    test('a 404 carries statusCode 404 (callers may special-case it)', () {
      final e = ApiException.fromDio(
        dioErrorFlat(404, error: 'not_found'),
      );
      expect(e.statusCode, 404);
    });

    test('a transport/timeout (null response) -> retryable, null statusCode',
        () {
      final e = ApiException.fromDio(dioTransport());
      expect(e.statusCode, isNull, reason: 'no HTTP response = transport');
      expect(e.field, isNull);
      expect(e.message, isNotEmpty);
    });

    test('does NOT read a nested {"error":{"field":...}} body (regression)',
        () {
      // The backend never sends this nested shape; if fromDio mistakenly
      // parsed it, the old R-0007 bug would re-appear. A nested map under
      // `error` must NOT surface as a top-level field.
      final req = RequestOptions(path: '/profile/me');
      final e = ApiException.fromDio(
        DioException(
          requestOptions: req,
          type: DioExceptionType.badResponse,
          response: Response<dynamic>(
            requestOptions: req,
            statusCode: 400,
            data: <String, dynamic>{
              'error': <String, dynamic>{'field': 'email'},
            },
          ),
        ),
      );
      expect(e.statusCode, 400);
      expect(e.field, isNull,
          reason: 'field comes from a flat data["field"], not error.field');
    });

    test('never surfaces as a raw stack trace (is an Exception)', () {
      final e = ApiException.fromDio(dioTransport());
      expect(e, isA<Exception>());
    });
  });
}
