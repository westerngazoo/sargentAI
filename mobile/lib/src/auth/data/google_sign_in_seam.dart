// R-0033 — dependency-inverted Google Sign-In seam.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:google_sign_in/google_sign_in.dart';

import '../../core/google_sign_in_config.dart';

/// What the auth layer needs from Google Sign-In — one ID token.
abstract class GoogleSignInSeam {
  Future<String?> signInForIdToken();
}

/// Production implementation using `google_sign_in`.
class PluginGoogleSignInSeam implements GoogleSignInSeam {
  PluginGoogleSignInSeam(this._client);

  final GoogleSignIn _client;

  @override
  Future<String?> signInForIdToken() async {
    final account = await _client.signIn();
    if (account == null) return null;
    final auth = await account.authentication;
    return auth.idToken;
  }
}

final googleSignInSeamProvider = Provider<GoogleSignInSeam>((ref) {
  if (!GoogleSignInConfig.enabled) {
    return _DisabledGoogleSignInSeam();
  }
  return PluginGoogleSignInSeam(
    GoogleSignIn(
      clientId: GoogleSignInConfig.clientId,
      scopes: const ['email', 'profile', 'openid'],
    ),
  );
});

class _DisabledGoogleSignInSeam implements GoogleSignInSeam {
  @override
  Future<String?> signInForIdToken() async => null;
}
