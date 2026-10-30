// R-0061 AC11 (OQ-1) -> SPEC-0061 §2.9: the app follows the device locale
// and falls back to Spanish when no catalogue matches.

import 'dart:ui';

import 'package:fitai/l10n/app_localizations.dart';
import 'package:fitai/src/l10n/locale_rule.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('a Spanish device gets es', () {
    expect(resolveLocale([const Locale('es', 'MX')], supportedLocales),
        const Locale('es'));
  });

  test('Spanish numbers follow es_MX: a decimal point (owner, 2026-10-30)', () {
    expect(numberLocale('es'), 'es_MX');
    expect(numberLocale('en'), 'en');
  });

  test('an English device gets en', () {
    expect(resolveLocale([const Locale('en', 'GB')], supportedLocales),
        const Locale('en'));
  });

  test('a device with no catalogue falls back to es', () {
    expect(
        resolveLocale(
            [const Locale('fr'), const Locale('de')], supportedLocales),
        const Locale('es'));
  });

  test('the first matching device language wins', () {
    expect(
        resolveLocale(
            [const Locale('fr'), const Locale('en')], supportedLocales),
        const Locale('en'));
  });

  test('no device preference at all falls back to es', () {
    expect(resolveLocale(null, supportedLocales), const Locale('es'));
    expect(resolveLocale(const [], supportedLocales), const Locale('es'));
  });

  test('the ordered list names exactly the generated catalogues', () {
    // Order is ours (first = fallback); the set must match gen-l10n's.
    expect(supportedLocales.toSet(), AppLocalizations.supportedLocales.toSet());
  });
}
