// Sargent AI design system v3 — military issue. Apple restraint, SpaceX
// precision, drill-sergeant green. One place owns color, type, and shape.
//
// Palette: night-ops green-black surfaces (dark) / field khaki (light) with
// an olive-drab accent gradient reserved for the few things that matter:
// the hero card, the mic, and primary actions. Everything else stays quiet —
// hairline-free surfaces, generous whitespace, tight display type.

import 'package:flutter/material.dart';

abstract final class AppTheme {
  /// Olive drab — the single accent everything keys off.
  static const seed = Color(0xFF6B8E23);

  /// Brand gradient endpoints (hero card, brand roundel, speak button):
  /// bright olive into deep forest.
  static const gradStart = Color(0xFF87A144);
  static const gradEnd = Color(0xFF39511F);

  static ThemeData light() => _build(Brightness.light);
  static ThemeData dark() => _build(Brightness.dark);

  static ThemeData _build(Brightness brightness) {
    final isDark = brightness == Brightness.dark;
    final base = ColorScheme.fromSeed(
      seedColor: seed,
      brightness: brightness,
      dynamicSchemeVariant: DynamicSchemeVariant.vibrant,
    );
    // Night-ops green-black (dark) / field khaki (light); olive accents,
    // brass for highlights.
    final scheme = isDark
        ? base.copyWith(
            primary: const Color(0xFF9FB65A),
            onPrimary: const Color(0xFF161C06),
            primaryContainer: const Color(0xFF2E3A14),
            onPrimaryContainer: const Color(0xFFDCE8AE),
            secondaryContainer: const Color(0xFF232D15),
            onSecondaryContainer: const Color(0xFFD4DDB6),
            tertiary: const Color(0xFFC9B458),
            surface: const Color(0xFF0C0F0A),
            onSurface: const Color(0xFFEFF0E9),
            onSurfaceVariant: const Color(0xFF99A18A),
            surfaceContainerLowest: const Color(0xFF090C07),
            surfaceContainerLow: const Color(0xFF141A10),
            surfaceContainer: const Color(0xFF171E13),
            surfaceContainerHigh: const Color(0xFF1D2617),
            surfaceContainerHighest: const Color(0xFF242F1D),
            outlineVariant: const Color(0xFF2C3722),
          )
        : base.copyWith(
            primary: const Color(0xFF4F6420),
            onPrimary: Colors.white,
            primaryContainer: const Color(0xFFDCE5B2),
            onPrimaryContainer: const Color(0xFF1B2405),
            secondaryContainer: const Color(0xFFE4E6CC),
            onSecondaryContainer: const Color(0xFF3A4028),
            tertiary: const Color(0xFF7C6A32),
            surface: const Color(0xFFF2F1E6),
            onSurface: const Color(0xFF1A1C14),
            onSurfaceVariant: const Color(0xFF696E58),
            surfaceContainerLowest: Colors.white,
            surfaceContainerLow: Colors.white,
            surfaceContainer: const Color(0xFFEAE9D9),
            surfaceContainerHigh: const Color(0xFFE2E1CC),
            surfaceContainerHighest: const Color(0xFFD9D8BF),
            outlineVariant: const Color(0xFFD3D2B8),
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

/// The signature brand gradient — hero card, brand roundel, speak button.
LinearGradient brandGradient() => const LinearGradient(
      begin: Alignment.topLeft,
      end: Alignment.bottomRight,
      colors: [AppTheme.gradStart, AppTheme.gradEnd],
    );
