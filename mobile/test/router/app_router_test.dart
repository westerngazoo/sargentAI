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
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/router/app_router.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';

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
        child: MaterialApp.router(routerConfig: router),
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
}
