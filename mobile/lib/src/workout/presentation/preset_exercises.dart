import '../domain/muscle_group.dart';

/// A preset lift: a display name plus a suggested muscle group. **Presentation
/// only** — a UI convenience that pre-fills the same validated free-text path.
/// No client-side schema; slated for replacement by the M4 exercise library.
class PresetExercise {
  const PresetExercise(this.name, this.group);

  final String name;
  final MuscleGroup group;
}

/// ~20 common lifts. Every name passes the same validator a typed name passes.
const List<PresetExercise> presetExercises = <PresetExercise>[
  PresetExercise('Squat', MuscleGroup.legs),
  PresetExercise('Front squat', MuscleGroup.legs),
  PresetExercise('Leg press', MuscleGroup.legs),
  PresetExercise('Romanian deadlift', MuscleGroup.legs),
  PresetExercise('Lunge', MuscleGroup.legs),
  PresetExercise('Hip thrust', MuscleGroup.legs),
  PresetExercise('Leg curl', MuscleGroup.legs),
  PresetExercise('Calf raise', MuscleGroup.legs),
  PresetExercise('Bench press', MuscleGroup.chest),
  PresetExercise('Incline bench press', MuscleGroup.chest),
  PresetExercise('Dip', MuscleGroup.chest),
  PresetExercise('Deadlift', MuscleGroup.back),
  PresetExercise('Barbell row', MuscleGroup.back),
  PresetExercise('Pull-up', MuscleGroup.back),
  PresetExercise('Lat pulldown', MuscleGroup.back),
  PresetExercise('Overhead press', MuscleGroup.shoulders),
  PresetExercise('Lateral raise', MuscleGroup.shoulders),
  PresetExercise('Face pull', MuscleGroup.shoulders),
  PresetExercise('Biceps curl', MuscleGroup.arms),
  PresetExercise('Triceps extension', MuscleGroup.arms),
];
