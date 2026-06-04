# SPEC-0007 — Flutter app architecture & auth shell

- **Status:** Implemented
- **Realizes:** R-0007
- **Author:** Claude (main session), with owner
- **Created:** 2026-06-02
- **Depends on:** SPEC-0001 (Implemented — `/mobile` scaffold, Flutter 3.44.0 / Dart 3.12.0 pin, `flutter analyze`/`test` CI), SPEC-0002 (Implemented — `/auth/register`, `/auth/login`, `/auth/me`, the `{token,user_id,expires_at}` login shape)
- **Module(s):** `mobile/` (Flutter app — first feature build-out: `lib/src/**`, `pubspec.yaml`, `test/**`, `integration_test/**`)

## 1. Motivation

Realizes [R-0007](../requirements/0007-flutter-app-shell.md): turn `/mobile` from
the R-0001 hello-world (one `HomeScreen`, zero non-SDK deps) into a real
authenticated client skeleton — **register → login → JWT in secure storage →
authenticated home shell** — plus the cross-cutting architecture every later M3
screen inherits: Riverpod state, a configurable HTTP client, a router with an
auth-gate, and one uniform loading/error pattern. It ships **no feature logger
UI** (R-0008+). This is the project's first mobile feature work, so the idioms
chosen here become the pattern for onboarding, the workout/nutrition loggers, and
the dashboard.

Three owner decisions are settled (R-0007 §6) and drive this design:
**(1)** the shell is its own requirement; **(2)** state management is
**Riverpod**; **(3)** token expiry is handled by **re-login on `401`** with the
JWT in platform secure storage (no refresh — the backend has no refresh
endpoint).

## 2. Design

### 2.1 Layering (feature-first, presentation depends inward)

Idiomatic Flutter layering that mirrors the backend's parse-don't-validate
discipline: **presentation → application → data → core**, dependencies pointing
inward. Riverpod providers are the seams.

```
lib/
  main.dart                         # runApp(ProviderScope(child: FitAiApp()))
  app.dart                          # MaterialApp.router bound to routerProvider
  src/
    core/
      config/app_config.dart        # API_BASE_URL (compile-time --dart-define)
      network/
        dio_provider.dart           # Dio + AuthInterceptor (bearer attach, 401 sink)
        api_exception.dart          # typed error model (status → field/message)
      storage/
        token_store.dart            # FlutterSecureStorage wrapper: read/write/clear
    auth/
      domain/
        auth_state.dart             # sealed AuthState: unknown | unauthenticated | authenticated
        session.dart                # value types: AuthToken, Session{userId}
      data/
        auth_api.dart               # register/login/me HTTP calls (typed)
        auth_repository.dart        # api + token_store orchestration
      application/
        auth_controller.dart        # Notifier<AuthState>: restore/register/login/logout
      presentation/
        login_screen.dart
        register_screen.dart
        auth_form.dart              # shared email/password form widget
    shell/
      home_shell.dart               # authenticated placeholder: shows user + logout
    router/
      app_router.dart               # routerProvider (GoRouter) with auth redirect
```

The R-0001 `lib/screens/home_screen.dart` placeholder and its widget test are
**removed**; the home shell (`src/shell/home_shell.dart`) supersedes them.

### 2.2 Dependencies (pubspec.yaml)

| Package | Version (compat. w/ Dart 3.12) | Role |
|---------|-------------------------------|------|
| `flutter_riverpod` | `^2.6.1` | state management (owner decision) |
| `go_router` | `^14.6.0` | declarative routing + redirect auth-gate (OQ-D2) |
| `dio` | `^5.7.0` | HTTP client; interceptors make bearer-attach + 401-sink clean (OQ-D1) |
| `flutter_secure_storage` | `^9.2.2` | JWT in Keychain/Keystore (owner decision) |
| **dev:** `mocktail` | `^1.0.4` | mock `AuthApi`/`Dio` in widget + integration tests |
| **dev:** `integration_test` | SDK | `login → home` e2e — **authored + compile-checked** now; gate execution deferred to R-0025 (needs platform folders — OQ-D4) |

