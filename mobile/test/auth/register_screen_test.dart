// SAC1 -> AC1: RegisterScreen calls register (which auto-logs-in). On success
//   the session authenticates; a 400 (bad email / weak password) and a 409
//   (duplicate email) each render a readable inline message and leave the user
//   unauthenticated (still on register). Nothing crashes.
//
// RED until package:fitai/src/auth/presentation/register_screen.dart defines
// RegisterScreen and the controller/repository surface it consumes.

import 'package:dio/dio.dart';
import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/auth/domain/auth_state.dart';
import 'package:fitai/src/auth/presentation/register_screen.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/l10n.dart';

void main() {
  setUpAll(registerFallbacks);

  late MockTokenStore tokenStore;
  late MockAuthRepository repo;

  setUp(() {
    tokenStore = MockTokenStore();
    repo = MockAuthRepository();
    when(() => tokenStore.read()).thenAnswer((_) async => null);
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
  });

  Future<ProviderContainer> pumpRegister(WidgetTester tester) async {
    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: localizedApp(home: const RegisterScreen()),
      ),
    );
    await tester.pump();
    return container;
  }

  Future<void> submit(WidgetTester tester) async {
    final fields = find.byType(TextField);
    await tester.enterText(fields.first, 'new@b.com');
    await tester.enterText(fields.last, 'password1');
    await tester.tap(find.byType(ElevatedButton));
    await tester.pumpAndSettle();
  }

  testWidgets('SAC1 register success auto-logs-in and authenticates',
      (tester) async {
    when(() => repo.register('new@b.com', 'password1'))
        .thenAnswer((_) async {});
    when(() => repo.login('new@b.com', 'password1'))
        .thenAnswer((_) async => sampleToken(userId: 'u-new'));
    final container = await pumpRegister(tester);

    await submit(tester);

    expect(container.read(authControllerProvider), isA<AuthAuthenticated>());
  });

  testWidgets('SAC1 a 400 bad email renders a readable inline message',
      (tester) async {
    when(() => repo.register(any(), any())).thenThrow(
      const ApiException('enter a valid email',
          statusCode: 400, field: 'email'),
    );
    final container = await pumpRegister(tester);

    await submit(tester);

    expect(find.textContaining('valid email'), findsOneWidget);
    expect(container.read(authControllerProvider), isA<AuthUnauthenticated>());
  });

  testWidgets('SAC1 a 409 duplicate email says the email has an account',
      (tester) async {
    // The exception exactly as the data layer builds it from the server's
    // real 409 body, which carries no message of its own. This test used to
    // fake the message ('that email is already registered'), so it kept
    // passing after the shared copy became the generic 'that already exists'
    // and the screen showed users a sentence that read as a rejection.
    final request = RequestOptions(path: '/auth/register');
    when(() => repo.register(any(), any())).thenThrow(
      ApiException.fromDio(
        DioException(
          requestOptions: request,
          response: Response(
            requestOptions: request,
            statusCode: 409,
            data: <String, dynamic>{'error': 'already_exists'},
          ),
        ),
      ),
    );
    final container = await pumpRegister(tester);

    await submit(tester);

    expect(find.text(l10n.emailAlreadyRegistered),
        findsOneWidget);
    expect(find.textContaining('that already exists'), findsNothing);
    expect(container.read(authControllerProvider), isA<AuthUnauthenticated>());
    verifyNever(() => repo.login(any(), any()));
  });
}
