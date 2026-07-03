// Anatomy chart (ported from the-goose-factor MuscleMap.tsx) — front and
// back body views with the active exercise's muscles lit: primary movers in
// the brand olive, assisters in brass. Geometry is the original hand-drawn
// SVG; fills are injected per region at build time.

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';

import '../domain/muscle_activation.dart';

class MuscleMap extends StatelessWidget {
  const MuscleMap({super.key, required this.activation});

  final MuscleActivation activation;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final primary = _rgba(cs.primary, 0.95);
    final secondary = _rgba(cs.tertiary, 0.85);
    const none = 'rgba(150,150,150,0.16)';
    const stroke = 'rgba(110,118,100,0.55)';

    String fill(Region r) {
      if (activation.primary.contains(r)) return primary;
      if (activation.secondary.contains(r)) return secondary;
      return none;
    }

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        SizedBox(
          height: 150,
          width: double.infinity,
          child: SvgPicture.string(
            _svg(fill, stroke, none),
            fit: BoxFit.contain,
          ),
        ),
        const SizedBox(height: 6),
        Wrap(
          spacing: 6,
          runSpacing: 4,
          alignment: WrapAlignment.center,
          children: [
            for (final r in activation.primary)
              _tag(context, regionLabels[r]!, cs.primary, cs.onPrimary),
            for (final r in activation.secondary)
              _tag(context, regionLabels[r]!, cs.tertiary, cs.surface),
          ],
        ),
      ],
    );
  }

  Widget _tag(BuildContext context, String label, Color bg, Color fg) =>
      Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
        decoration: BoxDecoration(
          color: bg,
          borderRadius: BorderRadius.circular(999),
        ),
        child: Text(
          label,
          style: Theme.of(context)
              .textTheme
              .labelSmall
              ?.copyWith(color: fg, fontWeight: FontWeight.w700),
        ),
      );

  static String _rgba(Color c, double alpha) {
    final r = (c.r * 255).round();
    final g = (c.g * 255).round();
    final b = (c.b * 255).round();
    return 'rgba($r,$g,$b,$alpha)';
  }

  String _svg(String Function(Region) f, String stroke, String none) => '''
<svg viewBox="0 0 360 345" xmlns="http://www.w3.org/2000/svg">
  <!-- FRONT -->
  <g stroke="$stroke" stroke-width="1">
    <circle cx="90" cy="34" r="16" fill="$none"/>
    <rect x="84" y="48" width="12" height="8" fill="${f(Region.neck)}"/>
    <path d="M67 58 Q90 48 113 58 L107 64 Q90 56 73 64 Z" fill="${f(Region.traps)}"/>
    <path d="M54 64 Q62 57 70 64 L68 78 Q58 78 52 74 Z" fill="${f(Region.shoulders)}"/>
    <path d="M110 64 Q118 57 126 64 L128 74 Q122 78 112 78 Z" fill="${f(Region.shoulders)}"/>
    <ellipse cx="46" cy="72" rx="6" ry="8" fill="${f(Region.shoulders)}"/>
    <ellipse cx="134" cy="72" rx="6" ry="8" fill="${f(Region.shoulders)}"/>
    <path d="M72 70 Q90 66 108 70 L106 78 Q90 74 74 78 Z" fill="${f(Region.chest)}"/>
    <path d="M74 80 Q90 78 106 80 L108 90 Q90 88 72 90 Z" fill="${f(Region.chest)}"/>
    <path d="M73 92 Q90 92 107 92 L105 102 Q90 106 75 102 Z" fill="${f(Region.chest)}"/>
    <ellipse cx="38" cy="100" rx="5" ry="18" fill="${f(Region.biceps)}"/>
    <ellipse cx="47" cy="100" rx="5" ry="18" fill="${f(Region.biceps)}"/>
    <ellipse cx="133" cy="100" rx="5" ry="18" fill="${f(Region.biceps)}"/>
    <ellipse cx="142" cy="100" rx="5" ry="18" fill="${f(Region.biceps)}"/>
    <ellipse cx="42" cy="122" rx="5" ry="5" fill="${f(Region.biceps)}"/>
    <ellipse cx="138" cy="122" rx="5" ry="5" fill="${f(Region.biceps)}"/>
    <ellipse cx="42" cy="152" rx="8" ry="22" fill="${f(Region.forearms)}"/>
    <ellipse cx="138" cy="152" rx="8" ry="22" fill="${f(Region.forearms)}"/>
    <rect x="76" y="108" width="28" height="22" rx="3" fill="${f(Region.core)}"/>
    <rect x="76" y="132" width="28" height="22" rx="3" fill="${f(Region.core)}"/>
    <path d="M64 112 L76 114 L76 152 L66 148 Z" fill="${f(Region.core)}"/>
    <path d="M116 112 L104 114 L104 152 L114 148 Z" fill="${f(Region.core)}"/>
    <rect x="70" y="156" width="40" height="10" rx="3" fill="$none"/>
    <path d="M62 172 Q66 168 72 172 L70 230 Q66 232 60 228 Z" fill="${f(Region.quads)}"/>
    <path d="M72 172 Q78 168 84 172 L82 234 Q76 234 72 232 Z" fill="${f(Region.quads)}"/>
    <path d="M73 215 Q80 218 87 215 L86 240 Q79 246 73 240 Z" fill="${f(Region.quads)}"/>
    <path d="M118 172 Q114 168 108 172 L110 230 Q114 232 120 228 Z" fill="${f(Region.quads)}"/>
    <path d="M108 172 Q102 168 96 172 L98 234 Q104 234 108 232 Z" fill="${f(Region.quads)}"/>
    <path d="M107 215 Q100 218 93 215 L94 240 Q101 246 107 240 Z" fill="${f(Region.quads)}"/>
    <ellipse cx="73" cy="285" rx="10" ry="28" fill="$none"/>
    <ellipse cx="107" cy="285" rx="10" ry="28" fill="$none"/>
    <ellipse cx="73" cy="324" rx="11" ry="5" fill="$none"/>
    <ellipse cx="107" cy="324" rx="11" ry="5" fill="$none"/>
  </g>
  <!-- BACK -->
  <g stroke="$stroke" stroke-width="1">
    <circle cx="270" cy="34" r="16" fill="$none"/>
    <rect x="264" y="48" width="12" height="8" fill="${f(Region.neck)}"/>
    <path d="M238 50 Q270 40 302 50 L294 70 Q270 60 246 70 Z" fill="${f(Region.traps)}"/>
    <path d="M254 76 L286 76 L278 108 L262 108 Z" fill="${f(Region.traps)}"/>
    <ellipse cx="232" cy="72" rx="9" ry="8" fill="${f(Region.shoulders)}"/>
    <ellipse cx="308" cy="72" rx="9" ry="8" fill="${f(Region.shoulders)}"/>
    <path d="M238 84 Q248 80 254 86 L252 96 Q242 96 238 94 Z" fill="${f(Region.upperBack)}"/>
    <path d="M302 84 Q292 80 286 86 L288 96 Q298 96 302 94 Z" fill="${f(Region.upperBack)}"/>
    <rect x="260" y="78" width="20" height="18" rx="2" fill="${f(Region.upperBack)}"/>
    <path d="M244 98 Q270 92 296 98 L292 145 Q270 156 248 145 Z" fill="${f(Region.lats)}"/>
    <rect x="263" y="112" width="14" height="38" rx="2" fill="${f(Region.erectors)}"/>
    <ellipse cx="213" cy="98" rx="5" ry="18" fill="${f(Region.triceps)}"/>
    <ellipse cx="222" cy="98" rx="5" ry="18" fill="${f(Region.triceps)}"/>
    <ellipse cx="217" cy="122" rx="5" ry="6" fill="${f(Region.triceps)}"/>
    <ellipse cx="318" cy="98" rx="5" ry="18" fill="${f(Region.triceps)}"/>
    <ellipse cx="327" cy="98" rx="5" ry="18" fill="${f(Region.triceps)}"/>
    <ellipse cx="323" cy="122" rx="5" ry="6" fill="${f(Region.triceps)}"/>
    <ellipse cx="216" cy="152" rx="8" ry="22" fill="${f(Region.forearms)}"/>
    <ellipse cx="324" cy="152" rx="8" ry="22" fill="${f(Region.forearms)}"/>
    <path d="M244 120 L254 124 L254 150 L246 148 Z" fill="${f(Region.core)}"/>
    <path d="M296 120 L286 124 L286 150 L294 148 Z" fill="${f(Region.core)}"/>
    <ellipse cx="246" cy="164" rx="8" ry="8" fill="${f(Region.glutes)}"/>
    <ellipse cx="294" cy="164" rx="8" ry="8" fill="${f(Region.glutes)}"/>
    <ellipse cx="258" cy="186" rx="14" ry="16" fill="${f(Region.glutes)}"/>
    <ellipse cx="282" cy="186" rx="14" ry="16" fill="${f(Region.glutes)}"/>
    <ellipse cx="250" cy="232" rx="8" ry="30" fill="${f(Region.hamstrings)}"/>
    <ellipse cx="266" cy="232" rx="8" ry="30" fill="${f(Region.hamstrings)}"/>
    <ellipse cx="274" cy="232" rx="8" ry="30" fill="${f(Region.hamstrings)}"/>
    <ellipse cx="290" cy="232" rx="8" ry="30" fill="${f(Region.hamstrings)}"/>
    <ellipse cx="258" cy="285" rx="10" ry="18" fill="${f(Region.calves)}"/>
    <ellipse cx="282" cy="285" rx="10" ry="18" fill="${f(Region.calves)}"/>
    <ellipse cx="258" cy="310" rx="8" ry="10" fill="${f(Region.calves)}"/>
    <ellipse cx="282" cy="310" rx="8" ry="10" fill="${f(Region.calves)}"/>
    <ellipse cx="258" cy="332" rx="11" ry="5" fill="$none"/>
    <ellipse cx="282" cy="332" rx="11" ry="5" fill="$none"/>
  </g>
</svg>
''';
}
