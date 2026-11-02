// R-0059 AC5 -> SPEC-0059 §2.4 (slices 59-c2a, 59-c2b): the caveats name
// every assumption the server states, which lengths came from the table,
// how the model places the arms, and that the legs between standing and the
// bottom follow the model's convention. Those three are the key lines under
// the figure; the assumptions are the list at the end. A token this client
// does not know still gets a line: an assumption is never hidden.

import 'package:fitai/src/physics/domain/caveat_text.dart';
import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/l10n.dart';
import '../../support/physics_fakes.dart';

/// Core's `Assumption` vocabulary (backend/crates/core/src/biomech/analysis.rs).
const _tokens = [
  'sagittal_plane',
  'quasi_static',
  'rigid_segments',
  'single_posture',
  'solved_baseline',
  'rigid_trunk_lumbar',
  'table_derived_masses',
  'table_derived_lengths',
  'knees_track_feet',
  'body_fat_ignored',
  'straight_body_upright',
  'joints_advance_together',
];

SquatRangeResult _stating(
  List<Assumption> range, {
  List<String> tableLengths = const [],
}) =>
    SquatRangeResult(
      end: const Bottom(),
      tableLengths: tableLengths,
      frames: squatGolden().frames,
      assumptions: range,
    );

void main() {
  final bottom = squatGolden().frames.last;
  final unknown = caveatLines(
    _stating(const [Assumption('a_token_from_the_future')]),
    bottom,
    l10n,
  ).assumptions;

  test('every core assumption has its own line, none the generic one', () {
    final lines = {
      for (final token in _tokens)
        [
          ...caveatLines(_stating([Assumption(token)]), bottom, l10n)
              .assumptions,
          token,
        ].first,
    };
    // The convention token is a key line, so it leaves no assumption line;
    // the token itself stands in for it here.
    expect(lines, hasLength(_tokens.length));
    expect(lines, isNot(contains(unknown.first)));
  });

  test('an unknown token still gets a line', () {
    expect(unknown.first, l10n.squatAssumption('other'));
  });

  test(
      'the key lines always say how the arms are placed and that the legs '
      'follow a convention', () {
    final key = caveatLines(_stating(const []), bottom, l10n).key;
    expect(key, [l10n.squatCaveatArms, l10n.squatCaveatConvention]);
  });

  test('the key lines name the lengths that came from the table, first', () {
    final key = caveatLines(
      _stating(
        const [Assumption('table_derived_lengths')],
        tableLengths: const ['thigh', 'trunk'],
      ),
      bottom,
      l10n,
    ).key;
    expect(key, [
      l10n.squatTableLengths(
        '${l10n.squatLength('thigh')}, ${l10n.squatLength('trunk')}',
      ),
      l10n.squatCaveatArms,
      l10n.squatCaveatConvention,
    ]);
  });

  test('a key line is not repeated in the assumptions', () {
    final caveats = caveatLines(
      _stating(
        const [
          Assumption('table_derived_lengths'),
          Assumption('joints_advance_together'),
          Assumption('quasi_static'),
        ],
        tableLengths: const ['thigh'],
      ),
      bottom,
      l10n,
    );
    expect(caveats.assumptions, [l10n.squatAssumption('quasi_static')]);
  });

  test('without table lengths, the table token keeps its generic line', () {
    final caveats = caveatLines(
        _stating(const [Assumption('table_derived_lengths')]), bottom, l10n);
    expect(
        caveats.assumptions, [l10n.squatAssumption('table_derived_lengths')]);
  });

  test('the range\'s and the frame\'s assumptions, each line once', () {
    final golden = squatGolden();
    final lines = caveatLines(golden, bottom, l10n).assumptions;
    expect(lines.toSet(), hasLength(lines.length));
    expect(lines, contains(l10n.squatAssumption('quasi_static')));
    expect(lines, contains(l10n.squatAssumption('straight_body_upright')));
  });
}
