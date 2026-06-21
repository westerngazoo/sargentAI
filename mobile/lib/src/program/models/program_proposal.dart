// R-0014 / SPEC-0014 §2.2 — program + diet proposal wire types.

class GeneratedProgram {
  const GeneratedProgram({
    required this.split,
    required this.daysPerWeek,
    required this.weeklyFrequencyPerMuscle,
    required this.volume,
    required this.intensityGuidance,
    required this.restGuidance,
    required this.progressionGuidance,
    required this.estimatedSessionDurationMin,
    required this.highlightExercises,
  });

  final String split;
  final int daysPerWeek;
  final int weeklyFrequencyPerMuscle;
  final String volume;
  final String intensityGuidance;
  final String restGuidance;
  final String progressionGuidance;
  final int estimatedSessionDurationMin;
  final List<String> highlightExercises;

  factory GeneratedProgram.fromJson(Map<String, dynamic> j) => GeneratedProgram(
        split: j['split'] as String,
        daysPerWeek: j['days_per_week'] as int,
        weeklyFrequencyPerMuscle: j['weekly_frequency_per_muscle'] as int,
        volume: j['volume'] as String,
        intensityGuidance: j['intensity_guidance'] as String,
        restGuidance: j['rest_guidance'] as String,
        progressionGuidance: j['progression_guidance'] as String,
        estimatedSessionDurationMin: j['estimated_session_duration_min'] as int,
        highlightExercises:
            List<String>.from(j['highlight_exercises'] as List<dynamic>),
      );
}

class GeneratedDiet {
  const GeneratedDiet({
    required this.approach,
    required this.calorieStrategy,
    required this.macroEmphasis,
    required this.mealStructure,
    required this.estimatedKcal,
    required this.proteinG,
    required this.carbsG,
    required this.fatG,
  });

  final String approach;
  final String calorieStrategy;
  final String macroEmphasis;
  final String mealStructure;
  final int estimatedKcal;
  final int proteinG;
  final int carbsG;
  final int fatG;

  factory GeneratedDiet.fromJson(Map<String, dynamic> j) => GeneratedDiet(
        approach: j['approach'] as String,
        calorieStrategy: j['calorie_strategy'] as String,
        macroEmphasis: j['macro_emphasis'] as String,
        mealStructure: j['meal_structure'] as String,
        estimatedKcal: j['estimated_kcal'] as int,
        proteinG: j['protein_g'] as int,
        carbsG: j['carbs_g'] as int,
        fatG: j['fat_g'] as int,
      );
}

class ProgramProposal {
  const ProgramProposal({
    required this.archetypeId,
    required this.displayName,
    required this.summary,
    required this.score,
    required this.distance,
    required this.program,
    required this.diet,
  });

  final String archetypeId;
  final String displayName;
  final String summary;
  final double score;
  final double distance;
  final GeneratedProgram program;
  final GeneratedDiet diet;

  factory ProgramProposal.fromJson(Map<String, dynamic> j) => ProgramProposal(
        archetypeId: j['archetype_id'] as String,
        displayName: j['display_name'] as String,
        summary: j['summary'] as String,
        score: (j['score'] as num).toDouble(),
        distance: (j['distance'] as num).toDouble(),
        program:
            GeneratedProgram.fromJson(j['program'] as Map<String, dynamic>),
        diet: GeneratedDiet.fromJson(j['diet'] as Map<String, dynamic>),
      );
}

/// Wraps the list returned by `GET /photo-sessions/:id/program-proposals`.
/// [ProposalsResponse.fromJson] accepts the `{'proposals': [...]}` shape used
/// by the test fakes; the service constructs it directly from the raw list.
class ProposalsResponse {
  const ProposalsResponse({required this.proposals});

  final List<ProgramProposal> proposals;

  factory ProposalsResponse.fromJson(Map<String, dynamic> j) =>
      ProposalsResponse(
        proposals: (j['proposals'] as List<dynamic>)
            .map((e) => ProgramProposal.fromJson(e as Map<String, dynamic>))
            .toList(),
      );
}
