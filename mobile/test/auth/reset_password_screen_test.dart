// R-0060 M3/M4 -> AC7: ResetPasswordScreen, opened from the emailed link
//   `/reset-password?token=<token>`.
//   M3: a mismatched confirmation blocks submit; a matching pair calls
//       AuthRepository.resetPassword(token: 'abc', password: ...); success
//       shows "Password updated" and lands on /login. The token is read once
//       and the route is replaced so the query leaves the address bar.
//   M4: a 401 replaces the form with "This link is no longer valid" and a
//       "Request a new link" button to /forgot-password; no token in the URL
//       shows that message straight away.
//
// Seam (SPEC-0007 §2.5): the screen reads authRepositoryProvider;
// AuthRepository.resetPassword passes through to AuthApi.resetPassword
// (SPEC-0060 §2.7), which auth_api_test.dart covers.
//
// Field order the tests bind to: first TextField = new password, last
// TextField = confirmation; one ElevatedButton "Set new password".
//
// RED until package:fitai/src/auth/presentation/reset_password_screen.dart
// defines ResetPasswordScreen({String? token}) and AuthRepository gains
// resetPassword.

import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/auth/presentation/reset_password_screen.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/l10n.dart';

final _invalidLink = l10n.linkNoLongerValid;

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
  });

  /// The screen under test wired exactly as app_router.dart wires it
  /// (SPEC-0060 §2.7: the route reads `state.uri.queryParameters['token']`),
  /// plus sentinel destinations for /login and /forgot-password.
  Future<void> pumpReset(WidgetTester tester, {required String at}) async {
    router = GoRouter(
      initialLocation: at,
      routes: [
        GoRoute(
          path: '/reset-password',
          builder: (_, state) =>
              ResetPasswordScreen(token: state.uri.queryParameters['token']),
        ),
        GoRoute(
          path: '/login',
          builder: (_, __) => const Scaffold(body: Text('login-screen')),
        ),
        GoRoute(
          path: '/forgot-password',
          builder: (_, __) => const Scaffold(body: Text('forgot-screen')),
        ),
      ],
    );
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
        child: localizedRouterApp(router),
      ),
    );
    await tester.pumpAndSettle();
  }

  Uri currentUri() => router.routerDelegate.currentConfiguration.uri;

  Future<void> submit(
    WidgetTester tester, {
    required String password,
    required String confirmation,
  }) async {
    final fields = find.byType(TextField);
    await tester.enterText(fields.first, password);
    await tester.enterText(fields.last, confirmation);
    await tester.tap(find.byType(ElevatedButton));
    await tester.pumpAndSettle();
  }

  group('M3 with ?token=abc', () {
    testWidgets('shows the two password fields and "Set new password"',
        (tester) async {
      await pumpReset(tester, at: '/reset-password?token=abc');

      expect(find.byType(TextField), findsNWidgets(2));
      expect(find.widgetWithText(ElevatedButton, l10n.setNewPassword),
          findsOneWidget);
      expect(find.textContaining(_invalidLink), findsNothing);
    });

    testWidgets('reads the token once and replaces the route without it',
        (tester) async {
      await pumpReset(tester, at: '/reset-password?token=abc');

      expect(currentUri().path, '/reset-password');
      expect(currentUri().queryParameters.containsKey('token'), isFalse,
          reason: 'the token must leave the address bar and history');
      expect(find.byType(TextField), findsNWidgets(2),
          reason: 'the form stays: the token was captured before the replace');
    });

    testWidgets('a mismatched confirmation blocks submit', (tester) async {
      await pumpReset(tester, at: '/reset-password?token=abc');

      await submit(tester, password: 'newpassword1', confirmation: 'other1');

      verifyNever(
        () => repo.resetPassword(
          token: any(named: 'token'),
          password: any(named: 'password'),
        ),
      );
      expect(find.text(l10n.passwordsDoNotMatch), findsOneWidget);
      expect(currentUri().path, '/reset-password');
    });

    testWidgets('a matching pair calls resetPassword(token: abc, ...)',
        (tester) async {
      when(
        () => repo.resetPassword(
          token: any(named: 'token'),
          password: any(named: 'password'),
        ),
      ).thenAnswer((_) async {});
      await pumpReset(tester, at: '/reset-password?token=abc');

      await submit(tester,
          password: 'newpassword1', confirmation: 'newpassword1');

      verify(() => repo.resetPassword(token: 'abc', password: 'newpassword1'))
          .called(1);
    });

    testWidgets('success shows "Password updated" and lands on /login',
        (tester) async {
      when(
        () => repo.resetPassword(
          token: any(named: 'token'),
          password: any(named: 'password'),
        ),
      ).thenAnswer((_) async {});
      await pumpReset(tester, at: '/reset-password?token=abc');

      await submit(tester,
          password: 'newpassword1', confirmation: 'newpassword1');

      expect(currentUri().path, '/login');
      expect(find.text('login-screen'), findsOneWidget);
      expect(find.text(l10n.passwordUpdated), findsOneWidget);
    });

    testWidgets('a transport error keeps the form and shows no raw exception',
        (tester) async {
      when(
        () => repo.resetPassword(
          token: any(named: 'token'),
          password: any(named: 'password'),
        ),
      ).thenThrow(const ApiException(ApiErrorKind.transport));
      await pumpReset(tester, at: '/reset-password?token=abc');

      await submit(tester,
          password: 'newpassword1', confirmation: 'newpassword1');

      expect(find.byType(TextField), findsNWidgets(2));
      expect(find.textContaining(_invalidLink), findsNothing);
      expect(find.text(l10n.apiErrorTransport), findsOneWidget);
      expect(find.textContaining('Exception'), findsNothing);
      expect(currentUri().path, '/reset-password');
    });
  });

  group('M4 link no longer valid', () {
    testWidgets(
        'a 401 replaces the form with the message and a new-link button',
        (tester) async {
      when(
        () => repo.resetPassword(
          token: any(named: 'token'),
          password: any(named: 'password'),
        ),
      ).thenThrow(
          const ApiException(ApiErrorKind.unauthorized, statusCode: 401));
      await pumpReset(tester, at: '/reset-password?token=abc');

      await submit(tester,
          password: 'newpassword1', confirmation: 'newpassword1');

      expect(find.byType(TextField), findsNothing);
      expect(find.textContaining(_invalidLink), findsOneWidget);
      expect(find.widgetWithText(ElevatedButton, l10n.requestANewLink),
          findsOneWidget);
      expect(currentUri().path, '/reset-password');
    });

    testWidgets('"Request a new link" navigates to /forgot-password',
        (tester) async {
      when(
        () => repo.resetPassword(
          token: any(named: 'token'),
          password: any(named: 'password'),
        ),
      ).thenThrow(
          const ApiException(ApiErrorKind.unauthorized, statusCode: 401));
      await pumpReset(tester, at: '/reset-password?token=abc');
      await submit(tester,
          password: 'newpassword1', confirmation: 'newpassword1');

      await tester.tap(find.text(l10n.requestANewLink));
      await tester.pumpAndSettle();

      expect(currentUri().path, '/forgot-password');
      expect(find.text('forgot-screen'), findsOneWidget);
    });

    testWidgets('no token in the URL shows the message straight away',
        (tester) async {
      await pumpReset(tester, at: '/reset-password');

      expect(find.byType(TextField), findsNothing);
      expect(find.textContaining(_invalidLink), findsOneWidget);
      expect(find.widgetWithText(ElevatedButton, l10n.requestANewLink),
          findsOneWidget);
      verifyNever(
        () => repo.resetPassword(
          token: any(named: 'token'),
          password: any(named: 'password'),
        ),
      );
    });
  });
}
