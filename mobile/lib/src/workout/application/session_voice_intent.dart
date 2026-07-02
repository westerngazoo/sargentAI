import '../../hub/voice_protocol.dart';

// Pure parser for in-session dictation (R-0032 slice) — "10 reps at 100
// kilos RPE 8", "next exercise", "finish workout" → typed intents the
// [VoiceCoach] applies to the [SessionDriver].

/// A recognised (or unrecognised) in-session voice command.
sealed class SessionVoiceIntent {
  const SessionVoiceIntent();
}

/// "10 reps at 100 kilos rpe 8" — any field may be absent; the driver's
/// validators remain the single enforcement point.
class LogSetIntent extends SessionVoiceIntent {
  const LogSetIntent({this.reps, this.weightKg, this.rpe});

  final int? reps;
  final double? weightKg;
  final double? rpe;
}

/// "next", "next exercise" → advance to the following exercise.
class NextExerciseIntent extends SessionVoiceIntent {
  const NextExerciseIntent();
}

/// "finish workout", "done", "save it" → submit the session.
class FinishSessionIntent extends SessionVoiceIntent {
  const FinishSessionIntent();
}

/// "pause", "stand by", "stop listening" → suspend the hands-free loop
/// without touching the session.
class PauseSessionIntent extends SessionVoiceIntent {
  const PauseSessionIntent();
}

/// Nothing matched — carries the transcript for the coach to echo back.
class UnknownSessionIntent extends SessionVoiceIntent {
  const UnknownSessionIntent(this.transcript);

  final String transcript;
}

/// Parses one dictation into a [SessionVoiceIntent]. Order: finish → next →
/// set (so "finish" inside a longer sentence is never read as a set).
SessionVoiceIntent parseSessionVoiceIntent(String transcript) {
  final t = transcript.toLowerCase().trim();
  if (t.isEmpty) return const UnknownSessionIntent('');

  if (_any(t, ['finish', 'save workout', 'end workout', 'we are done']) ||
      t == 'done') {
    return const FinishSessionIntent();
  }
  if (_any(t, ['pause', 'stand by', 'stop listening']) || isOut(t)) {
    return const PauseSessionIntent();
  }
  if (_any(t, ['next', 'skip'])) return const NextExerciseIntent();

  final reps = _firstInt(t, r'(\d+)\s*(?:reps?|repetitions?)');
  final weight =
      _firstDouble(t, r'(\d+(?:\.\d+)?)\s*(?:kg|kilos?|kilograms?)') ??
          _firstDouble(t, r'(?:at|with)\s+(\d+(?:\.\d+)?)');
  final rpe = _firstDouble(t, r'rpe\s*(\d+(?:\.\d+)?)');

  if (reps != null || weight != null || rpe != null) {
    return LogSetIntent(reps: reps, weightKg: weight, rpe: rpe);
  }

  // Bare numbers fallback: "10" → reps; "10 100" → reps then weight.
  final numbers =
      RegExp(r'\d+(?:\.\d+)?').allMatches(t).map((m) => m.group(0)!).toList();
  if (numbers.isNotEmpty) {
    return LogSetIntent(
      reps: int.tryParse(numbers[0]),
      weightKg: numbers.length > 1 ? double.tryParse(numbers[1]) : null,
    );
  }
  return UnknownSessionIntent(transcript);
}

bool _any(String t, List<String> keywords) => keywords.any(t.contains);

int? _firstInt(String t, String pattern) {
  final m = RegExp(pattern).firstMatch(t);
  return m == null ? null : int.tryParse(m.group(1)!);
}

double? _firstDouble(String t, String pattern) {
  final m = RegExp(pattern).firstMatch(t);
  return m == null ? null : double.tryParse(m.group(1)!);
}
