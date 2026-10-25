import 'package:flutter/material.dart';

/// The @fisicobuenfisico palette (R-0061 AC12, SPEC-0061 §2.10): black ink,
/// white paper and one colour, red, for force, torque, the hot end of the
/// tension ramp, the primary action and the stamp.
///
/// Carried as a `ThemeExtension` so widgets read it from `Theme.of(context)`
/// and the two modes stay in one place. Heat means tension magnitude and
/// nothing else: no token means good or bad (R-0045 AC14).
@immutable
class FbfTokens extends ThemeExtension<FbfTokens> {
  const FbfTokens({
    required this.ground,
    required this.panel,
    required this.panelAlt,
    required this.text,
    required this.muted,
    required this.line,
    required this.red,
    required this.onRed,
    required this.heat0,
    required this.heat1,
    required this.heat2,
    required this.bar,
    required this.barText,
    required this.barRed,
  });

  /// Background, under a faint halftone.
  final Color ground;

  /// Cards and panels, with a hard 2 px border in [line].
  final Color panel;

  /// A second panel tone for nested surfaces.
  final Color panelAlt;

  /// Body text.
  final Color text;

  /// Secondary text; body-text legible on [panel].
  final Color muted;

  /// Borders and the figure's ink.
  final Color line;

  /// The only colour. Passes as text on [panel] and [ground].
  final Color red;

  /// Text on a [red] fill.
  final Color onRed;

  /// Tension ramp, low to high. [heat0] and [heat1] colour marks only, never
  /// text; [heat2] is [red].
  final Color heat0;
  final Color heat1;
  final Color heat2;

  /// The CONCEPTO footer and focus mode, dark in both modes.
  final Color bar;
  final Color barText;
  final Color barRed;

  static const light = FbfTokens(
    ground: Color(0xFFF4F3F1),
    panel: Color(0xFFFFFFFF),
    panelAlt: Color(0xFFF1F0EE),
    text: Color(0xFF111111),
    muted: Color(0xFF505050),
    line: Color(0xFF111111),
    red: Color(0xFFC8102E),
    onRed: Color(0xFFFFFFFF),
    heat0: Color(0xFFBDBDBD),
    heat1: Color(0xFF7A7A7A),
    heat2: Color(0xFFC8102E),
    bar: Color(0xFF0B0B0B),
    barText: Color(0xFFF2F2F2),
    barRed: Color(0xFFFF4D4D),
  );

  static const dark = FbfTokens(
    ground: Color(0xFF0B0B0B),
    panel: Color(0xFF161616),
    panelAlt: Color(0xFF202020),
    text: Color(0xFFF2F2F2),
    muted: Color(0xFFABABAB),
    line: Color(0xFFD9D9D9),
    red: Color(0xFFFF4D4D),
    onRed: Color(0xFF0B0B0B),
    heat0: Color(0xFF4D4D4D),
    heat1: Color(0xFF8F8F8F),
    heat2: Color(0xFFFF4D4D),
    bar: Color(0xFF0B0B0B),
    barText: Color(0xFFF2F2F2),
    barRed: Color(0xFFFF4D4D),
  );

  /// The tension colour for a magnitude in `0..=1`.
  Color heat(double t) {
    final x = t.clamp(0.0, 1.0);
    return x < 0.5
        ? Color.lerp(heat0, heat1, x * 2)!
        : Color.lerp(heat1, heat2, (x - 0.5) * 2)!;
  }

  @override
  FbfTokens copyWith() => this;

  @override
  FbfTokens lerp(FbfTokens? other, double t) {
    if (other == null) return this;
    Color mix(Color a, Color b) => Color.lerp(a, b, t)!;
    return FbfTokens(
      ground: mix(ground, other.ground),
      panel: mix(panel, other.panel),
      panelAlt: mix(panelAlt, other.panelAlt),
      text: mix(text, other.text),
      muted: mix(muted, other.muted),
      line: mix(line, other.line),
      red: mix(red, other.red),
      onRed: mix(onRed, other.onRed),
      heat0: mix(heat0, other.heat0),
      heat1: mix(heat1, other.heat1),
      heat2: mix(heat2, other.heat2),
      bar: mix(bar, other.bar),
      barText: mix(barText, other.barText),
      barRed: mix(barRed, other.barRed),
    );
  }
}

/// The bundled faces (SPEC-0061 §2.10), declared in pubspec.yaml under these
/// family names. Nothing is fetched at run time.
abstract final class AppFonts {
  /// Short comic headlines (Bangers, OFL).
  static const headline = 'Bangers';

  /// Big readout numbers (Luckiest Guy, Apache 2.0): Bangers draws 1 and 7
  /// alike, which is why the reels switch faces for digits.
  static const big = 'LuckiestGuy';

  /// Data, setup lines and the formula bar (IBM Plex Mono, OFL).
  static const mono = 'IBMPlexMono';

  /// UI text (Archivo, OFL).
  static const ui = 'Archivo';
}
