import 'dart:ui';

/// The locales the app has a complete catalogue for (R-0061 AC11, OQ-1).
/// Spanish first: it is the template and the fallback.
const supportedLocales = [Locale('es'), Locale('en')];

/// Picks the app locale from the device's preferences: the first device
/// language that has a catalogue wins, and a device with none falls back to
/// Spanish (owner, 2026-10-22). Only the language code is matched, so
/// `es_MX` and `en_GB` resolve like `es` and `en`.
///
/// Pure, so it is unit-tested without a widget tree; `app.dart` hands it to
/// `MaterialApp.localeListResolutionCallback`.
Locale resolveLocale(List<Locale>? device, Iterable<Locale> supported) {
  for (final wanted in device ?? const <Locale>[]) {
    for (final have in supported) {
      if (have.languageCode == wanted.languageCode) return have;
    }
  }
  return supported.first;
}
