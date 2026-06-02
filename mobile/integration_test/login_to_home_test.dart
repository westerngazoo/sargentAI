// SAC10 -> AC10: the end-to-end happy path — login → home — driven on the
// flutter-tester target against a MOCKED Dio (no device, no real backend).
// Exercises the full production stack: LoginScreen form → AuthApi (POST
// /auth/login) → token persisted → AuthAuthenticated → router redirect → the
// HomeShell rendering the user from GET /auth/me with a Bearer header.
//
// Run: `flutter test integration_test/` (headless flutter-tester).
//
// RED until the package:fitai/src/** surface and the FitAiApp root exist.

import 'package:dio/dio.dart';
import 'package:fitai/app.dart';
import 'package:fitai/src/core/network/dio_provider.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:mocktail/mocktail.dart';

class _MockDio extends Mock implements Dio {}

class _MockSecureStorage extends Mock implements FlutterSecureStorage {}

Response<T> _resp<T>(String path, int status, T data) {
  final req = RequestOptions(path: path);
  return Response<T>(requestOptions: req, statusCode: status, data: data);
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() {
    registerFallbackValue(RequestOptions(path: '/'));
  });

  testWidgets('login → home end-to-end against a mocked Dio', (tester) async {
    final dio = _MockDio();
    final storage = _MockSecureStorage();

    // Cold start: nothing stored → app opens to /login.
    final store = <String, String>{};
    when(() => storage.read(key: any(named: 'key')))
        .thenAnswer((inv) async => store[inv.namedArguments[#key]]);
    when(() => storage.write(
            key: any(named: 'key'), value: any(named: 'value')))
        .thenAnswer((inv) async {
      store[inv.namedArguments[#key] as String] =
          inv.namedArguments[#value] as String;
    });
    when(() => storage.delete(key: any(named: 'key'))).thenAnswer((inv) async {
      store.remove(inv.namedArguments[#key]);
    });

    // The mocked transport: login returns a token; /auth/me returns the user.
    when(() => dio.options).thenReturn(BaseOptions());
    when(() => dio.interceptors).thenReturn(Interceptors());
    when(() => dio.post<dynamic>('/auth/login', data: any(named: 'data')))
        .thenAnswer((_) async => _resp('/auth/login', 200, {
              'token': 'jwt.e2e',
              'user_id': 'e2e-user',
              'expires_at': '2030-01-01T00:00:00Z',
            }));
    when(() => dio.get<dynamic>('/auth/me'))
        .thenAnswer((_) async => _resp('/auth/me', 200, {'user_id': 'e2e-user'}));

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          dioProvider.overrideWithValue(dio),
          tokenStoreProvider.overrideWithValue(TokenStore(storage)),
        ],
        child: const FitAiApp(),
      ),
    );
    await tester.pumpAndSettle();

    // Land on login (no stored token).
    expect(find.byType(TextField), findsWidgets);

    // Enter credentials and submit.
    final fields = find.byType(TextField);
    await tester.enterText(fields.first, 'e2e@b.com');
    await tester.enterText(fields.last, 'password1');
    await tester.tap(find.byType(ElevatedButton));
    await tester.pumpAndSettle();

    // Now on the home shell, showing the /auth/me user.
    expect(find.widgetWithText(AppBar, 'fitAI'), findsOneWidget);
    expect(find.textContaining('e2e-user'), findsOneWidget);

    // The token was persisted to secure storage.
    expect(store.isNotEmpty, isTrue);
    verify(() => dio.post<dynamic>('/auth/login', data: any(named: 'data')))
        .called(1);
  });
}