No `freezed`/codegen in this spec — the sealed `AuthState` and small DTOs are
hand-written to keep the first build dependency-light (revisit if DTOs multiply
in R-0008+).

### 2.3 Configuration — API base URL (AC7)

```dart
class AppConfig {
  static const apiBaseUrl = String.fromEnvironment(
    'API_BASE_URL',
    defaultValue: 'http://localhost:8080',
  );
}
```

`flutter run --dart-define=API_BASE_URL=https://api.example.com` overrides it;
the default targets the local backend. No production URL is compiled in. Flavors
are deferred to M8 (R-0007 §4).

### 2.4 Auth state machine (AC3/AC8)

A single sealed `AuthState` is the **one source of truth** the router reads:

```dart
sealed class AuthState {
  const AuthState();
}
class AuthUnknown extends AuthState { const AuthUnknown(); }          // pre-storage-read (splash)
class AuthUnauthenticated extends AuthState { const AuthUnauthenticated(); }
class AuthAuthenticated extends AuthState {                            // token present
  final Session session;
  const AuthAuthenticated(this.session);
}
```

`AuthUnknown` is the cold-start state while secure storage is read; the router
shows a splash for it, preventing a login-screen flash before the stored token
is checked (AC3).

### 2.5 AuthController (application — AC1/AC2/AC5/AC6)

A Riverpod `Notifier<AuthState>`; `build()` returns `AuthUnknown` and kicks off
`_restore()`. The token is held **in memory** on the controller for synchronous
interceptor access, hydrated from secure storage at restore and on login.

```dart
final authControllerProvider =
    NotifierProvider<AuthController, AuthState>(AuthController.new);

class AuthController extends Notifier<AuthState> {
  AuthToken? _token;                         // in-memory cache for the interceptor

  @override
  AuthState build() {
    Future.microtask(_restore);
    return const AuthUnknown();
  }

  Future<void> _restore() async {
    try {
      final stored = await ref.read(tokenStoreProvider).read();
      if (stored == null) {
        state = const AuthUnauthenticated();
      } else {
        // Optimistic: a present token → AuthAuthenticated. If it is expired,
        // the HomeShell's `GET /auth/me` 401s and the AuthInterceptor sink
        // logs out (the liveness check — architect Finding 3 / SAC5).
        _token = stored;
        state = AuthAuthenticated(Session(userId: stored.userId));
      }
    } catch (_) {
      // Corrupt entry / platform error must not strand the app in the splash
      // (architect Finding 2): clear the bad entry and fall back to login.
      await ref.read(tokenStoreProvider).clear();
      state = const AuthUnauthenticated();
    }
  }

  AuthToken? get token => _token;            // read by AuthInterceptor.onRequest

  /// AC1: register then auto-login (register returns no token).
  Future<void> register(String email, String password) async {
    await ref.read(authRepositoryProvider).register(email, password);
    await login(email, password);
  }

  /// AC2: login → persist JWT → authenticated.
  Future<void> login(String email, String password) async {
    final session = await ref.read(authRepositoryProvider).login(email, password);
    _token = session.token;
    state = AuthAuthenticated(Session(userId: session.token.userId));
  }

  /// AC5/AC6: clear token → unauthenticated (called by logout AND the 401 sink).
  Future<void> logout() async {
    _token = null;
    await ref.read(authRepositoryProvider).clear();
    state = const AuthUnauthenticated();
  }
}
```

`register`/`login` rethrow `ApiException` on failure so the screens render a
message (AC9); state stays put on failure (no partial auth).

### 2.6 HTTP layer & the 401 sink (AC4/AC5)

A single `Dio` provider installs an `AuthInterceptor`:

- **`onRequest`** — attach `Authorization: Bearer <token>` when
  `authController.token != null` (AC4).
- **`onError`** — when the response status is `401`, call
  `authController.logout()` (clears token, sets `AuthUnauthenticated`); the
  router redirect then routes to login (AC5). The error still propagates so the
  caller can stop. To avoid a logout loop, the `/auth/login` and `/auth/register`
  calls are exempt from the 401 sink (their 401 is "bad credentials", surfaced as
  a message, not a session-expiry).

