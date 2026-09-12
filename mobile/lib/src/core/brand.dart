/// Product branding — the single place the *Dart* user-facing name lives.
///
/// The repo, the Dart package and the crates stay `fitai`: those are
/// identifiers no user sees, and renaming them is compiler-checked churn.
///
/// This seam stops at the platform boundary. The name also appears, un-seamed,
/// in `android/app/src/main/AndroidManifest.xml` (`android:label`),
/// `ios/Runner/Info.plist` (display name, bundle name, two usage strings),
/// `web/index.html`, `web/manifest.json`, and the notification channel name in
/// `earbud_coach.dart`. A rename must touch those too — this class is not
/// "a one-line change", and describing it as one is how they got missed.
abstract final class Brand {
  static const appName = 'Goose Physics';
  static const tagline = 'The physics of your lift — measured, not guessed.';
}
