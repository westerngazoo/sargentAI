// Activated-muscles mapping: curated lifts, group fallback, target labels.

import 'package:fitai/src/workout/domain/muscle_activation.dart';
import 'package:fitai/src/workout/domain/muscle_group.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('curated lift: bench press → chest primary, triceps assist', () {
    final a = activationFor('Bench press');
    expect(a.primary, contains(Region.chest));
    expect(a.secondary, contains(Region.triceps));
  });

  test('name lookup is case-insensitive', () {
    expect(activationFor('BARBELL SQUAT').primary, contains(Region.quads));
  });

  test('unknown lift falls back to its coarse group', () {
    final a = activationFor('Cable crossover xyz', group: MuscleGroup.chest);
    expect(a.primary, {Region.chest});
    expect(a.secondary, isEmpty);
  });

  test('unknown lift without a group is empty', () {
    expect(activationFor('Mystery movement').isEmpty, isTrue);
  });

  test('targetLabel reads naturally', () {
    expect(activationFor('Bench press').targetLabel, 'chest');
    expect(activationFor('Barbell squat').targetLabel, 'quads and glutes');
  });

  test('expanded catalog: new lifts resolve to curated activations', () {
    expect(activationFor('Hack squat').primary, contains(Region.quads));
    expect(activationFor('T-bar row').primary, contains(Region.lats));
    expect(activationFor('Skullcrusher').primary, {Region.triceps});
    expect(activationFor('Plank').primary, {Region.core});
  });
}
