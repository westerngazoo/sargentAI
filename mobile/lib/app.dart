import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/core/theme/app_theme.dart';
import 'src/router/app_router.dart';

/// The application root: a `MaterialApp.router` wired to the auth-gated
/// [routerProvider] (SPEC-0007 §3.3) and the fitAI design system.
class FitAiApp extends ConsumerWidget {
  const FitAiApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return MaterialApp.router(
      title: 'fitAI',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.light(),
      darkTheme: AppTheme.dark(),
      routerConfig: ref.watch(routerProvider),
    );
  }
}
