// SAC9 -> AC9 (model layer): ApiException is the typed, user-safe error model.
//   kind (what went wrong, localized by the UI), statusCode (null =
//   transport/timeout), optional field. R-0061 AC11: no English fallback
//   message lives here — apiErrorText renders the kind from the catalogue.

import 'package:fitai/src/core/network/api_exception.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('SAC9 ApiException shape', () {
    test('carries kind, statusCode and field', () {
      const e = ApiException(
        ApiErrorKind.badRequest,
        statusCode: 400,
        field: 'email',
      );
      expect(e.kind, ApiErrorKind.badRequest);
      expect(e.statusCode, 400);
      expect(e.field, 'email');
    });

    test('a transport/timeout error has a null statusCode', () {
      const e = ApiException(ApiErrorKind.transport);
      expect(e.kind, ApiErrorKind.transport);
      expect(e.statusCode, isNull);
      expect(e.field, isNull);
    });

    test('is an Exception (never surfaces as a raw stack trace)', () {
      const e = ApiException(ApiErrorKind.unknown);
      expect(e, isA<Exception>());
    });
  });
}
