// SAC4 -> AC4: HomeShell renders the user_id from GET /auth/me.
// SAC11 -> AC11: the shell is a placeholder — AppBar titled 'fitAI', the user,
//   and a Logout action; no feature logger UI.
// SAC5 -> AC5 (cold-start-stale-token): when GET /auth/me 401s, the shell's
//   load triggers logout (token cleared, state AuthUnauthenticated).
//
// RED until package:fitai/src/shell/home_shell.dart defines HomeShell and the
// auth surface it consumes.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/auth/domain/auth_state.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/shell/home_shell.dart';
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
    when(() => tokenStore.read())
        .thenAnswer((_) async => sampleToken(userId: 'u-1'));
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
  });

  Future<ProviderContainer> pumpShell(WidgetTester tester) async {
    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
      ],
    );
    addTearDown(container.dispose);
    // Settle the restore so the controller is AuthAuthenticated.
    container.read(authControllerProvider);
    await Future<void>.delayed(Duration.zero);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: HomeShell()),
      ),
    );
    return container;
  }

  testWidgets('SAC4/SAC11 shows the AppBar title, the user, and Logout',
      (tester) async {
    when(() => repo.me()).thenAnswer((_) async => 'u-42');
    await pumpShell(tester);
    await tester.pumpAndSettle();

    expect(find.widgetWithText(AppBar, 'fitAI'), findsOneWidget);
    expect(find.textContaining('u-42'), findsOneWidget);
    expect(find.textContaining('Logout'), findsOneWidget);
  });

  testWidgets('SAC11 the shell renders no feature logger UI', (tester) async {
    when(() => repo.me()).thenAnswer((_) async => 'u-1');
    await pumpShell(tester);
    await tester.pumpAndSettle();

    for (final feature in const [
      'Workout',
      'Nutrition',
      'Dashboard',
      'Photo'
    ]) {
      expect(find.textContaining(feature), findsNothing);
    }
  });

  testWidgets('SAC5 a 401 from /auth/me logs the user out', (tester) async {
    when(() => repo.me()).thenThrow(
      const ApiException('session expired', statusCode: 401),
    );
    final container = await pumpShell(tester);
    // The 401 path calls logout() but deliberately leaves the loading spinner
    // up: in the running app the router redirect unmounts the shell, but this
    // isolated test has no router, so the CircularProgressIndicator never stops
    // animating and pumpAndSettle would block until its timeout. Bounded pumps
    // flush the async chain (me() 401 -> logout -> clear -> state) instead.
    await tester.pump(); // me() rejects, logout() runs, awaits clear()
    await tester.pump(); // clear() resolves, state -> AuthUnauthenticated

    verify(() => repo.clear()).called(1);
    expect(container.read(authControllerProvider), isA<AuthUnauthenticated>());
  });
}
