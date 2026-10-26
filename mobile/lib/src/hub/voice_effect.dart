// SPEC-0061 §2.8 (slice 3b): what a voice answer asks the screen to do, as a
// value. The server names it with an `effect` token, the local intents emit
// the same values, and the screen that opened the Dictado sheet performs it
// once the sheet has closed. No route string survives past the wire.

/// The panels a voice effect can open. One today: the choose flow and the
/// body match live inside Programa, so every program intent lands there.
enum PanelKind { programa }

/// A navigation asked for by voice (SPEC-0061 §2.8).
sealed class VoiceEffect {
  const VoiceEffect();
}

/// Start today's session, seeded from the schedule, and open focus mode.
final class OpenFocusMode extends VoiceEffect {
  const OpenFocusMode();
}

/// Open [panel] over the main screen.
final class OpenPanel extends VoiceEffect {
  const OpenPanel(this.panel);

  final PanelKind panel;

  @override
  bool operator ==(Object other) => other is OpenPanel && other.panel == panel;

  @override
  int get hashCode => panel.hashCode;
}

/// Open onboarding, to edit the profile.
final class OpenOnboarding extends VoiceEffect {
  const OpenOnboarding();
}

/// Close the sheet and nothing more: the calendar is behind it.
final class Close extends VoiceEffect {
  const Close();
}

/// The effect of a `navigate` answer: its [effect] token, or, for a server
/// still answering with only the deprecated [route], the effect that route
/// stood for. A present token is final, so an unknown one closes; so does an
/// unknown route, or nothing at all.
VoiceEffect parseVoiceEffect({String? effect, String? route}) =>
    switch (effect) {
      'open_focus_mode' => const OpenFocusMode(),
      'open_program' => const OpenPanel(PanelKind.programa),
      'open_onboarding' => const OpenOnboarding(),
      'close' => const Close(),
      null => switch (route) {
          '/session' => const OpenFocusMode(),
          '/programs/current' ||
          '/programs/get' =>
            const OpenPanel(PanelKind.programa),
          '/onboarding' => const OpenOnboarding(),
          _ => const Close(),
        },
      _ => const Close(),
    };