```dart
final dioProvider = Provider<Dio>((ref) {
  final dio = Dio(BaseOptions(
    baseUrl: AppConfig.apiBaseUrl,
    connectTimeout: const Duration(seconds: 5),
    receiveTimeout: const Duration(seconds: 5),
  ));
  dio.interceptors.add(AuthInterceptor(ref));
  return dio;
});
```

`ApiException` maps transport/status to a typed, user-safe error (AC9):

```dart
class ApiException implements Exception {
  final int? statusCode;     // null = transport/timeout (no response)
  final String? field;       // from the backend {"error":{"field":"…"}} body, if present
  final String message;      // user-safe; never a raw stack trace
  const ApiException(this.message, {this.statusCode, this.field});
}
```

`AuthApi` translates `DioException` → `ApiException`: `400` → field-aware
validation message; `401` → "invalid email or password" (login) or session-expiry
(authed calls); no response / timeout → "can't reach the server — retry".

### 2.7 Router & auth-gate (AC3/AC5/AC6/AC8)

`go_router` with a `redirect` that reads the sealed `AuthState`. The router is
built **once** (the provider body does not `watch` auth — that would rebuild the
whole `GoRouter` on every state change and discard navigation state, architect
Finding 1); instead a `ValueNotifier` driven by `ref.listen` is the
`refreshListenable`, so a logout/expiry re-evaluates redirects in place.

```dart
final routerProvider = Provider<GoRouter>((ref) {
  // One ValueNotifier pokes the router to re-run `redirect` on auth changes;
  // the router instance itself is constructed once (no `watch` in the body).
  final refresh = ValueNotifier<AuthState>(ref.read(authControllerProvider));
  ref.listen(authControllerProvider, (_, next) => refresh.value = next);
  ref.onDispose(refresh.dispose);

  return GoRouter(
    initialLocation: '/home',
    refreshListenable: refresh,
    redirect: (context, state) {
      final loc = state.matchedLocation;
      return switch (ref.read(authControllerProvider)) {
        AuthUnknown()         => loc == '/splash' ? null : '/splash',
        AuthUnauthenticated() => (loc == '/login' || loc == '/register') ? null : '/login',
        AuthAuthenticated()   => (loc == '/login' || loc == '/register' || loc == '/splash') ? '/home' : null,
      };
    },
    routes: [
      GoRoute(path: '/splash',   builder: (_, __) => const SplashScreen()),
      GoRoute(path: '/login',    builder: (_, __) => const LoginScreen()),
      GoRoute(path: '/register', builder: (_, __) => const RegisterScreen()),
      GoRoute(path: '/home',     builder: (_, __) => const HomeShell()),
    ],
  );
});
```

The exhaustive `switch` over the sealed `AuthState` makes the gate total — a new
state can't compile without handling it here.

### 2.8 Screens (AC1/AC2/AC9/AC11)

- **LoginScreen / RegisterScreen** — a shared `AuthForm` (email + password,
  client-side non-empty/format checks), a submit button that shows a spinner and
  is disabled while a call is in flight (AC9, no double-submit), and an inline
  error area fed by the caught `ApiException`. On success the controller's state
  change drives the redirect (no imperative navigation).
- **HomeShell** — the only screen beyond auth (AC11): an `AppBar` titled "fitAI",
  the current user (`GET /auth/me`) shown via a `FutureProvider`/`AsyncValue`
  (loading/error/data), and a **Logout** action calling `authController.logout()`.
  It is a deliberate placeholder; R-0008+ replace its body with real navigation.

## 3. Code outline

Representative shapes (final form reconciled in step 5 against Flutter 3.44.0 /
Dart 3.12.0). Tests are authored by `qa` in step 3 against §6.

### 3.1 `token_store.dart`

```dart
class AuthToken {
  final String jwt;
  final String userId;       // from the login response `user_id` (no JWT decode)
  final DateTime expiresAt;  // from login response (advisory; expiry enforced server-side)
  const AuthToken({required this.jwt, required this.userId, required this.expiresAt});
}

final tokenStoreProvider = Provider<TokenStore>((_) => TokenStore(const FlutterSecureStorage()));

class TokenStore {
  static const _k = 'fitai.jwt';
  final FlutterSecureStorage _storage;
  const TokenStore(this._storage);

  Future<AuthToken?> read() async { /* read _k, JSON-decode, or null */ }
  Future<void> write(AuthToken token) async { /* JSON-encode → _storage.write */ }
  Future<void> clear() async => _storage.delete(key: _k);
}
```

