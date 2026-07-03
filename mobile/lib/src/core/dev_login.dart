import 'package:flutter/foundation.dart';

/// Quick login — fills the real login flow with a seeded test account so
/// nobody types credentials. Shown in debug builds automatically, and in any
/// build compiled with `--dart-define=ENABLE_TEST_LOGIN=true` (used for the
/// hosted preview). Override the account with `--dart-define=TEST_LOGIN_EMAIL=…
/// --dart-define=TEST_LOGIN_PASSWORD=…`.
abstract final class DevLogin {
  static const enabled =
      kDebugMode || bool.fromEnvironment('ENABLE_TEST_LOGIN');

  static const email = String.fromEnvironment(
    'TEST_LOGIN_EMAIL',
    defaultValue: 'demo@fitai.app',
  );

  static const password = String.fromEnvironment(
    'TEST_LOGIN_PASSWORD',
    defaultValue: 'demo1234',
  );
}
