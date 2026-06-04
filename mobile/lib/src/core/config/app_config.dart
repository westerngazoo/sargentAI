/// Compile-time application configuration.
///
/// The API base URL is injected at build time via
/// `--dart-define=API_BASE_URL=...`; the default targets a local backend so
/// `flutter run` and `flutter test` work without extra flags (SPEC-0007 §2.3,
/// AC7). No production URL is compiled in.
class AppConfig {
  const AppConfig._();

  static const String apiBaseUrl = String.fromEnvironment(
    'API_BASE_URL',
    defaultValue: 'http://localhost:8080',
  );
}