### 3.2 `auth_api.dart` (typed calls; maps the R-0002 wire shapes)

```dart
class AuthApi {
  final Dio _dio;
  const AuthApi(this._dio);

  /// POST /auth/register → 201 {user_id}. 400/409 → ApiException.
  Future<void> register(String email, String password) async { /* … */ }

  /// POST /auth/login → 200 {token,user_id,expires_at}. 401 → ApiException.
  Future<AuthToken> login(String email, String password) async { /* … */ }

  /// GET /auth/me → 200 {user_id}. (authed; 401 handled by interceptor)
  Future<String> me() async { /* returns user_id */ }
}
```

### 3.3 `main.dart` / `app.dart` (replacing the R-0001 placeholder)

```dart
// main.dart
void main() => runApp(const ProviderScope(child: FitAiApp()));

// app.dart
class FitAiApp extends ConsumerWidget {
  const FitAiApp({super.key});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return MaterialApp.router(
      title: 'fitAI',
      routerConfig: ref.watch(routerProvider),
    );
  }
}
```

## 4. Non-goals

- **No feature logger / dashboard UI** — onboarding (R-0008), workout (R-0009),
  nutrition (R-0010), dashboard (R-0011); photo capture re-homed on R-0006.
- **No token refresh / silent re-auth** — re-login on `401` only.
- **No OAuth/social login, no biometric unlock.**
- **No offline cache / sync queue** — online only.
- **No codegen (`freezed`/`json_serializable`)** in this spec — hand-written DTOs.
- **No release signing / store metadata / platform-folder device builds** beyond
  what `integration_test` needs (R-0025/R-0026).
- **No theming system / design tokens** beyond default Material 3 — visual polish
  is deferred; the shell is functional, not finished.

## 5. Open questions

HOW-level questions deferred from R-0007 §5. **All five were resolved by the
architect review (2026-06-02, APPROVE WITH NITS) — every proposed answer
confirmed.** Folded into §2/§3 above; status is now `Accepted`.

- **OQ-D1 — HTTP client. RESOLVED → `dio`.** Its interceptor model expresses the
  bearer-attach (AC4) and the 401-sink (AC5) in one seam; a hand-rolled `http`
  wrapper would re-implement it worse.
- **OQ-D2 — Router. RESOLVED → `go_router`.** Declarative `redirect` is the
  natural home for the auth-gate; the redundant `watch`+`refreshListenable`
  ambiguity flagged by the architect (Finding 1) is fixed in §2.7 — the router is
  built once and a `ValueNotifier` driven by `ref.listen` is the sole refresh
  mechanism.
- **OQ-D3 — Token source for the interceptor. RESOLVED → in-memory**
  (`AuthController._token`, hydrated at restore/login). Synchronous `onRequest`,
  no Keychain read per call; secure storage stays the cross-start source of truth.
- **OQ-D4 — `integration_test` without a device. RESOLVED → author +
  compile-check the e2e now; defer gate execution to R-0025.** The file is
  written against a mocked `Dio` and kept honest by `flutter analyze`, but it is
  **run by no gate**: `flutter test` (and the CI mobile job) execute `test/`
  only, and `flutter test integration_test/` routes to a device/host target that
  needs the `android/`/`ios/` folders R-0007 deliberately does not create
  (committing an unsigned, untested device surface is R-0025 scope, §4). The
  `login → home` capability is covered in the meantime by the `test/` widget
  suites (login + router gate + home shell). *(Corrected 2026-06-03 — the
  original resolution wrongly assumed the flutter-tester path needs no platform
  folders; qa step-7 found it does. See Changelog.)*
- **OQ-D5 — `AuthController` shape. RESOLVED → `Notifier<AuthState>`** with an
  explicit `AuthUnknown` splash state (not `AsyncNotifier<AuthState>`). The sealed
  state already models loading; the router's exhaustive `switch` reads cleaner
  over a concrete union. `Future.microtask(_restore)` in `build()` is the
  established idiom, paired with the restore error-handling from Finding 2 (§2.5).

