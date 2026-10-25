// The @fisicobuenfisico look (R-0061 AC12, SPEC-0061 §2.10): black ink on
// white paper, or white on near-black, and red as the only colour. Hard
// borders, hard shadows, comic headlines, mono data. One place owns colour,
// type and shape; the values live in `tokens.dart`.

import 'package:flutter/material.dart';

import 'tokens.dart';

abstract final class AppTheme {
  static ThemeData light() => _build(FbfTokens.light, Brightness.light);
  static ThemeData dark() => _build(FbfTokens.dark, Brightness.dark);

  /// Focus mode (a session in progress) is dark whatever the device theme,
  /// for legibility in the gym. Today that is the dark theme itself; it gets
  /// its own tokens only if the two ever diverge.
  static ThemeData focus() => dark();

  static ThemeData _build(FbfTokens t, Brightness brightness) {
    final scheme = ColorScheme(
      brightness: brightness,
      primary: t.red,
      onPrimary: t.onRed,
      primaryContainer: t.panelAlt,
      onPrimaryContainer: t.text,
      secondary: t.text,
      onSecondary: t.ground,
      secondaryContainer: t.panelAlt,
      onSecondaryContainer: t.text,
      tertiary: t.muted,
      onTertiary: t.ground,
      error: t.red,
      onError: t.onRed,
      surface: t.ground,
      onSurface: t.text,
      onSurfaceVariant: t.muted,
      surfaceContainerLowest: t.panel,
      surfaceContainerLow: t.panel,
      surfaceContainer: t.panel,
      surfaceContainerHigh: t.panelAlt,
      surfaceContainerHighest: t.panelAlt,
      outline: t.line,
      outlineVariant: t.line,
    );

    final base = ThemeData(
      colorScheme: scheme,
      useMaterial3: true,
      fontFamily: AppFonts.ui,
    );
    final text = base.textTheme.copyWith(
      // Headlines are the comic face, short and sparing (§2.10).
      headlineLarge: base.textTheme.headlineLarge?.copyWith(
          fontFamily: AppFonts.headline, letterSpacing: 0.5, height: 1.0),
      headlineMedium: base.textTheme.headlineMedium?.copyWith(
          fontFamily: AppFonts.headline, letterSpacing: 0.5, height: 1.0),
      headlineSmall: base.textTheme.headlineSmall
          ?.copyWith(fontFamily: AppFonts.headline, letterSpacing: 0.4),
      titleLarge:
          base.textTheme.titleLarge?.copyWith(fontWeight: FontWeight.w800),
      titleMedium:
          base.textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
      bodyMedium: base.textTheme.bodyMedium?.copyWith(height: 1.45),
      bodySmall: base.textTheme.bodySmall?.copyWith(height: 1.4),
      labelLarge:
          base.textTheme.labelLarge?.copyWith(fontWeight: FontWeight.w700),
      // Data lines are mono.
      labelMedium: base.textTheme.labelMedium
          ?.copyWith(fontFamily: AppFonts.mono, letterSpacing: 0.2),
      labelSmall: base.textTheme.labelSmall
          ?.copyWith(fontFamily: AppFonts.mono, letterSpacing: 0.6),
    );

    final hardBorder = BorderSide(color: t.line, width: 2);
    final square = RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(6),
      side: hardBorder,
    );

    return base.copyWith(
      extensions: [t],
      textTheme: text,
      scaffoldBackgroundColor: t.ground,
      splashFactory: InkSparkle.splashFactory,
      appBarTheme: AppBarTheme(
        centerTitle: false,
        elevation: 0,
        scrolledUnderElevation: 0,
        backgroundColor: t.ground,
        foregroundColor: t.text,
        titleTextStyle: text.titleLarge?.copyWith(color: t.text),
      ),
      cardTheme: base.cardTheme.copyWith(
        elevation: 0,
        color: t.panel,
        shape: square,
        margin: EdgeInsets.zero,
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          minimumSize: const Size.fromHeight(54),
          backgroundColor: t.red,
          foregroundColor: t.onRed,
          shape: square.copyWith(side: BorderSide.none),
          textStyle: text.labelLarge?.copyWith(fontSize: 16),
        ),
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          minimumSize: const Size.fromHeight(54),
          elevation: 0,
          backgroundColor: t.red,
          foregroundColor: t.onRed,
          shape: square.copyWith(side: BorderSide.none),
          textStyle: text.labelLarge?.copyWith(fontSize: 16),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          minimumSize: const Size.fromHeight(48),
          foregroundColor: t.text,
          shape: square.copyWith(side: BorderSide.none),
          side: hardBorder,
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(
          foregroundColor: t.red,
          textStyle: text.labelLarge,
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: t.panel,
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: hardBorder,
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: hardBorder,
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: BorderSide(color: t.red, width: 2),
        ),
        contentPadding:
            const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
      ),
      chipTheme: base.chipTheme.copyWith(
        shape: square,
        side: hardBorder,
        backgroundColor: t.panel,
        labelStyle: text.labelMedium?.copyWith(color: t.text),
      ),
      floatingActionButtonTheme: FloatingActionButtonThemeData(
        elevation: 0,
        backgroundColor: t.red,
        foregroundColor: t.onRed,
        shape: const CircleBorder(),
      ),
      snackBarTheme: SnackBarThemeData(
        behavior: SnackBarBehavior.floating,
        backgroundColor: t.bar,
        contentTextStyle: text.bodyMedium?.copyWith(color: t.barText),
        // SnackBarAction paints with inversePrimary, whose fallback is
        // onPrimary: near-black on the black bar in dark mode. Say red.
        actionTextColor: t.barRed,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      ),
      bottomSheetTheme: BottomSheetThemeData(
        backgroundColor: t.panel,
        shape: RoundedRectangleBorder(
          borderRadius: const BorderRadius.vertical(top: Radius.circular(12)),
          side: hardBorder,
        ),
      ),
      dialogTheme: base.dialogTheme.copyWith(
        backgroundColor: t.panel,
        shape: square,
      ),
      dividerTheme: DividerThemeData(color: t.line, thickness: 1),
    );
  }
}
