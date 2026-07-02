// Activated-muscles model (ported from the-goose-factor MuscleMap) — which
// anatomy-chart regions an exercise lights up, primary vs secondary. Known
// lifts get curated activations; anything else falls back to its coarse
// [MuscleGroup].

import 'muscle_group.dart';

/// A region on the anatomy chart (front + back views).
enum Region {
  neck,
  traps,
  shoulders,
  chest,
  biceps,
  triceps,
  forearms,
  core,
  lats,
  upperBack,
  erectors,
  quads,
  hamstrings,
  glutes,
  calves,
}

const Map<Region, String> regionLabels = {
  Region.neck: 'Neck',
  Region.traps: 'Traps',
  Region.shoulders: 'Shoulders',
  Region.chest: 'Chest',
  Region.biceps: 'Biceps',
  Region.triceps: 'Triceps',
  Region.forearms: 'Forearms',
  Region.core: 'Core',
  Region.lats: 'Lats',
  Region.upperBack: 'Upper back',
  Region.erectors: 'Lower back',
  Region.quads: 'Quads',
  Region.hamstrings: 'Hamstrings',
  Region.glutes: 'Glutes',
  Region.calves: 'Calves',
};

/// Primary movers and secondary assisters for one exercise.
class MuscleActivation {
  const MuscleActivation({
    this.primary = const {},
    this.secondary = const {},
  });

  final Set<Region> primary;
  final Set<Region> secondary;

  bool get isEmpty => primary.isEmpty && secondary.isEmpty;

  /// Spoken/displayed target line, e.g. "chest" or "quads and glutes".
  String get targetLabel {
    final names = primary.map((r) => regionLabels[r]!.toLowerCase()).toList();
    if (names.isEmpty) return '';
    if (names.length == 1) return names.first;
    return '${names.sublist(0, names.length - 1).join(', ')} '
        'and ${names.last}';
  }
}