## 6. Acceptance criteria

Each maps 1:1 to an R-0007 acceptance criterion and to the qa agent's test.

- [ ] **SAC1 → AC1.** Register screen calls `POST /auth/register`; on `201`
  auto-logs-in (`POST /auth/login`) and lands on `/home`. A `400` (bad email /
  weak password) and a `409` (duplicate email) each render a readable inline
  message and leave the user on the register screen. Widget test with a mocked
  `AuthApi` pins all three outcomes.
- [ ] **SAC2 → AC2.** Login screen calls `POST /auth/login`; on `200` the
  `AuthToken` is written to `TokenStore` (secure storage) and state →
  `AuthAuthenticated`, routing to `/home`. A `401` renders a non-enumerating
  "invalid email or password" message. Tests assert `TokenStore.write` is called
  and the 401 message.
- [ ] **SAC3 → AC3.** With a token present in `TokenStore`, app start resolves
  `AuthUnknown → AuthAuthenticated` and the router lands on `/home` (no login
  flash); with none, it lands on `/login`. A `TokenStore.read` that **throws**
  (corrupt entry / platform error) must not strand the app in splash — it clears
  the entry and resolves to `/login` (architect Finding 2). Test overrides
  `TokenStore.read` for all three branches.
- [ ] **SAC4 → AC4.** `AuthInterceptor.onRequest` attaches
  `Authorization: Bearer <jwt>` when authenticated; `HomeShell` renders the
  `user_id` from `GET /auth/me`. Test asserts the header is present on an authed
  call and absent pre-login.
- [ ] **SAC5 → AC5.** An authed request returning `401` triggers
  `AuthController.logout()` (token cleared, state `AuthUnauthenticated`) and the
  router redirects to `/login`; `/auth/login` and `/auth/register` are exempt
  from the sink. Includes the **restored-but-expired-token** path (architect
  Finding 3): cold start with a stale JWT resolves optimistically to
  `AuthAuthenticated`, then the home shell's `GET /auth/me` 401s → sink → `/login`.
  Test drives a 401 on `me()` (both fresh-session and cold-start variants) and
  asserts logout + redirect.
- [ ] **SAC6 → AC6.** The logout action clears `TokenStore` and routes to
  `/login`; a simulated cold start afterward resolves to `/login`. Test asserts
  `TokenStore.clear` and the post-logout route.
- [ ] **SAC7 → AC7.** `AppConfig.apiBaseUrl` reads `String.fromEnvironment`
  with the `http://localhost:8080` default and no compiled-in production URL.
  Test asserts the default and that `Dio.options.baseUrl` honours it.
- [ ] **SAC8 → AC8.** `authControllerProvider` is the sole session source; the
  router `redirect` reads it via an exhaustive `switch` over the sealed
  `AuthState` (no duplicated auth checks). Verified by inspection + the gate tests.
- [ ] **SAC9 → AC9.** No raw exception reaches the UI: `ApiException` is the only
  error type screens render; a transport/timeout error shows a retryable message;
  the submit button shows a spinner and is disabled in-flight (no double-submit).
  Tests pin the timeout message and the disabled-while-loading state.
- [ ] **SAC10 → AC10.** Widget tests cover login, register, and the auth gate and
  together exercise the full `login → home` path (login → router gate → home
  shell). An `integration_test` driving `login → home` against a mocked `Dio` is
  authored and compile-checked (via `flutter analyze`); **running** it in a gate
  is deferred to R-0025 — it needs platform folders R-0007 does not create
  (OQ-D4). `flutter analyze`, `dart format --set-exit-if-changed .`, and
  `flutter test` (the `test/` unit + widget suite) are all green.
- [ ] **SAC11 → AC11.** No workout/nutrition/photo/dashboard UI exists; the only
  screens are splash, login, register, and the placeholder `HomeShell`
  (user + logout). Verified by the `lib/src` tree + a route-table assertion.

