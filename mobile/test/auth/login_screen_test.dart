// SAC2 -> AC2: LoginScreen drives AuthController.login; success authenticates,
//   a 401 renders the non-enumerating "invalid email or password" message.
// SAC9 -> AC9: in-flight submit shows a spinner and is disabled (no
//   double-submit); a transport/timeout error renders a retryable message; no
//   raw exception reaches the UI.
//
// RED until package:fitai/src/auth/presentation/login_screen.dart defines
// LoginScreen and the controller/repository surface it consumes.

import 'dart:async';

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/auth/domain/auth_state.dart';
import 'package:fitai/src/auth/presentation/login_screen.dart';
import 'package:fitai/src/core/dev_login.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';

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

  Future<ProviderContainer> pumpLogin(WidgetTester tester) async {
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
        child: const MaterialApp(home: LoginScreen()),
      ),
    );
    await tester.pump();
    return container;
  }

  Future<void> enterCreds(WidgetTester tester) async {
    final fields = find.byType(TextField);
    await tester.enterText(fields.first, 'a@b.com');
    await tester.enterText(fields.last, 'password1');
  }

  testWidgets('SAC2 successful login authenticates the session',
      (tester) async {
    when(() => repo.login('a@b.com', 'password1'))
        .thenAnswer((_) async => sampleToken(userId: 'u-ok'));
    final container = await pumpLogin(tester);

    await enterCreds(tester);
    await tester.tap(find.byType(ElevatedButton));
    await tester.pumpAndSettle();

    expect(container.read(authControllerProvider), isA<AuthAuthenticated>());
  });

  testWidgets('SAC2 a 401 renders the non-enumerating message', (tester) async {
    when(() => repo.login(any(), any())).thenThrow(
      const ApiException('invalid email or password', statusCode: 401),
    );
    await pumpLogin(tester);

    await enterCreds(tester);
    await tester.tap(find.byType(ElevatedButton));
    await tester.pumpAndSettle();

    expect(find.textContaining('invalid email or password'), findsOneWidget);
  });

  testWidgets('SAC9 a timeout renders a retryable message', (tester) async {
    when(() => repo.login(any(), any())).thenThrow(
      const ApiException("can't reach the server — retry"),
    );
    await pumpLogin(tester);

    await enterCreds(tester);
    await tester.tap(find.byType(ElevatedButton));
    await tester.pumpAndSettle();

    expect(find.textContaining('retry'), findsOneWidget);
  });

  testWidgets('SAC9 the button shows a spinner and disables while in flight',
      (tester) async {
    final gate = Completer<AuthToken>();
    when(() => repo.login(any(), any())).thenAnswer((_) => gate.future);
    await pumpLogin(tester);

    await enterCreds(tester);
    await tester.tap(find.byType(ElevatedButton));
    await tester.pump(); // start the async call, do not settle

    // Busy indicator visible.
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    // Double-submit prevented: the button is disabled (onPressed == null).
    final button = tester.widget<ElevatedButton>(find.byType(ElevatedButton));
    expect(button.onPressed, isNull);

    gate.complete(sampleToken());
    await tester.pumpAndSettle();
  });

  testWidgets('SAC9 no raw exception text is shown to the user',
      (tester) async {
    when(() => repo.login(any(), any())).thenThrow(
      const ApiException('invalid email or password', statusCode: 401),
    );
    await pumpLogin(tester);

    await enterCreds(tester);
    await tester.tap(find.byType(ElevatedButton));
    await tester.pumpAndSettle();

    expect(find.textContaining('Exception'), findsNothing);
    expect(find.textContaining('DioException'), findsNothing);
  });

  testWidgets('debug quick login submits the DevLogin test account (real flow)',
      (tester) async {
    // flutter test always runs in debug mode, so the button must be present.
    when(() => repo.login(DevLogin.email, DevLogin.password))
        .thenAnswer((_) async => sampleToken(userId: 'u-test'));
    when(() => tokenStore.write(any())).thenAnswer((_) async {});
    final container = await pumpLogin(tester);

    await tester.tap(find.text('Use test account'));
    await tester.pumpAndSettle();

    verify(() => repo.login(DevLogin.email, DevLogin.password)).called(1);
    expect(container.read(authControllerProvider), isA<AuthAuthenticated>());
  });
}
