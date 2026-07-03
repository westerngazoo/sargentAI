/// Google OAuth client id from `--dart-define=GOOGLE_CLIENT_ID=...`.
class GoogleSignInConfig {
  const GoogleSignInConfig._();

  static const String clientId = String.fromEnvironment('GOOGLE_CLIENT_ID');

  static bool get enabled => clientId.isNotEmpty;
}
