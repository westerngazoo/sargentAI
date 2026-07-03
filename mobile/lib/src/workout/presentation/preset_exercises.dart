import '../domain/muscle_group.dart';

/// A preset lift: a display name plus a suggested muscle group. **Presentation
/// only** — a UI convenience that pre-fills the same validated free-text path.
/// Catalog ported from the-goose-factor routine data.
class PresetExercise {
  const PresetExercise(this.name, this.group);

  final String name;
  final MuscleGroup group;
}

/// ~20 common lifts. Every name passes the same validator a typed name passes.
const List<PresetExercise> presetExercises = <PresetExercise>[
  // Legs — squat pattern
  PresetExercise('Squat', MuscleGroup.legs),
  PresetExercise('Front squat', MuscleGroup.legs),
  PresetExercise('Box squat', MuscleGroup.legs),
  PresetExercise('Leg press', MuscleGroup.legs),
  PresetExercise('Hack squat', MuscleGroup.legs),
  PresetExercise('Bulgarian split squat', MuscleGroup.legs),
  PresetExercise('Walking lunge', MuscleGroup.legs),
  PresetExercise('Lunge', MuscleGroup.legs),
  PresetExercise('Goblet squat', MuscleGroup.legs),
  PresetExercise('Leg extension', MuscleGroup.legs),
  // Legs — hinge pattern
  PresetExercise('Deadlift', MuscleGroup.back),
  PresetExercise('Sumo deadlift', MuscleGroup.back),
  PresetExercise('Romanian deadlift', MuscleGroup.legs),
  PresetExercise('Trap bar deadlift', MuscleGroup.back),
  PresetExercise('Good morning', MuscleGroup.legs),
  PresetExercise('Hip thrust', MuscleGroup.legs),
  PresetExercise('Kettlebell swing', MuscleGroup.legs),
  PresetExercise('Glute ham raise', MuscleGroup.legs),
  PresetExercise('Leg curl', MuscleGroup.legs),
  PresetExercise('Cable pull-through', MuscleGroup.legs),
  PresetExercise('Back hyperextension', MuscleGroup.legs),
  PresetExercise('Calf raise', MuscleGroup.legs),
  PresetExercise('Seated calf raise', MuscleGroup.legs),
  // Chest
  PresetExercise('Bench press', MuscleGroup.chest),
  PresetExercise('Paused bench press', MuscleGroup.chest),
  PresetExercise('Close-grip bench press', MuscleGroup.chest),
  PresetExercise('Incline bench press', MuscleGroup.chest),
  PresetExercise('Incline dumbbell press', MuscleGroup.chest),
  PresetExercise('Flat dumbbell press', MuscleGroup.chest),
  PresetExercise('Machine chest press', MuscleGroup.chest),
  PresetExercise('Dip', MuscleGroup.chest),
  PresetExercise('Push-up', MuscleGroup.chest),
  PresetExercise('Cable fly', MuscleGroup.chest),
  PresetExercise('Pec deck', MuscleGroup.chest),
  // Back
  PresetExercise('Barbell row', MuscleGroup.back),
  PresetExercise('Pendlay row', MuscleGroup.back),
  PresetExercise('T-bar row', MuscleGroup.back),
  PresetExercise('Dumbbell row', MuscleGroup.back),
  PresetExercise('Seated cable row', MuscleGroup.back),
  PresetExercise('Chest-supported row', MuscleGroup.back),
  PresetExercise('Pull-up', MuscleGroup.back),
  PresetExercise('Chin-up', MuscleGroup.back),
  PresetExercise('Lat pulldown', MuscleGroup.back),
  PresetExercise('Straight-arm pulldown', MuscleGroup.back),
  PresetExercise('Shrug', MuscleGroup.back),
  // Shoulders
  PresetExercise('Overhead press', MuscleGroup.shoulders),
  PresetExercise('Push press', MuscleGroup.shoulders),
  PresetExercise('Seated dumbbell press', MuscleGroup.shoulders),
  PresetExercise('Arnold press', MuscleGroup.shoulders),
  PresetExercise('Lateral raise', MuscleGroup.shoulders),
  PresetExercise('Cable lateral raise', MuscleGroup.shoulders),
  PresetExercise('Front raise', MuscleGroup.shoulders),
  PresetExercise('Rear delt fly', MuscleGroup.shoulders),
  PresetExercise('Face pull', MuscleGroup.shoulders),
  PresetExercise('Upright row', MuscleGroup.shoulders),
  // Arms
  PresetExercise('Biceps curl', MuscleGroup.arms),
  PresetExercise('Hammer curl', MuscleGroup.arms),
  PresetExercise('Preacher curl', MuscleGroup.arms),
  PresetExercise('Incline dumbbell curl', MuscleGroup.arms),
  PresetExercise('Cable curl', MuscleGroup.arms),
  PresetExercise('Triceps extension', MuscleGroup.arms),
  PresetExercise('Skullcrusher', MuscleGroup.arms),
  PresetExercise('Triceps pushdown', MuscleGroup.arms),
  PresetExercise('Overhead triceps extension', MuscleGroup.arms),
  PresetExercise('Wrist curl', MuscleGroup.arms),
  // Core
  PresetExercise('Plank', MuscleGroup.core),
  PresetExercise('Hanging leg raise', MuscleGroup.core),
  PresetExercise('Cable crunch', MuscleGroup.core),
  PresetExercise('Ab wheel rollout', MuscleGroup.core),
  PresetExercise('Russian twist', MuscleGroup.core),
  PresetExercise('Side plank', MuscleGroup.core),
];
