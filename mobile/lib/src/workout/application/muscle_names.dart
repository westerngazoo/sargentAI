import '../../../l10n/app_localizations.dart';
import '../domain/muscle_activation.dart';
import '../domain/muscle_group.dart';

/// The catalogue name of an anatomy-chart region (R-0061 AC11).
String regionName(Region region, AppLocalizations l10n) => switch (region) {
      Region.neck => l10n.regionNeck,
      Region.traps => l10n.regionTraps,
      Region.shoulders => l10n.regionShoulders,
      Region.chest => l10n.regionChest,
      Region.biceps => l10n.regionBiceps,
      Region.triceps => l10n.regionTriceps,
      Region.forearms => l10n.regionForearms,
      Region.core => l10n.regionCore,
      Region.lats => l10n.regionLats,
      Region.upperBack => l10n.regionUpperBack,
      Region.erectors => l10n.regionLowerBack,
      Region.quads => l10n.regionQuads,
      Region.hamstrings => l10n.regionHamstrings,
      Region.glutes => l10n.regionGlutes,
      Region.calves => l10n.regionCalves,
    };

/// The catalogue name of a coarse muscle group; the wire token stays data.
String muscleGroupName(MuscleGroup group, AppLocalizations l10n) =>
    switch (group) {
      MuscleGroup.chest => l10n.muscleChest,
      MuscleGroup.back => l10n.muscleBack,
      MuscleGroup.shoulders => l10n.muscleShoulders,
      MuscleGroup.arms => l10n.muscleArms,
      MuscleGroup.legs => l10n.muscleLegs,
      MuscleGroup.core => l10n.muscleCore,
    };

/// A plan day's name from its focus (SPEC-0061 §2.1, §2.9): "Piernas",
/// "Pecho y brazos". Built on the client from the server's muscle-group
/// tokens, so the day is never named by a lift and AC11 holds; empty for an
/// empty focus.
String focusName(List<MuscleGroup> focus, AppLocalizations l10n) {
  final names = [
    for (final (i, g) in focus.indexed)
      i == 0
          ? muscleGroupName(g, l10n)
          : muscleGroupName(g, l10n).toLowerCase(),
  ];
  if (names.isEmpty) return '';
  if (names.length == 1) return names.first;
  return l10n.listAnd(
    names.sublist(0, names.length - 1).join(', '),
    names.last,
  );
}

/// The spoken target line for an activation: "chest", "quads and glutes";
/// empty for an unknown lift.
String targetLabel(MuscleActivation activation, AppLocalizations l10n) {
  final names =
      activation.primary.map((r) => regionName(r, l10n).toLowerCase()).toList();
  if (names.isEmpty) return '';
  if (names.length == 1) return names.first;
  return l10n.listAnd(
    names.sublist(0, names.length - 1).join(', '),
    names.last,
  );
}
