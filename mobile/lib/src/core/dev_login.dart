import 'package:flutter/foundation.dart';

/// Debug-only quick login — compiled out of release builds via [kDebugMode].
///
/// Fills the real login flow with a seeded test account so nobody types
/// credentials during development. Override the account per run with
/// `--dart-define=TEST_LOGIN_EMAIL=… --dart-define=TEST_LOGIN_PASSWORD=…`.
abstract final class DevLogin {
  static const enabled = kDebugMode;

  static const email = String.fromEnvironment(
    'TEST_LOGIN_EMAIL',
    defaultValue: 'demo@fitai.app',
  );

  static const password = String.fromEnvironment(
    'TEST_LOGIN_PASSWORD',
    defaultValue: 'demo1234',
  );
}
