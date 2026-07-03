// Radio protocol (R-0032): military comms for the mic. "… over" ends a
// command immediately — no waiting for the engine's silence timeout — and
// "out" ends the conversation. Pure functions, shared by the sergeant and
// the coach.

final _overTail = RegExp(r'[,.!\s]*\bover\b[.!]?\s*$', caseSensitive: false);

/// True when a (partial) transcript ends with the "over" terminator.
bool endsWithOver(String transcript) => _overTail.hasMatch(transcript.trim());

/// Removes a trailing "over" terminator; returns the bare command.
String stripOver(String transcript) =>
    transcript.trim().replaceFirst(_overTail, '').trim();

/// True when the transcript is an "out" sign-off (word-boundary, so
/// "workout" never matches).
bool isOut(String transcript) =>
    RegExp(r'\bout\b', caseSensitive: false).hasMatch(transcript) &&
    !RegExp(r'\bworkout\b', caseSensitive: false).hasMatch(transcript);
