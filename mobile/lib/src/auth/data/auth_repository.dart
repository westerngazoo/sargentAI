import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/network/dio_provider.dart';
import '../../core/storage/token_store.dart';
import 'auth_api.dart';

/// Orchestrates the [AuthApi] (transport) and [TokenStore] (persistence) so the
/// `AuthController` depends on one seam, not two (SPEC-0007 §2.5). `login`
/// persists the returned token; `clear` wipes it on logout / 401.
class AuthRepository {
  const AuthRepository(this._api, this._store);

  final AuthApi _api;
  final TokenStore _store;

  Future<void> register(String email, String password) =>
      _api.register(email, password);

  Future<AuthToken> login(String email, String password) async {
    final token = await _api.login(email, password);
    await _store.write(token);
    return token;
  }

  Future<AuthToken> loginWithGoogle(String idToken) async {
    final token = await _api.loginWithGoogle(idToken);
    await _store.write(token);
    return token;
  }

  Future<String> me() => _api.me();

  Future<void> clear() => _store.clear();
}

final authRepositoryProvider = Provider<AuthRepository>(
  (ref) => AuthRepository(
    AuthApi(ref.watch(dioProvider)),
    ref.watch(tokenStoreProvider),
  ),
);