## 7. Decision log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-02 | **Feature-first layering (`presentation → application → data → core`) under `lib/src`, Riverpod providers as seams.** | Mirrors the backend's inward-pointing dependency rule; keeps each later logger a thin slice over the same core. |
| 2026-06-02 | **Sealed `AuthState` (unknown/unauthenticated/authenticated) as the single source of truth; router gates on an exhaustive `switch`.** | Total, compile-checked auth-gate; `AuthUnknown` prevents a login flash on cold start (AC3). |
| 2026-06-02 | **Token held in memory on `AuthController`, hydrated from secure storage; storage is the cross-start source of truth.** | Synchronous interceptor access without a Keychain read per request (OQ-D3). |
| 2026-06-02 | **401-sink exempts `/auth/login` and `/auth/register`.** | Their 401 is "bad credentials" (a message), not session expiry — prevents a logout loop (AC5). |
| 2026-06-02 | **API base URL via `--dart-define`, dev default `http://localhost:8080`; no flavors yet.** | AC7; flavors are an M8 concern. |
| 2026-06-02 | **R-0001 `home_screen.dart` placeholder + its test removed, superseded by `src/shell/home_shell.dart`.** | The shell is the real authenticated home; no duplicate placeholder. |
| 2026-06-02 | **No codegen (`freezed`/`json_serializable`) in this spec.** | Few small DTOs; keep the first mobile build dependency-light. Revisit if DTOs multiply in R-0008+. |
| 2026-06-02 | **(architect) Router built once; a `ValueNotifier` driven by `ref.listen` is the sole `refreshListenable` — the provider body does not `watch` auth.** | Avoids rebuilding the whole `GoRouter` (and discarding nav state) on every auth change; removes the undefined `authChangeNotifierProvider` ambiguity (Finding 1). |
| 2026-06-02 | **(architect) `_restore` wraps storage reads; a throw clears the entry and falls back to `AuthUnauthenticated`.** | A corrupt/failed secure-storage read must not strand the app in the splash state (Finding 2; SAC3). |
| 2026-06-02 | **(architect) Restored token is optimistic; `GET /auth/me` is the liveness check that 401-sinks an expired one.** | Documents the cold-start-with-stale-token path (Finding 3; SAC5). |
| 2026-06-03 | **(qa step-7) The `integration_test` is authored + compile-checked but run by no gate; execution deferred to R-0025.** | `flutter test` and CI run `test/` only; `flutter test integration_test/` needs the platform folders R-0007 omits (corrects OQ-D4). The `login → home` capability is redundantly covered by the widget suites, so SAC10/AC10 were amended to match reality rather than gate-running the e2e now. |

## Changelog

- _2026-06-02 — created (Draft). Realizes the accepted R-0007 (Flutter app architecture + auth shell). Five HOW-level design questions (OQ-D1..D5: dio, go_router, in-memory token, headless integration_test, Notifier) raised for the architect review._
- _2026-06-02 — **Accepted.** Architect review returned APPROVE WITH NITS; all five OQ-D resolved as proposed (dio, go_router, in-memory token, headless flutter-tester e2e with no `flutter create`, Notifier). Applied the four findings in lockstep: router built once + `ValueNotifier`/`ref.listen` refresh (§2.7), `_restore` error handling (§2.5), optimistic-restore liveness note (§2.5/SAC5), `AuthToken.userId` "no JWT decode" wording (§3.1); SAC3/SAC5 sub-cases added._
- _2026-06-03 — **AC10 e2e clause amended (qa step-7 sign-off, owner-approved).** The full `test/` suite is green (49/49) and qa signed off, but the `integration_test` is run by no gate: it needs platform folders R-0007 deliberately omits, so the OQ-D4 "headless flutter-tester" resolution did not actually execute. Scoped SAC10 / OQ-D4 / §2.2 gate-table to "authored + compile-checked now; gate execution deferred to R-0025"; the `login → home` capability stays covered by the `test/` widget suites. Separately, two qa-authored test-harness bugs were fixed during implementation with no production change: the `pumpShell` `Future.delayed` fake-async hang (all three shell tests), and the bare-`ErrorInterceptorHandler` interceptor tests (now drive a real `Dio` request through `onError`)._
