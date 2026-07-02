// R-0032 (slice 1) — pure voice-intent parser.
//
// Maps a speech transcript to a [VoiceIntent] with plain keyword matching.
// Deliberately dumb: the LLM-backed parser (R-0032 AC3) replaces this later;
// the sealed intent type is the seam that stays.

/// A recognised (or unrecognised) voice command.
sealed class VoiceIntent {
  const VoiceIntent();
}

/// "log a workout", "start session", "train" → live session screen.
class LogWorkoutIntent extends VoiceIntent {
  const LogWorkoutIntent();
}

/// "log a meal", "I ate …" → meal quick-log sheet, macros prefilled when
/// the transcript carries them ("40 grams of protein, 60 carbs, 20 fat").
class LogMealIntent extends VoiceIntent {
  const LogMealIntent({this.proteinG, this.carbsG, this.fatG});

  final double? proteinG;
  final double? carbsG;
  final double? fatG;
}

/// "show my program" → program detail screen.
class ShowProgramIntent extends VoiceIntent {
  const ShowProgramIntent();
}

/// "body type", "match me" → body-type picker (R-0030).
class BodyMatchIntent extends VoiceIntent {
  const BodyMatchIntent();
}

/// "history", "my sessions" → home session list.
class ShowHistoryIntent extends VoiceIntent {
  const ShowHistoryIntent();
}

/// "profile", "my details" → onboarding/profile screen.
class ShowProfileIntent extends VoiceIntent {
  const ShowProfileIntent();
}

/// Nothing matched — carries the transcript so the UI can echo it back.
class UnknownIntent extends VoiceIntent {
  const UnknownIntent(this.transcript);

  final String transcript;
}

/// Hub ring labels — kept in sync with [VoiceHubScreen] option order.
const hubOptionLabels = [
  'Workout',
  'Meal',
  'Program',
  'Body match',
  'History',
  'Profile',
];

/// Maps a parsed intent to the hub ring label it activates, if any.
String? hubOptionLabelForIntent(VoiceIntent intent) => switch (intent) {
      LogWorkoutIntent() => 'Workout',
      LogMealIntent() => 'Meal',
      ShowProgramIntent() => 'Program',
      BodyMatchIntent() => 'Body match',
      ShowHistoryIntent() => 'History',
      ShowProfileIntent() => 'Profile',
      UnknownIntent() => null,
    };

/// Best-effort match for live ring highlighting while the user dictates.
/// Checks spoken option names first, then keyword intent parsing.
String? matchedHubOptionLabel(String transcript) {
  final t = transcript.toLowerCase().trim();
  if (t.isEmpty) return null;
  for (final label in hubOptionLabels) {
    if (t.contains(label.toLowerCase())) return label;
  }
  return hubOptionLabelForIntent(parseVoiceIntent(transcript));
}

/// Parses a transcript into a [VoiceIntent]. Case-insensitive, first match
/// wins in the order meal → workout → body match → program → history →
/// profile (meal before workout so "log my lunch workout shake" stays food).
VoiceIntent parseVoiceIntent(String transcript) {
  final t = transcript.toLowerCase().trim();
  if (t.isEmpty) return const UnknownIntent('');

  if (_matchesAny(t, [
    'meal',
    'food',
    'eat',
    'ate',
    'lunch',
    'dinner',
    'breakfast',
    'nutrition',
    'macro'
  ])) {
    return LogMealIntent(
      proteinG: _grams(t, 'protein'),
      carbsG: _grams(t, 'carb'),
      fatG: _grams(t, 'fat'),
    );
  }
  if (_matchesAny(
      t, ['workout', 'session', 'train', 'exercise', 'gym', 'lift'])) {
    return const LogWorkoutIntent();
  }
  if (_matchesAny(t, ['body type', 'body match', 'match me', 'find my type'])) {
    return const BodyMatchIntent();
  }
  if (_matchesAny(t, ['program', 'plan', 'routine'])) {
    return const ShowProgramIntent();
  }
  if (_matchesAny(t, ['history', 'past', 'log list', 'sessions'])) {
    return const ShowHistoryIntent();
  }
  if (_matchesAny(t, ['profile', 'my details', 'settings', 'account'])) {
    return const ShowProfileIntent();
  }
  return UnknownIntent(transcript);
}

bool _matchesAny(String t, List<String> keywords) => keywords.any(t.contains);

/// Extracts `"<number> [g|gram|grams] [of] <macro>"` or `"<macro> <number>"`.
double? _grams(String t, String macro) {
  final before =
      RegExp(r'(\d+(?:\.\d+)?)\s*(?:g|grams?)?\s*(?:of\s+)?' + macro);
  final after = RegExp(macro + r'[a-z]*\s+(\d+(?:\.\d+)?)');
  final m = before.firstMatch(t) ?? after.firstMatch(t);
  return m == null ? null : double.tryParse(m.group(1)!);
}
