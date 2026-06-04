// SAC4 -> AC4: AuthInterceptor.onRequest attaches `Authorization: Bearer <jwt>`
//   when authenticated, and attaches nothing pre-login.
// SAC5 -> AC5: a 401 on an authed call triggers AuthController.logout(); the
//   `/auth/login` and `/auth/register` paths are EXEMPT from the 401 sink.
//
// RED until package:fitai/src/core/network/dio_provider.dart defines
// AuthInterceptor (constructed with a Riverpod Ref, per SPEC-0007 §2.6) and
// package:fitai/src/auth/application/auth_controller.dart defines
// authControllerProvider with an in-memory `token` getter + logout().

import 'package:dio/dio.dart';
import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/auth/domain/auth_state.dart';
import 'package:fitai/src/core/network/dio_provider.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';

/// Exposes a live AuthInterceptor built with a real Ref (SPEC-0007 §2.6 builds
/// `AuthInterceptor(ref)` inside `dioProvider`; this mirrors that construction
/// so the test drives the exact production seam).
final _interceptorProbe = Provider<AuthInterceptor>(AuthInterceptor.new);

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
    when(() => tokenStore.read()).thenAnswer((_) async => null);
    when(() => tokenStore.clear()).thenAnswer((_) async {});
    when(() => repo.clear()).thenAnswer((_) async {});
  });

  group('SAC4 bearer attach', () {
    test('onRequest attaches Bearer header when a token is held', () async {
      final container = makeContainer();
      final controller = container.read(authControllerProvider.notifier);
      when(() => repo.login(any(), any()))
          .thenAnswer((_) async => sampleToken(jwt: 'live.jwt'));
      await controller.login('a@b.com', 'pw');
      expect(container.read(authControllerProvider), isA<AuthAuthenticated>());

      final interceptor = container.read(_interceptorProbe);
      final options = RequestOptions(path: '/auth/me');
      interceptor.onRequest(options, RequestInterceptorHandler());

      expect(options.headers['Authorization'], 'Bearer live.jwt');
    });

    test('onRequest attaches no Authorization header before login', () async {
      final container = makeContainer();
      container.read(authControllerProvider);
      await pumpEventQueue();

      final interceptor = container.read(_interceptorProbe);
      final options = RequestOptions(path: '/auth/me');
      interceptor.onRequest(options, RequestInterceptorHandler());

      expect(options.headers.containsKey('Authorization'), isFalse);
    });
  });

  group('SAC5 401 sink', () {
    test('a 401 on an authed call logs the user out', () async {
      final container = makeContainer();
      final controller = container.read(authControllerProvider.notifier);
      when(() => repo.login(any(), any()))
          .thenAnswer((_) async => sampleToken());
      await controller.login('a@b.com', 'pw');
      expect(container.read(authControllerProvider), isA<AuthAuthenticated>());

      final interceptor = container.read(_interceptorProbe);
      interceptor.onError(
        dioError(401, path: '/auth/me'),
        ErrorInterceptorHandler(),
      );
      await pumpEventQueue();

      verify(() => repo.clear()).called(1);
      expect(controller.token, isNull);
      expect(
        container.read(authControllerProvider),
        isA<AuthUnauthenticated>(),
      );
    });

    test('a 401 on /auth/login is EXEMPT (no logout)', () async {
      final container = makeContainer();
      final controller = container.read(authControllerProvider.notifier);
      when(() => repo.login(any(), any()))
          .thenAnswer((_) async => sampleToken());
      await controller.login('a@b.com', 'pw');

      final interceptor = container.read(_interceptorProbe);
      interceptor.onError(
        dioError(401, path: '/auth/login'),
        ErrorInterceptorHandler(),
      );
      await pumpEventQueue();

      verifyNever(() => repo.clear());
      expect(
        container.read(authControllerProvider),
        isA<AuthAuthenticated>(),
      );
    });

    test('a 401 on /auth/register is EXEMPT (no logout)', () async {
      final container = makeContainer();
      final controller = container.read(authControllerProvider.notifier);
      when(() => repo.login(any(), any()))
          .thenAnswer((_) async => sampleToken());
      await controller.login('a@b.com', 'pw');

      final interceptor = container.read(_interceptorProbe);
      interceptor.onError(
        dioError(401, path: '/auth/register'),
        ErrorInterceptorHandler(),
      );
      await pumpEventQueue();

      verifyNever(() => repo.clear());
    });
  });
}
