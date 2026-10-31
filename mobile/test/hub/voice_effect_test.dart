// R-0061 AC2 -> SPEC-0061 §2.8 (slice 3b): the wire `effect` token becomes a
// typed [VoiceEffect]. Each token maps; the deprecated `route` of a server
// still answering with one maps to the same values; anything else closes
// the sheet. The line spoken for each effect is the client's own, in its
// language, never the server's prose.

import 'package:fitai/src/hub/effect_lines.dart';
import 'package:fitai/src/hub/voice_effect.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/l10n.dart';

void main() {
  group('parseVoiceEffect', () {
    test('each effect token', () {
      expect(
          parseVoiceEffect(effect: 'open_focus_mode'), const OpenFocusMode());
      expect(parseVoiceEffect(effect: 'open_program'),
          const OpenPanel(PanelKind.programa));
      expect(
          parseVoiceEffect(effect: 'open_onboarding'), const OpenOnboarding());
      expect(parseVoiceEffect(effect: 'close'), const Close());
    });

    test('a legacy route alone maps to the same values', () {
      expect(parseVoiceEffect(route: '/session'), const OpenFocusMode());
      expect(parseVoiceEffect(route: '/programs/current'),
          const OpenPanel(PanelKind.programa));
      expect(parseVoiceEffect(route: '/programs/get'),
          const OpenPanel(PanelKind.programa));
      expect(parseVoiceEffect(route: '/onboarding'), const OpenOnboarding());
      expect(parseVoiceEffect(route: '/home'), const Close());
    });

    test('the token wins over the route', () {
      expect(
          parseVoiceEffect(effect: 'close', route: '/session'), const Close());
    });

    test('garbage and nothing close', () {
      expect(parseVoiceEffect(), const Close());
      expect(parseVoiceEffect(effect: 'open_sesame'), const Close());
      expect(parseVoiceEffect(route: '/nowhere'), const Close());
      expect(parseVoiceEffect(effect: '', route: ''), const Close());
    });

    test('effects are values', () {
      expect(const OpenPanel(PanelKind.programa),
          equals(OpenPanel(PanelKind.programa)));
      expect(const OpenPanel(PanelKind.programa).hashCode,
          OpenPanel(PanelKind.programa).hashCode);
    });
  });

  group('lineForEffect', () {
    test('every effect has a line in the catalogue', () {
      expect(lineForEffect(const OpenFocusMode(), l10n),
          l10n.sergeantStartingSession);
      expect(lineForEffect(const OpenPanel(PanelKind.programa), l10n),
          l10n.sergeantOpeningProgram);
      expect(lineForEffect(const OpenOnboarding(), l10n),
          l10n.sergeantOpeningProfile);
      expect(lineForEffect(const Close(), l10n), l10n.sergeantRoger);
    });
  });
}
