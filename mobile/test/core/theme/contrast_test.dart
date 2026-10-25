// R-0061 AC12 -> SPEC-0061 §2.10: every colour pair the theme defines meets
// WCAG 2.1 AA in both modes. The pairs come from the built ColorScheme (each
// `on*` against its base), not from a hand-written list, plus the custom
// tokens classified by their allowed use: body text needs 4.5:1, large text
// and marks 3:1. The heat ramp's low end is marks-only on purpose.

import 'dart:math' as math;

import 'package:fitai/src/core/theme/app_theme.dart';
import 'package:fitai/src/core/theme/tokens.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

double _luminance(Color c) {
  double channel(double v) =>
      v <= 0.03928 ? v / 12.92 : math.pow((v + 0.055) / 1.055, 2.4).toDouble();
  return 0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b);
}

double contrast(Color a, Color b) {
  final la = _luminance(a), lb = _luminance(b);
  return (math.max(la, lb) + 0.05) / (math.min(la, lb) + 0.05);
}

const bodyText = 4.5;
const largeOrMark = 3.0;

void main() {
  for (final (name, theme) in [
    ('light', AppTheme.light()),
    ('dark', AppTheme.dark()),
    ('focus', AppTheme.focus()),
  ]) {
    group('$name theme', () {
      final cs = theme.colorScheme;
      final t = theme.extension<FbfTokens>()!;

      final schemePairs = <String, (Color, Color)>{
        'onPrimary/primary': (cs.onPrimary, cs.primary),
        'onSecondary/secondary': (cs.onSecondary, cs.secondary),
        'onTertiary/tertiary': (cs.onTertiary, cs.tertiary),
        'onError/error': (cs.onError, cs.error),
        'onSurface/surface': (cs.onSurface, cs.surface),
        'onSurfaceVariant/surface': (cs.onSurfaceVariant, cs.surface),
        'onPrimaryContainer/primaryContainer':
            (cs.onPrimaryContainer, cs.primaryContainer),
        'onSecondaryContainer/secondaryContainer':
            (cs.onSecondaryContainer, cs.secondaryContainer),
        'onSurface/surfaceContainer': (cs.onSurface, cs.surfaceContainer),
        'onSurface/surfaceContainerHigh':
            (cs.onSurface, cs.surfaceContainerHigh),
      };
      for (final e in schemePairs.entries) {
        test('${e.key} is body-text legible', () {
          expect(contrast(e.value.$1, e.value.$2),
              greaterThanOrEqualTo(bodyText),
              reason: e.key);
        });
      }

      test('text and muted text on panel are body-text legible', () {
        expect(contrast(t.text, t.panel), greaterThanOrEqualTo(bodyText));
        expect(contrast(t.muted, t.panel), greaterThanOrEqualTo(bodyText));
        expect(contrast(t.text, t.ground), greaterThanOrEqualTo(bodyText));
      });

      test('red passes as text on panel and ground', () {
        expect(contrast(t.red, t.panel), greaterThanOrEqualTo(bodyText));
        expect(contrast(t.red, t.ground), greaterThanOrEqualTo(bodyText));
        expect(contrast(t.onRed, t.red), greaterThanOrEqualTo(bodyText));
      });

      test('the footer bar pairs are legible', () {
        expect(contrast(t.barText, t.bar), greaterThanOrEqualTo(bodyText));
        expect(contrast(t.barRed, t.bar), greaterThanOrEqualTo(bodyText));
      });

      test('the hot end of the heat ramp reads as a mark', () {
        expect(contrast(t.heat2, t.panel), greaterThanOrEqualTo(largeOrMark));
      });

      test('the border line reads as a mark on the panel', () {
        expect(contrast(t.line, t.panel), greaterThanOrEqualTo(largeOrMark));
      });
    });
  }
}
