// R-0014 / SPEC-0014 §2.4 — persisted program wire types.

import 'program_proposal.dart';

class UserProgram {
  const UserProgram({
    required this.id,
    required this.archetypeId,
    this.sourceSessionId,
    required this.program,
    required this.diet,
    required this.active,
    required this.chosenAt,
  });

  final String id;
  final String archetypeId;
  final String? sourceSessionId;
  final GeneratedProgram program;
  final GeneratedDiet diet;
  final bool active;
  final DateTime chosenAt;

  factory UserProgram.fromJson(Map<String, dynamic> j) => UserProgram(
        id: j['id'] as String,
        archetypeId: j['archetype_id'] as String,
        sourceSessionId: j['source_session_id'] as String?,
        program:
            GeneratedProgram.fromJson(j['program'] as Map<String, dynamic>),
        diet: GeneratedDiet.fromJson(j['diet'] as Map<String, dynamic>),
        active: j['active'] as bool,
        chosenAt: DateTime.parse(j['chosen_at'] as String),
      );
}

class ProgramHistoryResponse {
  const ProgramHistoryResponse({
    required this.programs,
    required this.total,
    required this.limit,
    required this.offset,
  });

  final List<UserProgram> programs;
  final int total;
  final int limit;
  final int offset;

  factory ProgramHistoryResponse.fromJson(Map<String, dynamic> j) =>
      ProgramHistoryResponse(
        programs: (j['programs'] as List<dynamic>)
            .map((e) => UserProgram.fromJson(e as Map<String, dynamic>))
            .toList(),
        total: j['total'] as int,
        limit: j['limit'] as int,
        offset: j['offset'] as int,
      );
}
