import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/storage/token_store.dart';
import '../data/auth_repository.dart';
import '../domain/auth_state.dart';
import '../domain/session.dart';

final authControllerProvider =
    NotifierProvider<AuthController, AuthState>(AuthController.new);

/// The authenticated user's id, or `null` outside a session. User-scoped data
/// providers watch this so an account switch (logout → different login) drops
/// their caches instead of leaking the previous user's data.
final authUserIdProvider = Provider<String?>(
  (ref) => switch (ref.watch(authControllerProvider)) {
    AuthAuthenticated(:final session) => session.userId,
    _ => null,
  },
);

/// The session state machine and the single source of truth for "am I logged
/// in?" (SPEC-0007 §2.5, AC8). `build` starts in [AuthUnknown] and kicks off a
/// restore from secure storage; the token is also cached in memory for the
/// synchronous `AuthInterceptor` (OQ-D3).
class AuthController extends Notifier<AuthState> {
  AuthToken? _token;

  /// Read by `AuthInterceptor.onRequest` to attach the Bearer header (AC4).
  AuthToken? get token => _token;

  @override
  AuthState build() {
    Future.microtask(_restore);
    return const AuthUnknown();
  }

  Future<void> _restore() async {
    try {
      final stored = await ref.read(tokenStoreProvider).read();
      // A login/logout may have already resolved the session while the async
      // read was in flight — never clobber it (restore only resolves the
      // initial AuthUnknown).
      if (state is! AuthUnknown) return;
      if (stored == null) {
        state = const AuthUnauthenticated();
      } else {
        _token = stored;
        state = AuthAuthenticated(Session(userId: stored.userId));
      }
    } catch (_) {
      // A corrupt/failed secure-storage read must not strand the app in the
      // splash state (architect Finding 2): clear and fall back to login.
      if (state is! AuthUnknown) return;
      await ref.read(tokenStoreProvider).clear();
      state = const AuthUnauthenticated();
    }
  }

  /// AC1: register, then auto-login (register returns no token). A register
  /// failure rethrows and never auto-logs-in.
  Future<void> register(String email, String password) async {
    await ref.read(authRepositoryProvider).register(email, password);
    await login(email, password);
  }

  /// AC2: login → cache token → authenticated. Rethrows [ApiException] on
  /// failure and leaves state untouched.
  Future<void> login(String email, String password) async {
    final token = await ref.read(authRepositoryProvider).login(email, password);
    _token = token;
    state = AuthAuthenticated(Session(userId: token.userId));
  }

  /// Google Sign-In (R-0033): same token handoff as password login.
  Future<void> loginWithGoogle(String idToken) async {
    final token =
        await ref.read(authRepositoryProvider).loginWithGoogle(idToken);
    _token = token;
    state = AuthAuthenticated(Session(userId: token.userId));
  }

  /// AC5/AC6: clear the in-memory token + persisted token and go
  /// unauthenticated. Called by the logout action AND the 401 sink.
  Future<void> logout() async {
    _token = null;
    await ref.read(authRepositoryProvider).clear();
    state = const AuthUnauthenticated();
  }
}
