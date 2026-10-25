// R-0061 AC11: muscle regions and groups are shown by catalogue name, and the
// spoken "target" line joins them in the catalogue's own conjunction.

import 'package:fitai/src/workout/domain/muscle_activation.dart';
import 'package:fitai/src/workout/domain/muscle_group.dart';
import 'package:fitai/src/workout/application/muscle_names.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/l10n.dart';

void main() {
  test('every region and group has a catalogue name', () {
    for (final r in Region.values) {
      expect(regionName(r, l10n), isNotEmpty);
    }
    expect(regionName(Region.chest, l10n), l10n.regionChest);
    expect(regionName(Region.erectors, l10n), l10n.regionLowerBack);
    for (final g in MuscleGroup.values) {
      expect(muscleGroupName(g, l10n), isNotEmpty);
    }
    expect(muscleGroupName(MuscleGroup.legs, l10n), l10n.muscleLegs);
  });

  test('targetLabel reads naturally in the catalogue language', () {
    expect(targetLabel(activationFor('Bench press'), l10n),
        l10n.regionChest.toLowerCase());
    expect(
      targetLabel(activationFor('Barbell squat'), l10n),
      l10n.listAnd(
        l10n.regionQuads.toLowerCase(),
        l10n.regionGlutes.toLowerCase(),
      ),
    );
    expect(targetLabel(activationFor('Mystery movement'), l10n), isEmpty);
  });

  test('focusName names a plan day by its focus, never by a lift (SPEC-0061)',
      () {
    expect(focusName([MuscleGroup.legs], l10n), l10n.muscleLegs);
    expect(
      focusName([MuscleGroup.chest, MuscleGroup.arms], l10n),
      l10n.listAnd(l10n.muscleChest, l10n.muscleArms.toLowerCase()),
    );
    expect(
      focusName(
          [MuscleGroup.back, MuscleGroup.shoulders, MuscleGroup.core], l10n),
      l10n.listAnd(
        '${l10n.muscleBack}, ${l10n.muscleShoulders.toLowerCase()}',
        l10n.muscleCore.toLowerCase(),
      ),
    );
    expect(focusName(const [], l10n), isEmpty);
  });
}
