// R-0061 AC12 -> SPEC-0061 §2.10: the @fisicobuenfisico look. Black, white
// and red; red is the only colour; focus mode is dark in both themes; the
// four bundled faces and their licences.

import 'package:fitai/src/core/theme/app_theme.dart';
import 'package:fitai/src/core/theme/tokens.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  // The licence loader reads the licence texts from the asset bundle.
  TestWidgetsFlutterBinding.ensureInitialized();

  group('tokens', () {
    test('light is black ink on white with red', () {
      final t = FbfTokens.light;
      expect(t.ground, const Color(0xFFF4F3F1));
      expect(t.panel, const Color(0xFFFFFFFF));
      expect(t.text, const Color(0xFF111111));
      expect(t.red, const Color(0xFFC8102E));
      expect(t.heat2, t.red, reason: 'the hot end of the ramp is red');
    });

    test('dark is white ink on near-black with the same red family', () {
      final t = FbfTokens.dark;
      expect(t.ground, const Color(0xFF0B0B0B));
      expect(t.text, const Color(0xFFF2F2F2));
      expect(t.red, const Color(0xFFFF4D4D));
      expect(t.heat2, t.red);
    });

    test('the footer bar is the same in both themes', () {
      expect(FbfTokens.light.bar, FbfTokens.dark.bar);
      expect(FbfTokens.light.barRed, FbfTokens.dark.barRed);
    });
  });

  group('themes', () {
    test('primary is red, surface is ground, in both modes', () {
      for (final (theme, t) in [
        (AppTheme.light(), FbfTokens.light),
        (AppTheme.dark(), FbfTokens.dark),
      ]) {
        expect(theme.colorScheme.primary, t.red);
        expect(theme.colorScheme.onPrimary, t.onRed);
        expect(theme.colorScheme.surface, t.ground);
        expect(theme.scaffoldBackgroundColor, t.ground);
        expect(theme.extension<FbfTokens>(), t);
      }
    });

    test('focus mode is dark whatever the device theme', () {
      final focus = AppTheme.focus();
      expect(focus.brightness, Brightness.dark);
      expect(focus.colorScheme.surface, FbfTokens.dark.bar);
    });

    test('UI text is Archivo; headlines are Bangers; numbers are mono', () {
      final theme = AppTheme.light();
      expect(theme.textTheme.bodyMedium?.fontFamily, AppFonts.ui);
      expect(theme.textTheme.headlineMedium?.fontFamily, AppFonts.headline);
      expect(AppFonts.big, 'LuckiestGuy');
      expect(AppFonts.mono, 'IBMPlexMono');
    });

    test('no olive left: the old seed and gradient are gone', () {
      // The old theme keyed everything off olive drab 0xFF6B8E23.
      for (final theme in [AppTheme.light(), AppTheme.dark()]) {
        final cs = theme.colorScheme;
        for (final c in [cs.primary, cs.secondary, cs.tertiary, cs.surface]) {
          expect(c, isNot(const Color(0xFF6B8E23)));
          expect(c, isNot(const Color(0xFF87A144)));
          expect(c, isNot(const Color(0xFF39511F)));
        }
      }
    });
  });

  group('font licences', () {
    test('the four faces register their licences', () async {
      registerFontLicences();
      final packages = <String>{};
      await for (final entry in LicenseRegistry.licenses) {
        packages.addAll(entry.packages);
      }
      expect(
        packages,
        containsAll(<String>[
          'Bangers',
          'Luckiest Guy',
          'IBM Plex Mono',
          'Archivo',
        ]),
      );
    });
  });
}
