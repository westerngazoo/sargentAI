import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// Registers the bundled faces' licences so they appear on the app's licence
/// page (SPEC-0061 §2.10). Called once from `main`. Kept apart from the theme
/// because it reads the asset bundle, and the theme is pure.
void registerFontLicences() {
  LicenseRegistry.addLicense(() async* {
    for (final (package, file) in const [
      ('Bangers', 'assets/fonts/OFL-Bangers.txt'),
      ('Luckiest Guy', 'assets/fonts/LICENSE-LuckiestGuy.txt'),
      ('IBM Plex Mono', 'assets/fonts/OFL-IBMPlexMono.txt'),
      ('Archivo', 'assets/fonts/OFL-Archivo.txt'),
    ]) {
      yield LicenseEntryWithLineBreaks(
        [package],
        await rootBundle.loadString(file),
      );
    }
  });
}
