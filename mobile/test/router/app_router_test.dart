// SAC3 -> AC3: the router auth-gate routes AuthUnknown->/splash,
//   AuthUnauthenticated->/login, AuthAuthenticated->/home.
// SAC5/SAC6 -> AC5/AC6: a logout (state -> AuthUnauthenticated) re-runs the
//   redirect in place and lands on /login (refreshListenable, not a rebuild).
// SAC8 -> AC8: the redirect reads authControllerProvider as the sole source.
//
// RED until package:fitai/src/router/app_router.dart defines routerProvider
// (GoRouter) and the screen widgets it routes to.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/auth/presentation/reset_password_screen.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/router/app_router.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
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
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
    when(() => repo.me()).thenAnswer((_) async => 'u-1');
  });

  Future<(ProviderContainer, GoRouter)> pumpApp(WidgetTester tester) async {
    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
      ],
    );
    addTearDown(container.dispose);
    final router = container.read(routerProvider);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: localizedRouterApp(router),
      ),
    );
    await tester.pumpAndSettle();
    return (container, router);
  }

  String currentLocation(GoRouter router) =>
      router.routerDelegate.currentConfiguration.uri.path;

  testWidgets('SAC3 a present token lands on /home (no login flash)',
      (tester) async {
    when(() => tokenStore.read())
        .thenAnswer((_) async => sampleToken(userId: 'u-1'));
    final (_, router) = await pumpApp(tester);
    expect(currentLocation(router), '/home');
  });

  testWidgets('SAC3 no token lands on /login', (tester) async {
    when(() => tokenStore.read()).thenAnswer((_) async => null);
    final (_, router) = await pumpApp(tester);
    expect(currentLocation(router), '/login');
  });

  testWidgets('SAC3 a throwing read clears the entry and lands on /login',
      (tester) async {
    when(() => tokenStore.read()).thenThrow(Exception('corrupt'));
    final (_, router) = await pumpApp(tester);
    verify(() => tokenStore.clear()).called(1);
    expect(currentLocation(router), '/login');
  });

  testWidgets('SAC5/SAC6 logout re-runs the gate and lands on /login',
      (tester) async {
    when(() => tokenStore.read())
        .thenAnswer((_) async => sampleToken(userId: 'u-1'));
    final (container, router) = await pumpApp(tester);
    expect(currentLocation(router), '/home');

    await container.read(authControllerProvider.notifier).logout();
    await tester.pumpAndSettle();

    expect(currentLocation(router), '/login');
  });

  // R-0060 M6 -> AC7 (SPEC-0060 §2.7, architect finding 1): an emailed link
  // is always a cold start, so the AuthUnknown arm must pass /reset-password
  // through with its query intact instead of sending it to /splash.
  group('R-0060 M6 cold start on the reset link', () {
    /// Mirrors the browser opening the app at [location]: the router sees the
    /// link while auth is still AuthUnknown (restore is a pending microtask),
    /// then auth resolves from the token store.
    Future<GoRouter> coldStartAt(WidgetTester tester, String location) async {
      final container = ProviderContainer(
        overrides: [
          tokenStoreProvider.overrideWithValue(tokenStore),
          authRepositoryProvider.overrideWithValue(repo),
        ],
      );
      addTearDown(container.dispose);
      final router = container.read(routerProvider);
      router.go(location);
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: localizedRouterApp(router),
        ),
      );
      await tester.pumpAndSettle();
      return router;
    }

    void expectResetScreenWithToken(GoRouter router) {
      expect(currentLocation(router), '/reset-password');
      expect(find.byType(ResetPasswordScreen), findsOneWidget);
      // The token was captured: the form is shown, not the invalid-link
      // message the screen shows when it has no token.
      expect(find.text(l10n.linkNoLongerValid), findsNothing);
      expect(find.byType(TextField), findsNWidgets(2));
      // ...and then dropped from the URL (SPEC-0060 §2.7, finding 12).
      expect(
        router.routerDelegate.currentConfiguration.uri.queryParameters
            .containsKey('token'),
        isFalse,
      );
    }

    testWidgets('unauthenticated: lands on the reset screen with the token',
        (tester) async {
      when(() => tokenStore.read()).thenAnswer((_) async => null);
      final router = await coldStartAt(tester, '/reset-password?token=abc');
      expectResetScreenWithToken(router);
    });

    testWidgets('authenticated: lands on the reset screen with the token',
        (tester) async {
      when(() => tokenStore.read())
          .thenAnswer((_) async => sampleToken(userId: 'u-1'));
      final router = await coldStartAt(tester, '/reset-password?token=abc');
      expectResetScreenWithToken(router);
    });

    testWidgets('unauthenticated: /forgot-password is allowed', (tester) async {
      when(() => tokenStore.read()).thenAnswer((_) async => null);
      final router = await coldStartAt(tester, '/forgot-password');
      expect(currentLocation(router), '/forgot-password');
    });

    testWidgets('authenticated: /forgot-password redirects home like /login',
        (tester) async {
      when(() => tokenStore.read())
          .thenAnswer((_) async => sampleToken(userId: 'u-1'));
      final router = await coldStartAt(tester, '/forgot-password');
      expect(currentLocation(router), '/home');
    });
  });
}
