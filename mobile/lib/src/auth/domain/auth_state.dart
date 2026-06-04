import 'session.dart';

/// The single source of truth for the session, switched on by the router's
/// auth-gate (SPEC-0007 §2.4, AC8). Sealed so the gate's `switch` is total — a
/// new variant cannot compile without being handled everywhere.
sealed class AuthState {
  const AuthState();
}

/// Cold-start: secure storage has not been read yet. Drives the splash and
/// prevents a login-screen flash before the stored token is checked (AC3).
class AuthUnknown extends AuthState {
  const AuthUnknown();
}

/// No valid session — the router routes to `/login`.
class AuthUnauthenticated extends AuthState {
  const AuthUnauthenticated();
}

/// A token is held — the router routes to `/home`.
class AuthAuthenticated extends AuthState {
  const AuthAuthenticated(this.session);

  final Session session;
}
