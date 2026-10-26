import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../auth/application/auth_controller.dart';
import '../auth/domain/auth_state.dart';
import '../auth/presentation/forgot_password_screen.dart';
import '../auth/presentation/login_screen.dart';
import '../auth/presentation/register_screen.dart';
import '../auth/presentation/reset_password_screen.dart';
import '../profile/presentation/onboarding_screen.dart';
import '../home/main_screen.dart';
import '../shell/splash_screen.dart';
import '../workout/application/session_driver.dart';
import '../workout/presentation/live_session_screen.dart';

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
      // The reset link from an email is always a cold start (AuthUnknown).
      // Sending it to /splash would discard its token, so the two
      // password-recovery screens pass through every arm that can open them
      // (SPEC-0060 §2.7).
      final recovering = loc == '/forgot-password' || loc == '/reset-password';
      return switch (ref.read(authControllerProvider)) {
        AuthUnknown() => (loc == '/splash' || recovering) ? null : '/splash',
        AuthUnauthenticated() =>
          (loc == '/login' || loc == '/register' || recovering)
              ? null
              : '/login',
        AuthAuthenticated() => (loc == '/login' ||
                loc == '/register' ||
                loc == '/splash' ||
                loc == '/forgot-password' ||
                // The screens that became panels, and the hub that became
                // the Dictado sheet (SPEC-0061 §2.5, §2.8): an old URL lands
                // on the day.
                loc == '/hub' ||
                loc == '/progress' ||
                loc == '/programs/current' ||
                loc == '/programs/get' ||
                // A cold start at /session has no draft: the session lives in
                // memory, so a reload lands on the day (SPEC-0061 §2.5).
                (loc == '/session' &&
                    ref.read(sessionDriverProvider).draft == null))
            ? '/home'
            : null,
      };
    },
    routes: [
      GoRoute(path: '/splash', builder: (_, __) => const SplashScreen()),
      GoRoute(path: '/login', builder: (_, __) => const LoginScreen()),
      GoRoute(path: '/register', builder: (_, __) => const RegisterScreen()),
      GoRoute(
        path: '/forgot-password',
        builder: (_, __) => const ForgotPasswordScreen(),
      ),
      GoRoute(
        path: '/reset-password',
        builder: (_, state) =>
            ResetPasswordScreen(token: state.uri.queryParameters['token']),
      ),
      GoRoute(path: '/home', builder: (_, __) => const MainScreen()),
      GoRoute(
        path: '/onboarding',
        builder: (_, __) => const OnboardingScreen(),
      ),
      GoRoute(
        path: '/session',
        builder: (_, __) => const LiveSessionScreen(),
      ),
    ],
  );
});
