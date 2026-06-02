// AuthController — the single source of truth for session state (SAC8).
//   SAC1: register() -> auto-login() -> AuthAuthenticated.
//   SAC2: login() -> token cached + AuthAuthenticated.
//   SAC3: build()/_restore() resolves AuthUnknown -> {Authenticated|Unauthenticated},
//         and a TokenStore.read THROW clears the entry and falls back to
//         AuthUnauthenticated (architect Finding 2).
//   SAC5/SAC6: logout() clears the in-memory token + repo + state.
//   AC9: a failed login rethrows ApiException and leaves state untouched.
//
// RED until package:fitai/src/auth/application/auth_controller.dart defines
// authControllerProvider (NotifierProvider<AuthController, AuthState>) with
// register/login/logout + the in-memory `token` getter and `_restore` on build.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/auth/domain/auth_state.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';

void main() {
  setUpAll(registerFallbacks);

  late MockTokenStore tokenStore;
  late MockAuthRepository repo;

  ProviderContainer makeContainer() {
    final container = ProviderContainer(
      overrides: [
        tokenStoreProvider.overrideWithValue(tokenStore),
        authRepositoryProvider.overrideWithValue(repo),
      ],
    );
    addTearDown(container.dispose);
    return container;
  }

  setUp(() {
    tokenStore = MockTokenStore();
    repo = MockAuthRepository();
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
  });

  group('SAC3 cold-start restore', () {
    test('build() begins in AuthUnknown', () {
      when(() => tokenStore.read()).thenAnswer((_) async => null);
      final container = makeContainer();
      // Read synchronously before the microtask restore runs.
      expect(container.read(authControllerProvider), isA<AuthUnknown>());
    });

    test('a present token resolves to AuthAuthenticated', () async {
      when(() => tokenStore.read())
          .thenAnswer((_) async => sampleToken(userId: 'u-cold'));
      final container = makeContainer();
      container.read(authControllerProvider);
      await pumpEventQueue();

      final state = container.read(authControllerProvider);
      expect(state, isA<AuthAuthenticated>());
      expect((state as AuthAuthenticated).session.userId, 'u-cold');
      expect(container.read(authControllerProvider.notifier).token, isNotNull);
    });

    test('no token resolves to AuthUnauthenticated', () async {
      when(() => tokenStore.read()).thenAnswer((_) async => null);
      final container = makeContainer();
      container.read(authControllerProvider);
      await pumpEventQueue();
      expect(
        container.read(authControllerProvider),
        isA<AuthUnauthenticated>(),
      );
    });

    test('a THROWING read clears the entry and falls back to unauthenticated',
        () async {
      when(() => tokenStore.read()).thenThrow(Exception('corrupt entry'));
      final container = makeContainer();
      container.read(authControllerProvider);
      await pumpEventQueue();

      verify(() => tokenStore.clear()).called(1);
      expect(
        container.read(authControllerProvider),
        isA<AuthUnauthenticated>(),
      );
    });
  });

  group('SAC2 login', () {
    setUp(() => when(() => tokenStore.read()).thenAnswer((_) async => null));

    test('successful login caches the token and authenticates', () async {
      when(() => repo.login('a@b.com', 'pw'))
          .thenAnswer((_) async => sampleToken(userId: 'u-login'));
      final container = makeContainer();
      final controller = container.read(authControllerProvider.notifier);

      await controller.login('a@b.com', 'pw');

      final state = container.read(authControllerProvider);
      expect(state, isA<AuthAuthenticated>());
      expect((state as AuthAuthenticated).session.userId, 'u-login');
      expect(controller.token, isNotNull);
    });

    test('a failed login rethrows ApiException and keeps state put', () async {
      when(() => repo.login(any(), any()))
          .thenThrow(const ApiException('invalid email or password',
              statusCode: 401));
      final container = makeContainer();
      final controller = container.read(authControllerProvider.notifier);
      container.read(authControllerProvider);
      await pumpEventQueue();

      await expectLater(
        controller.login('a@b.com', 'bad'),
        throwsA(isApiExceptionWithStatus(401)),
      );
      expect(
        container.read(authControllerProvider),
        isA<AuthUnauthenticated>(),
      );
      expect(controller.token, isNull);
    });
  });

  group('SAC1 register auto-login', () {
    setUp(() => when(() => tokenStore.read()).thenAnswer((_) async => null));

    test('register then auto-login lands AuthAuthenticated', () async {
      when(() => repo.register('new@b.com', 'pw')).thenAnswer((_) async {});
      when(() => repo.login('new@b.com', 'pw'))
          .thenAnswer((_) async => sampleToken(userId: 'u-new'));
      final container = makeContainer();
      final controller = container.read(authControllerProvider.notifier);

      await controller.register('new@b.com', 'pw');

      verify(() => repo.register('new@b.com', 'pw')).called(1);
      verify(() => repo.login('new@b.com', 'pw')).called(1);
      expect(
        container.read(authControllerProvider),
        isA<AuthAuthenticated>(),
      );
    });

    test('a register failure rethrows and never auto-logs-in', () async {
      when(() => repo.register(any(), any()))
          .thenThrow(const ApiException('duplicate email', statusCode: 409));
      final container = makeContainer();
      final controller = container.read(authControllerProvider.notifier);
      container.read(authControllerProvider);
      await pumpEventQueue();

      await expectLater(
        controller.register('dup@b.com', 'pw'),
        throwsA(isApiExceptionWithStatus(409)),
      );
      verifyNever(() => repo.login(any(), any()));
    });
  });

  group('SAC5/SAC6 logout', () {
    test('logout clears the token, repo, and sets unauthenticated', () async {
      when(() => tokenStore.read()).thenAnswer((_) async => null);
      when(() => repo.login(any(), any()))
          .thenAnswer((_) async => sampleToken());
      final container = makeContainer();
      final controller = container.read(authControllerProvider.notifier);
      await controller.login('a@b.com', 'pw');
      expect(container.read(authControllerProvider), isA<AuthAuthenticated>());

      await controller.logout();

      verify(() => repo.clear()).called(1);
      expect(controller.token, isNull);
      expect(
        container.read(authControllerProvider),
        isA<AuthUnauthenticated>(),
      );
    });
  });
}
