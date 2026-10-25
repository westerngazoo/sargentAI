import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'l10n/app_localizations.dart';
import 'src/core/brand.dart';
import 'src/core/theme/app_theme.dart';
import 'src/l10n/locale_rule.dart';
import 'src/router/app_router.dart';

/// The application root: a `MaterialApp.router` wired to the auth-gated
/// [routerProvider] (SPEC-0007 §3.3) and the fitAI design system.
class FitAiApp extends ConsumerWidget {
  const FitAiApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return MaterialApp.router(
      title: Brand.appName,
      debugShowCheckedModeBanner: false,
      theme: AppTheme.light(),
      darkTheme: AppTheme.dark(),
      // R-0061 AC11: every UI string comes from the catalogues; the device
      // language picks one and Spanish is the fallback (SPEC-0061 §2.9).
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: supportedLocales,
      localeListResolutionCallback: resolveLocale,
      routerConfig: ref.watch(routerProvider),
    );
  }
}
