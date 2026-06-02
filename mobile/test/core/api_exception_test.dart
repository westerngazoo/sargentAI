// SAC9 -> AC9 (model layer): ApiException is the typed, user-safe error model.
//   statusCode (null = transport/timeout), optional field, user-safe message.
//
// RED until package:fitai/src/core/network/api_exception.dart defines
// ApiException { statusCode, field, message }.

import 'package:fitai/src/core/network/api_exception.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('SAC9 ApiException shape', () {
    test('carries statusCode, field, and a user-safe message', () {
      const e = ApiException(
        'bad email',
        statusCode: 400,
        field: 'email',
      );
      expect(e.statusCode, 400);
      expect(e.field, 'email');
      expect(e.message, 'bad email');
    });

    test('a transport/timeout error has a null statusCode', () {
      const e = ApiException("can't reach the server — retry");
      expect(e.statusCode, isNull);
      expect(e.field, isNull);
      expect(e.message, isNotEmpty);
    });

    test('is an Exception (never surfaces as a raw stack trace)', () {
      const e = ApiException('x');
      expect(e, isA<Exception>());
    });
  });
}