/// Curated activations for the preset/program lifts (source: the
/// goose-factor routine data). Keys are lowercase.
const Map<String, MuscleActivation> _byName = {
  'squat': MuscleActivation(
    primary: {Region.quads, Region.glutes},
    secondary: {Region.hamstrings, Region.core, Region.erectors},
  ),
  'barbell squat': MuscleActivation(
    primary: {Region.quads, Region.glutes},
    secondary: {Region.hamstrings, Region.core, Region.erectors},
  ),
  'front squat': MuscleActivation(
    primary: {Region.quads},
    secondary: {Region.glutes, Region.core},
  ),
  'leg press': MuscleActivation(
    primary: {Region.quads, Region.glutes},
    secondary: {Region.hamstrings},
  ),
  'romanian deadlift': MuscleActivation(
    primary: {Region.hamstrings, Region.glutes},
    secondary: {Region.erectors, Region.forearms},
  ),
  'lunge': MuscleActivation(
    primary: {Region.quads, Region.glutes},
    secondary: {Region.hamstrings, Region.core},
  ),
  'hip thrust': MuscleActivation(
    primary: {Region.glutes},
    secondary: {Region.hamstrings, Region.quads},
  ),
  'leg curl': MuscleActivation(primary: {Region.hamstrings}),
  'calf raise': MuscleActivation(primary: {Region.calves}),
  'deadlift': MuscleActivation(
    primary: {Region.hamstrings, Region.glutes, Region.erectors},
    secondary: {Region.lats, Region.traps, Region.forearms, Region.quads},
  ),
  'bench press': MuscleActivation(
    primary: {Region.chest},
    secondary: {Region.triceps, Region.shoulders},
  ),
  'incline bench press': MuscleActivation(
    primary: {Region.chest},
    secondary: {Region.shoulders, Region.triceps},
  ),
  'dip': MuscleActivation(
    primary: {Region.chest, Region.triceps},
    secondary: {Region.shoulders},
  ),
  'barbell row': MuscleActivation(
    primary: {Region.lats, Region.upperBack},
    secondary: {Region.biceps, Region.erectors, Region.traps},
  ),
  'pull-up': MuscleActivation(
    primary: {Region.lats},
    secondary: {Region.biceps, Region.upperBack},
  ),
  'lat pulldown': MuscleActivation(
    primary: {Region.lats},
    secondary: {Region.biceps},
  ),
  'overhead press': MuscleActivation(
    primary: {Region.shoulders},
    secondary: {Region.triceps, Region.traps, Region.core},
  ),
  'lateral raise': MuscleActivation(primary: {Region.shoulders}),
  'face pull': MuscleActivation(
    primary: {Region.upperBack, Region.shoulders},
    secondary: {Region.traps},
  ),
  'biceps curl': MuscleActivation(
    primary: {Region.biceps},
    secondary: {Region.forearms},
  ),
  'triceps extension': MuscleActivation(primary: {Region.triceps}),
  // Expanded catalog (the-goose-factor port)
  'box squat': MuscleActivation(
    primary: {Region.quads, Region.glutes},
    secondary: {Region.hamstrings, Region.erectors},
  ),
  'hack squat': MuscleActivation(
    primary: {Region.quads},
    secondary: {Region.glutes},
  ),
  'bulgarian split squat': MuscleActivation(
    primary: {Region.quads, Region.glutes},
    secondary: {Region.hamstrings, Region.core},
  ),
  'walking lunge': MuscleActivation(
    primary: {Region.quads, Region.glutes},
    secondary: {Region.hamstrings, Region.core},
  ),
  'goblet squat': MuscleActivation(
    primary: {Region.quads},
    secondary: {Region.glutes, Region.core},
  ),
  'leg extension': MuscleActivation(primary: {Region.quads}),
  'sumo deadlift': MuscleActivation(
    primary: {Region.glutes, Region.quads, Region.hamstrings},
    secondary: {Region.erectors, Region.traps, Region.forearms},
  ),
  'trap bar deadlift': MuscleActivation(
    primary: {Region.quads, Region.glutes, Region.hamstrings},
    secondary: {Region.erectors, Region.traps, Region.forearms},
  ),
  'good morning': MuscleActivation(
    primary: {Region.hamstrings, Region.erectors},
    secondary: {Region.glutes},
  ),
  'kettlebell swing': MuscleActivation(
    primary: {Region.glutes, Region.hamstrings},
    secondary: {Region.erectors, Region.core, Region.shoulders},
  ),
  'glute ham raise': MuscleActivation(
    primary: {Region.hamstrings},
    secondary: {Region.glutes, Region.calves},
  ),
  'cable pull-through': MuscleActivation(
    primary: {Region.glutes, Region.hamstrings},
    secondary: {Region.erectors},
  ),
  'back hyperextension': MuscleActivation(
    primary: {Region.erectors, Region.glutes},
    secondary: {Region.hamstrings},
  ),
  'seated calf raise': MuscleActivation(primary: {Region.calves}),
  'paused bench press': MuscleActivation(
    primary: {Region.chest},
    secondary: {Region.triceps, Region.shoulders},
  ),
  'close-grip bench press': MuscleActivation(
    primary: {Region.triceps, Region.chest},
    secondary: {Region.shoulders},
  ),
  'incline dumbbell press': MuscleActivation(
    primary: {Region.chest},
    secondary: {Region.shoulders, Region.triceps},
  ),
  'flat dumbbell press': MuscleActivation(
    primary: {Region.chest},
    secondary: {Region.triceps, Region.shoulders},
  ),
  'machine chest press': MuscleActivation(
    primary: {Region.chest},
    secondary: {Region.triceps},
  ),
  'push-up': MuscleActivation(
    primary: {Region.chest},
    secondary: {Region.triceps, Region.shoulders, Region.core},
  ),
  'cable fly': MuscleActivation(primary: {Region.chest}),
  'pec deck': MuscleActivation(primary: {Region.chest}),
  'pendlay row': MuscleActivation(
    primary: {Region.lats, Region.upperBack},
    secondary: {Region.biceps, Region.erectors},
  ),
  't-bar row': MuscleActivation(
    primary: {Region.lats, Region.upperBack},
    secondary: {Region.biceps, Region.erectors},
  ),
  'dumbbell row': MuscleActivation(
    primary: {Region.lats},
    secondary: {Region.biceps, Region.upperBack},
  ),
  'seated cable row': MuscleActivation(
    primary: {Region.lats, Region.upperBack},
    secondary: {Region.biceps},
  ),
  'chest-supported row': MuscleActivation(
    primary: {Region.upperBack, Region.lats},
    secondary: {Region.biceps},
  ),
  'chin-up': MuscleActivation(
    primary: {Region.lats, Region.biceps},
    secondary: {Region.upperBack},
  ),
  'straight-arm pulldown': MuscleActivation(
    primary: {Region.lats},
    secondary: {Region.triceps, Region.core},
  ),
  'shrug': MuscleActivation(
    primary: {Region.traps},
    secondary: {Region.forearms},
  ),
  'push press': MuscleActivation(
    primary: {Region.shoulders},
    secondary: {Region.triceps, Region.quads, Region.core},
  ),
  'seated dumbbell press': MuscleActivation(
    primary: {Region.shoulders},
    secondary: {Region.triceps},
  ),
  'arnold press': MuscleActivation(
    primary: {Region.shoulders},
    secondary: {Region.triceps},
  ),
  'cable lateral raise': MuscleActivation(primary: {Region.shoulders}),
  'front raise': MuscleActivation(primary: {Region.shoulders}),
  'rear delt fly': MuscleActivation(
    primary: {Region.shoulders},
    secondary: {Region.upperBack},
  ),
  'upright row': MuscleActivation(
    primary: {Region.shoulders, Region.traps},
    secondary: {Region.biceps},
  ),
  'hammer curl': MuscleActivation(
    primary: {Region.biceps},
    secondary: {Region.forearms},
  ),
  'preacher curl': MuscleActivation(primary: {Region.biceps}),
  'incline dumbbell curl': MuscleActivation(primary: {Region.biceps}),
  'cable curl': MuscleActivation(primary: {Region.biceps}),
  'skullcrusher': MuscleActivation(primary: {Region.triceps}),
  'triceps pushdown': MuscleActivation(primary: {Region.triceps}),
  'overhead triceps extension': MuscleActivation(primary: {Region.triceps}),
  'wrist curl': MuscleActivation(primary: {Region.forearms}),
  'plank': MuscleActivation(
    primary: {Region.core},
    secondary: {Region.shoulders, Region.glutes},
  ),
  'hanging leg raise': MuscleActivation(
    primary: {Region.core},
    secondary: {Region.forearms},
  ),
  'cable crunch': MuscleActivation(primary: {Region.core}),
  'ab wheel rollout': MuscleActivation(
    primary: {Region.core},
    secondary: {Region.lats, Region.shoulders},
  ),
  'russian twist': MuscleActivation(primary: {Region.core}),
  'side plank': MuscleActivation(
    primary: {Region.core},
    secondary: {Region.shoulders},
  ),
};

/// Coarse fallback when a lift isn't in the curated table.
const Map<MuscleGroup, Set<Region>> _byGroup = {
  MuscleGroup.chest: {Region.chest},
  MuscleGroup.back: {
    Region.lats,
    Region.upperBack,
    Region.traps,
    Region.erectors,
  },
  MuscleGroup.shoulders: {Region.shoulders},
  MuscleGroup.arms: {Region.biceps, Region.triceps, Region.forearms},
  MuscleGroup.legs: {
    Region.quads,
    Region.hamstrings,
    Region.glutes,
    Region.calves,
  },
  MuscleGroup.core: {Region.core},
};

/// Resolves an exercise (by name, then coarse group) to its activation.
MuscleActivation activationFor(String name, {MuscleGroup? group}) {
  final curated = _byName[name.trim().toLowerCase()];
  if (curated != null) return curated;
  final regions = group == null ? null : _byGroup[group];
  if (regions == null) return const MuscleActivation();
  return MuscleActivation(primary: regions);
}
