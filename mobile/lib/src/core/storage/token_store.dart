import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// The persisted session credential (SPEC-0007 §3.1). `userId` comes from the
/// login response `user_id` (no JWT decode); `expiresAt` is advisory — expiry
/// is enforced server-side and surfaced as a 401 (AC5).
class AuthToken {
  const AuthToken({
    required this.jwt,
    required this.userId,
    required this.expiresAt,
  });

  final String jwt;
  final String userId;
  final DateTime expiresAt;
}

final tokenStoreProvider = Provider<TokenStore>(
  (_) => const TokenStore(FlutterSecureStorage()),
);

/// Wraps platform secure storage (iOS Keychain / Android Keystore) with a
/// JSON round-trip of the [AuthToken]. `read` deliberately does NOT swallow a
/// storage failure — the caller (`AuthController._restore`) decides how to
/// recover (architect Finding 2).
class TokenStore {
  const TokenStore(this._storage);

  static const String _key = 'fitai.jwt';

  final FlutterSecureStorage _storage;

  Future<AuthToken?> read() async {
    final raw = await _storage.read(key: _key);
    if (raw == null) return null;
    final map = jsonDecode(raw) as Map<String, dynamic>;
    return AuthToken(
      jwt: map['jwt'] as String,
      userId: map['userId'] as String,
      expiresAt: DateTime.parse(map['expiresAt'] as String),
    );
  }

  Future<void> write(AuthToken token) {
    return _storage.write(
      key: _key,
      value: jsonEncode(<String, String>{
        'jwt': token.jwt,
        'userId': token.userId,
        'expiresAt': token.expiresAt.toIso8601String(),
      }),
    );
  }

  Future<void> clear() => _storage.delete(key: _key);
}
