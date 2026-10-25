import 'package:fitai/l10n/app_localizations.dart';
import 'package:fitai/src/l10n/locale_rule.dart';
import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

/// The Spanish catalogue, for assertions that name a string by key instead of
/// repeating its text (R-0061 AC11). Tests run the app in `es`.
final l10n = lookupAppLocalizations(const Locale('es'));

const _delegates = AppLocalizations.localizationsDelegates;

/// A `MaterialApp` with the app's catalogues, pinned to Spanish. Takes a
/// [home], or named [routes] with an [initialRoute], like `MaterialApp`.
Widget localizedApp({
  Widget? home,
  Map<String, WidgetBuilder> routes = const {},
  String? initialRoute,
}) =>
    MaterialApp(
      home: home,
      routes: routes,
      initialRoute: initialRoute,
      locale: const Locale('es'),
      supportedLocales: supportedLocales,
      localizationsDelegates: _delegates,
    );

/// A `MaterialApp.router` with the app's catalogues, pinned to Spanish.
Widget localizedRouterApp(GoRouter router) => MaterialApp.router(
      routerConfig: router,
      locale: const Locale('es'),
      supportedLocales: supportedLocales,
      localizationsDelegates: _delegates,
    );
