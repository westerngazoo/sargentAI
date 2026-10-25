import 'dart:ui';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import 'locale_rule.dart';

/// The catalogue for code that runs outside the widget tree: the voice loops
/// speak lines from notifiers, which hold no `BuildContext` (R-0061 AC11).
///
/// Resolved with the same rule `MaterialApp` applies to the platform's locale
/// list (`localeListResolutionCallback: resolveLocale`), so a spoken line and
/// a drawn one agree on the language. Tests override it with the Spanish
/// catalogue (`test/support/l10n.dart`).
///
/// Resolved once per app run: a system-language change while the app is
/// open reaches `MaterialApp`, but the voice loops keep this catalogue until
/// the next start. Accepted for the beta.
final appLocalizationsProvider = Provider<AppLocalizations>(
  (ref) => lookupAppLocalizations(
    resolveLocale(PlatformDispatcher.instance.locales, supportedLocales),
  ),
);
