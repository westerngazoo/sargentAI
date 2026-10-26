// Unit tests for the voice intent service response model.
//
// R-0061 AC2 -> SPEC-0061 §2.8 (slice 3b): a `navigate` answer exposes its
// `effect`, parsed from the wire token or, for a server still answering with
// only a `route`, from that; never a route string. The server's `message`
// stays on the model but nothing speaks it.

import 'package:fitai/src/hub/voice_effect.dart';
import 'package:fitai/src/hub/voice_intent_service.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('VoiceIntentResult.fromJson parses logged workout', () {
    final r = VoiceIntentResult.fromJson({
      'status': 'logged_workout',
      'message': 'Logged bench press.',
      'record_id': 'abc-123',
    });
    expect(r.isLoggedWorkout, isTrue);
    expect(r.message, 'Logged bench press.');
    expect(r.recordId, 'abc-123');
    expect(r.effect, isNull);
  });

  test('VoiceIntentResult.fromJson parses clarify', () {
    final r = VoiceIntentResult.fromJson({
      'status': 'clarify',
      'prompt': 'How many grams of protein?',
    });
    expect(r.isClarify, isTrue);
    expect(r.prompt, 'How many grams of protein?');
    expect(r.effect, isNull);
  });

  group('a navigate answer carries an effect', () {
    VoiceIntentResult navigate(Map<String, dynamic> fields) =>
        VoiceIntentResult.fromJson({
          'status': 'navigate',
          'message': 'Opening.',
          ...fields,
        });

    test('each token, beside the deprecated route', () {
      expect(
          navigate({'effect': 'open_focus_mode', 'route': '/session'}).effect,
          const OpenFocusMode());
      expect(
          navigate({'effect': 'open_program', 'route': '/programs/current'})
              .effect,
          const OpenPanel(PanelKind.programa));
      expect(
          navigate({'effect': 'open_onboarding', 'route': '/onboarding'})
              .effect,
          const OpenOnboarding());
      expect(navigate({'effect': 'close', 'route': '/home'}).effect,
          const Close());
    });

    test('a legacy route alone', () {
      expect(navigate({'route': '/session'}).effect, const OpenFocusMode());
      expect(navigate({'route': '/programs/get'}).effect,
          const OpenPanel(PanelKind.programa));
    });

    test('garbage closes', () {
      expect(navigate({'effect': 42}).effect, const Close());
      expect(navigate({}).effect, const Close());
    });
  });
}
