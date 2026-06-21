// Shared test doubles + helpers for the R-0014 program proposals suite.
//
// Authored by the qa agent in step 3 (TDD red) — BEFORE any
// `lib/src/program` production code exists. These imports resolve to the exact
// class/provider surface SPEC-0014 §2.5 names, so the suite is RED until
// step-5 implementation creates them, then turns GREEN with no test edits.
//
// Targeted production symbols (all under package:fitai/src/program/...):
//   models/program_proposal.dart  -> GeneratedProgram, GeneratedDiet,
//                                    ProgramProposal, ProposalsResponse
//   models/user_program.dart      -> UserProgram, ProgramHistoryResponse
//   services/program_service.dart -> ProgramService, programServiceProvider
//   presentation/program_proposals_screen.dart -> ProgramProposalsScreen
//   presentation/program_detail_screen.dart    -> ProgramDetailScreen
//   application/program_providers.dart         -> currentProgramProvider

import 'package:fitai/src/program/models/program_proposal.dart';
import 'package:fitai/src/program/models/user_program.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:mocktail/mocktail.dart';

/// Mock [ProgramService] — stubs getProposals / chooseProgram / getCurrent /
/// getHistory without touching the network.
class MockProgramService extends Mock implements ProgramService {}

// ---------------------------------------------------------------------------
// JSON factory helpers (keys mirror the Rust API wire shape exactly)
// ---------------------------------------------------------------------------

/// A `GeneratedProgram`-shaped JSON map.
Map<String, dynamic> generatedProgramJson({
  String split = '4-day split (delts/triceps, back, chest/biceps, legs)',
  int daysPerWeek = 4,
  int weeklyFrequencyPerMuscle = 1,
  String volume = 'low',
  String intensityGuidance = '1 all-out working set to failure',
  String restGuidance = '2-3 min between sets',
  String progressionGuidance = 'add load once rep target is reached',
  int estimatedSessionDurationMin = 45,
  List<String>? highlightExercises,
}) =>
    <String, dynamic>{
      'split': split,
      'days_per_week': daysPerWeek,
      'weekly_frequency_per_muscle': weeklyFrequencyPerMuscle,
      'volume': volume,
      'intensity_guidance': intensityGuidance,
      'rest_guidance': restGuidance,
      'progression_guidance': progressionGuidance,
      'estimated_session_duration_min': estimatedSessionDurationMin,
      'highlight_exercises': highlightExercises ??
          [
            'Bench Press',
            'Overhead Press',
            'Squat',
            'Barbell Row',
            'Deadlift',
            'Pull-up'
          ],
    };

/// A `GeneratedDiet`-shaped JSON map.
Map<String, dynamic> generatedDietJson({
  String approach = 'high-protein structured clean bulk',
  String calorieStrategy = 'moderate surplus for lean mass',
  String macroEmphasis = 'high_protein',
  String mealStructure = '~6 meals per day',
  int estimatedKcal = 3200,
  int proteinG = 176,
  int carbsG = 360,
  int fatG = 89,
}) =>
    <String, dynamic>{
      'approach': approach,
      'calorie_strategy': calorieStrategy,
      'macro_emphasis': macroEmphasis,
      'meal_structure': mealStructure,
      'estimated_kcal': estimatedKcal,
      'protein_g': proteinG,
      'carbs_g': carbsG,
      'fat_g': fatG,
    };

/// A `ProgramProposal`-shaped JSON map.
Map<String, dynamic> proposalJson({
  String archetypeId = 'heavy-duty-mass',
  String displayName = 'Low-Volume Mass Builder',
  String summary = 'Brief, brutally hard sessions.',
  double score = 0.92,
  double distance = 0.08,
  Map<String, dynamic>? program,
  Map<String, dynamic>? diet,
}) =>
    <String, dynamic>{
      'archetype_id': archetypeId,
      'display_name': displayName,
      'summary': summary,
      'score': score,
      'distance': distance,
      'program': program ?? generatedProgramJson(),
      'diet': diet ?? generatedDietJson(),
    };

/// Three-proposal `ProposalsResponse`-shaped JSON (the proposals endpoint body).
Map<String, dynamic> proposalsResponseJson({
  List<Map<String, dynamic>>? proposals,
}) =>
    <String, dynamic>{
      'proposals': proposals ??
          [
            proposalJson(
              archetypeId: 'heavy-duty-mass',
              displayName: 'Low-Volume Mass Builder',
              score: 0.92,
              distance: 0.08,
            ),
            proposalJson(
              archetypeId: 'high-intensity-minimalist',
              displayName: 'Minimalist High-Intensity',
              score: 0.80,
              distance: 0.20,
            ),
            proposalJson(
              archetypeId: 'powerbuilder-leverage',
              displayName: 'Compact Powerbuilder',
              score: 0.70,
              distance: 0.30,
            ),
          ],
    };

/// Parsed [ProposalsResponse] convenience builder.
ProposalsResponse sampleProposals() =>
    ProposalsResponse.fromJson(proposalsResponseJson());

/// A `UserProgram`-shaped JSON map (the choose / current / history body).
Map<String, dynamic> userProgramJson({
  String id = 'prog-uuid-001',
  String archetypeId = 'heavy-duty-mass',
  Map<String, dynamic>? program,
  Map<String, dynamic>? diet,
  bool active = true,
  String chosenAt = '2026-06-20T12:00:00Z',
}) =>
    <String, dynamic>{
      'id': id,
      'archetype_id': archetypeId,
      'program': program ?? generatedProgramJson(),
      'diet': diet ?? generatedDietJson(),
      'active': active,
      'chosen_at': chosenAt,
    };

/// Parsed [UserProgram] convenience builder.
UserProgram sampleUserProgram({
  String id = 'prog-uuid-001',
  String archetypeId = 'heavy-duty-mass',
  bool active = true,
}) =>
    UserProgram.fromJson(userProgramJson(
      id: id,
      archetypeId: archetypeId,
      active: active,
    ));

/// Registers mocktail fallbacks for non-primitive types used in `when()`/
/// `thenAnswer()` calls in the program test suite.
void registerProgramFallbacks() {
  // No non-primitive argument types needed for the current stubs;
  // extend here if a future stub requires `registerFallbackValue`.
}
