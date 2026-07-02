// Sargent AI design system v2 — Apple restraint, SpaceX precision, Venice
// Beach warmth. One place owns color, type, and shape.
//
// Palette: deep launch-pad slate surfaces (dark) / warm sand (light) with a
// Venice-sunset accent (coral → dusk magenta) reserved for the few things
// that matter: the hero card, the mic, and primary actions. Everything else
// stays quiet — hairline-free surfaces, generous whitespace, tight display
// type.

import 'package:flutter/material.dart';

abstract final class AppTheme {
  /// Venice sunset coral — the single accent everything keys off.
  static const seed = Color(0xFFFF6B45);

  /// Sunset gradient endpoints (hero card, brand roundel, speak button).
  static const sunsetStart = Color(0xFFFF7A59);
  static const sunsetEnd = Color(0xFFE0447C);

  static ThemeData light() => _build(Brightness.light);
  static ThemeData dark() => _build(Brightness.dark);

  static ThemeData _build(Brightness brightness) {
    final isDark = brightness == Brightness.dark;
    final base = ColorScheme.fromSeed(
      seedColor: seed,
      brightness: brightness,
      dynamicSchemeVariant: DynamicSchemeVariant.vibrant,
    );
    // SpaceX slate (dark) / Venice sand (light) neutrals; sunset accents.
    final scheme = isDark
        ? base.copyWith(
            primary: sunsetStart,
            onPrimary: const Color(0xFF1C0D08),
            primaryContainer: const Color(0xFF3B1D12),
            onPrimaryContainer: const Color(0xFFFFD9CC),
            tertiary: sunsetEnd,
            surface: const Color(0xFF0D1117),
            onSurface: const Color(0xFFF0EEEB),
            onSurfaceVariant: const Color(0xFF9AA3AF),
            surfaceContainerLowest: const Color(0xFF0A0D12),
            surfaceContainerLow: const Color(0xFF141A22),
            surfaceContainer: const Color(0xFF171E27),
            surfaceContainerHigh: const Color(0xFF1D2530),
            surfaceContainerHighest: const Color(0xFF242E3B),
            outlineVariant: const Color(0xFF2A333F),
          )
        : base.copyWith(
            primary: const Color(0xFFE8502B),
            onPrimary: Colors.white,
            primaryContainer: const Color(0xFFFFE0D5),
            onPrimaryContainer: const Color(0xFF551D0C),
            tertiary: const Color(0xFFC2417F),
            surface: const Color(0xFFFAF6F0),
            onSurface: const Color(0xFF1C1B1A),
            onSurfaceVariant: const Color(0xFF6E675F),
            surfaceContainerLowest: Colors.white,
            surfaceContainerLow: Colors.white,
            surfaceContainer: const Color(0xFFF3EDE5),
            surfaceContainerHigh: const Color(0xFFEDE5DA),
            surfaceContainerHighest: const Color(0xFFE6DCCF),
            outlineVariant: const Color(0xFFE0D6C8),
          );

    final baseTheme = ThemeData(colorScheme: scheme, useMaterial3: true);
    // Apple-like type: big, tight, confident display; relaxed readable body.
    final textTheme = baseTheme.textTheme.copyWith(
      headlineMedium: baseTheme.textTheme.headlineMedium?.copyWith(
          fontWeight: FontWeight.w800, letterSpacing: -0.8, height: 1.1),
      headlineSmall: baseTheme.textTheme.headlineSmall
          ?.copyWith(fontWeight: FontWeight.w800, letterSpacing: -0.5),
      titleLarge: baseTheme.textTheme.titleLarge
          ?.copyWith(fontWeight: FontWeight.w700, letterSpacing: -0.4),
      titleMedium: baseTheme.textTheme.titleMedium
          ?.copyWith(fontWeight: FontWeight.w700, letterSpacing: -0.2),
      bodyMedium: baseTheme.textTheme.bodyMedium?.copyWith(height: 1.45),
      bodySmall: baseTheme.textTheme.bodySmall?.copyWith(height: 1.4),
      labelLarge:
          baseTheme.textTheme.labelLarge?.copyWith(fontWeight: FontWeight.w600),
    );

    return baseTheme.copyWith(
      textTheme: textTheme,
      scaffoldBackgroundColor: scheme.surface,
      splashFactory: InkSparkle.splashFactory,
      appBarTheme: AppBarTheme(
        centerTitle: false,
        elevation: 0,
        scrolledUnderElevation: 0,
        backgroundColor: scheme.surface,
        titleTextStyle: textTheme.titleLarge?.copyWith(
          color: scheme.onSurface,
          fontWeight: FontWeight.w800,
          letterSpacing: -0.5,
        ),
      ),
      cardTheme: baseTheme.cardTheme.copyWith(
        elevation: 0,
        color: scheme.surfaceContainerLow,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(24)),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          minimumSize: const Size.fromHeight(54),
          shape: const StadiumBorder(),
          textStyle: textTheme.labelLarge?.copyWith(fontSize: 16),
        ),
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          minimumSize: const Size.fromHeight(54),
          elevation: 0,
          backgroundColor: scheme.primary,
          foregroundColor: scheme.onPrimary,
          shape: const StadiumBorder(),
          textStyle: textTheme.labelLarge?.copyWith(fontSize: 16),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          minimumSize: const Size.fromHeight(48),
          shape: const StadiumBorder(),
          side: BorderSide(color: scheme.outlineVariant),
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(textStyle: textTheme.labelLarge),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: isDark
            ? scheme.surfaceContainerHigh.withValues(alpha: 0.6)
            : Colors.white,
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(16),
          borderSide: BorderSide.none,
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(16),
          borderSide: isDark
              ? BorderSide.none
              : BorderSide(color: scheme.outlineVariant),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(16),
          borderSide: BorderSide(color: scheme.primary, width: 1.5),
        ),
        contentPadding:
            const EdgeInsets.symmetric(horizontal: 16, vertical: 15),
      ),
      chipTheme: baseTheme.chipTheme.copyWith(
        shape: const StadiumBorder(),
        side: BorderSide.none,
        backgroundColor: scheme.surfaceContainerHigh,
        labelStyle: textTheme.labelMedium,
      ),
      floatingActionButtonTheme: FloatingActionButtonThemeData(
        elevation: 3,
        backgroundColor: scheme.primary,
        foregroundColor: scheme.onPrimary,
        extendedTextStyle: textTheme.labelLarge?.copyWith(fontSize: 15),
        shape: const StadiumBorder(),
      ),
      snackBarTheme: SnackBarThemeData(
        behavior: SnackBarBehavior.floating,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
      ),
      bottomSheetTheme: BottomSheetThemeData(
        backgroundColor: scheme.surface,
        shape: const RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(top: Radius.circular(32)),
        ),
      ),
      dialogTheme: baseTheme.dialogTheme.copyWith(
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(28)),
      ),
      dividerTheme: DividerThemeData(
        color: scheme.outlineVariant.withValues(alpha: 0.5),
        thickness: 0.7,
      ),
    );
  }
}

/// The signature sunset gradient — hero card, brand roundel, speak button.
LinearGradient sunsetGradient() => const LinearGradient(
      begin: Alignment.topLeft,
      end: Alignment.bottomRight,
      colors: [AppTheme.sunsetStart, AppTheme.sunsetEnd],
    );
