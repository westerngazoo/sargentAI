import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../auth/application/auth_controller.dart';
import '../auth/domain/auth_state.dart';
import '../auth/presentation/login_screen.dart';
import '../auth/presentation/register_screen.dart';
import '../profile/presentation/onboarding_screen.dart';
import '../shell/home_shell.dart';
import '../shell/splash_screen.dart';

/// The app router with the auth-gate (AC3/AC5/AC6/AC8).
///
/// The `GoRouter` is built ONCE — the provider body does not `watch` auth
/// (which would rebuild the router and discard navigation state, architect
/// Finding 1). Instead a [ValueNotifier], poked by `ref.listen`, is the
/// `refreshListenable`, so a logout/expiry re-runs `redirect` in place. The
/// `redirect` reads the sealed [AuthState] via an exhaustive `switch`, the
/// single source of truth (AC8).
final routerProvider = Provider<GoRouter>((ref) {
  final refresh = ValueNotifier<AuthState>(ref.read(authControllerProvider));
  ref.listen<AuthState>(
    authControllerProvider,
    (_, next) => refresh.value = next,
  );
  ref.onDispose(refresh.dispose);

  return GoRouter(
    initialLocation: '/home',
    refreshListenable: refresh,
    redirect: (context, state) {
      final loc = state.matchedLocation;
      return switch (ref.read(authControllerProvider)) {
        AuthUnknown() => loc == '/splash' ? null : '/splash',
        AuthUnauthenticated() =>
          (loc == '/login' || loc == '/register') ? null : '/login',
        AuthAuthenticated() =>
          (loc == '/login' || loc == '/register' || loc == '/splash')
              ? '/home'
              : null,
      };
    },
    routes: [
      GoRoute(path: '/splash', builder: (_, __) => const SplashScreen()),
      GoRoute(path: '/login', builder: (_, __) => const LoginScreen()),
      GoRoute(path: '/register', builder: (_, __) => const RegisterScreen()),
      GoRoute(path: '/home', builder: (_, __) => const HomeShell()),
      GoRoute(
        path: '/onboarding',
        builder: (_, __) => const OnboardingScreen(),
      ),
    ],
  );
});
