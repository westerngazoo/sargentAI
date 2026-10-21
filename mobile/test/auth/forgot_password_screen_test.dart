// R-0060 M2 -> AC7: ForgotPasswordScreen submits the trimmed email through
//   AuthRepository.forgotPassword and then replaces the form with the neutral
//   confirmation (the same text for every address) and a "Back to log in"
//   button. A failed request shows no confirmation and no raw exception.
//
// Seam (SPEC-0007 §2.5, one seam for screens): the screen reads
// authRepositoryProvider; AuthRepository.forgotPassword passes through to
// AuthApi.forgotPassword (SPEC-0060 §2.7), which auth_api_test.dart covers.
//
// RED until package:fitai/src/auth/presentation/forgot_password_screen.dart
// defines ForgotPasswordScreen and AuthRepository gains forgotPassword.

import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/auth/presentation/forgot_password_screen.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';

const _confirmation =
    'If that address has an account, a reset link is on its way';

void main() {
  setUpAll(registerFallbacks);

  late MockTokenStore tokenStore;
  late MockAuthRepository repo;
  late GoRouter router;

  setUp(() {
    tokenStore = MockTokenStore();
    repo = MockAuthRepository();
    when(() => tokenStore.read()).thenAnswer((_) async => null);
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
    // A minimal router: the screen under test plus the destination its
    // "Back to log in" button targets.
    router = GoRouter(
      initialLocation: '/forgot-password',
      routes: [
        GoRoute(
          path: '/forgot-password',
          builder: (_, __) => const ForgotPasswordScreen(),
        ),
        GoRoute(
          path: '/login',
          builder: (_, __) => const Scaffold(body: Text('login-screen')),
        ),
      ],
    );
  });

  Future<void> pumpForgot(WidgetTester tester) async {
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
        child: MaterialApp.router(routerConfig: router),
      ),
    );
    await tester.pump();
  }

  Future<void> submit(WidgetTester tester, String email) async {
    await tester.enterText(find.byType(TextField), email);
    await tester.tap(find.byType(ElevatedButton));
    await tester.pumpAndSettle();
  }

  testWidgets('M2 the screen asks for one email and offers "Send reset link"',
      (tester) async {
    await pumpForgot(tester);

    expect(find.byType(TextField), findsOneWidget);
    expect(
        find.widgetWithText(ElevatedButton, 'Send reset link'), findsOneWidget);
    expect(find.textContaining(_confirmation), findsNothing);
  });

  testWidgets('M2 submit calls forgotPassword with the trimmed email',
      (tester) async {
    when(() => repo.forgotPassword(any())).thenAnswer((_) async {});
    await pumpForgot(tester);

    await submit(tester, '  a@b.com  ');

    verify(() => repo.forgotPassword('a@b.com')).called(1);
  });

  testWidgets('M2 after 202 the form is replaced by the neutral confirmation',
      (tester) async {
    when(() => repo.forgotPassword(any())).thenAnswer((_) async {});
    await pumpForgot(tester);

    await submit(tester, 'a@b.com');

    expect(find.byType(TextField), findsNothing);
    expect(find.textContaining(_confirmation), findsOneWidget);
    expect(find.textContaining('30 minutes'), findsOneWidget);
    expect(
        find.widgetWithText(ElevatedButton, 'Back to log in'), findsOneWidget);
  });

  testWidgets('M2 "Back to log in" navigates to /login', (tester) async {
    when(() => repo.forgotPassword(any())).thenAnswer((_) async {});
    await pumpForgot(tester);
    await submit(tester, 'a@b.com');

    await tester.tap(find.text('Back to log in'));
    await tester.pumpAndSettle();

    expect(router.routerDelegate.currentConfiguration.uri.path, '/login');
    expect(find.text('login-screen'), findsOneWidget);
  });

  testWidgets('M2 the confirmation is the same text for any address',
      (tester) async {
    when(() => repo.forgotPassword(any())).thenAnswer((_) async {});
    await pumpForgot(tester);
    await submit(tester, 'nobody-has-this@b.com');
    final shown = tester.widget<Text>(find.textContaining(_confirmation)).data;

    expect(shown, isNotNull);
    expect(shown, isNot(contains('nobody-has-this')));
  });

  testWidgets('M2 a failed request shows no confirmation and no raw exception',
      (tester) async {
    when(() => repo.forgotPassword(any())).thenThrow(
      const ApiException("can't reach the server — retry"),
    );
    await pumpForgot(tester);

    await submit(tester, 'a@b.com');

    expect(find.textContaining(_confirmation), findsNothing);
    expect(find.byType(TextField), findsOneWidget);
    expect(find.textContaining('retry'), findsOneWidget);
    expect(find.textContaining('Exception'), findsNothing);
  });

  testWidgets('M2 the button disables while the request is in flight',
      (tester) async {
    when(() => repo.forgotPassword(any()))
        .thenAnswer((_) => Future<void>.delayed(const Duration(seconds: 1)));
    await pumpForgot(tester);

    await tester.enterText(find.byType(TextField), 'a@b.com');
    await tester.tap(find.byType(ElevatedButton));
    await tester.pump();

    final button = tester.widget<ElevatedButton>(find.byType(ElevatedButton));
    expect(button.onPressed, isNull);
    await tester.pumpAndSettle();
  });
}
