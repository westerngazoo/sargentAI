// SPEC-0061 §2.8 (slice 3b): the line the loop speaks for an effect the
// server answered with. The server's `message` is English prose; the client
// says it in its own language (R-0061 AC11).

import '../../l10n/app_localizations.dart';
import 'voice_effect.dart';

/// The line spoken, and shown in the Dictado thread, when the server answers
/// with [effect].
String lineForEffect(VoiceEffect effect, AppLocalizations l10n) =>
    switch (effect) {
      OpenFocusMode() => l10n.sergeantStartingSession,
      OpenPanel(panel: PanelKind.programa) => l10n.sergeantOpeningProgram,
      OpenOnboarding() => l10n.sergeantOpeningProfile,
      Close() => l10n.sergeantRoger,
    };
