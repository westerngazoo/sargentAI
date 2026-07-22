// R-0017 — wire model for GET /adjustments. The `change` payload is kept as a
// raw map: v1 renders the rationale, and the tagged `kind` drives the icon;
// machine-applying changes is a later requirement.

import 'package:flutter/foundation.dart';

@immutable
class CoachSuggestion {
  const CoachSuggestion({
    required this.kind,
    required this.severity,
    required this.rationale,
  });

  factory CoachSuggestion.fromJson(Map<String, dynamic> json) {
    final change = json['change'] as Map<String, dynamic>? ?? const {};
    return CoachSuggestion(
      kind: change['kind'] as String? ?? 'unknown',
      severity: json['severity'] as String? ?? 'info',
      rationale: json['rationale'] as String? ?? '',
    );
  }

  /// Tagged change kind, e.g. `deload_lift`, `progress_lift`,
  /// `add_weekly_sets`, `reduce_days_per_week`, `adjust_kcal`.
  final String kind;

  /// `action` (do something) or `info` (nice-to-know).
  final String severity;

  final String rationale;

  bool get isAction => severity == 'action';
}

@immutable
class CoachAdvice {
  const CoachAdvice({
    required this.windowWeeks,
    required this.suggestions,
    this.reason,
  });

  factory CoachAdvice.fromJson(Map<String, dynamic> json) => CoachAdvice(
        windowWeeks: json['window_weeks'] as int? ?? 0,
        suggestions: (json['suggestions'] as List<dynamic>? ?? const [])
            .map((e) => CoachSuggestion.fromJson(e as Map<String, dynamic>))
            .toList(),
        reason: json['reason'] as String?,
      );

  final int windowWeeks;
  final List<CoachSuggestion> suggestions;

  /// `no_active_program` when the engine had nothing to advise against.
  final String? reason;

  bool get hasProgram => reason != 'no_active_program';
}
