// SAC2 (persistence) + SAC6 (clear): TokenStore wraps FlutterSecureStorage with
// read/write/clear, JSON round-tripping an AuthToken {jwt,userId,expiresAt}.
// Also exercises the SAC3 read-throws branch at the store boundary.
//
// RED until package:fitai/src/core/storage/token_store.dart defines AuthToken
// and TokenStore(FlutterSecureStorage) with read/write/clear (SPEC-0007 §3.1).

import 'package:fitai/src/core/storage/token_store.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

class _MockSecureStorage extends Mock implements FlutterSecureStorage {}

void main() {
  late _MockSecureStorage storage;
  late TokenStore store;

  setUp(() {
    storage = _MockSecureStorage();
    store = TokenStore(storage);
  });

  group('SAC2/SAC6 TokenStore round-trip', () {
    test('write then read returns an equivalent AuthToken', () async {
      String? written;
      when(() => storage.write(
            key: any(named: 'key'),
            value: any(named: 'value'),
          )).thenAnswer((inv) async {
        written = inv.namedArguments[const Symbol('value')] as String;
      });
      when(() => storage.read(key: any(named: 'key')))
          .thenAnswer((_) async => written);

      final token = AuthToken(
        jwt: 'jwt.abc',
        userId: 'user-9',
        expiresAt: DateTime.utc(2030, 5, 1, 12),
      );
      await store.write(token);
      final got = await store.read();

      expect(got, isNotNull);
      expect(got!.jwt, 'jwt.abc');
      expect(got.userId, 'user-9');
      expect(got.expiresAt, DateTime.utc(2030, 5, 1, 12));
    });

    test('read returns null when nothing is stored', () async {
      when(() => storage.read(key: any(named: 'key')))
          .thenAnswer((_) async => null);
      expect(await store.read(), isNull);
    });

    test('clear deletes the stored entry', () async {
      when(() => storage.delete(key: any(named: 'key')))
          .thenAnswer((_) async {});
      await store.clear();
      verify(() => storage.delete(key: any(named: 'key'))).called(1);
    });
  });

  group('SAC3 corrupt/failed read', () {
    test('a storage read error propagates out of TokenStore.read', () async {
      when(() => storage.read(key: any(named: 'key')))
          .thenThrow(Exception('keystore unavailable'));
      expect(store.read(), throwsA(isA<Exception>()));
    });
  });
}
