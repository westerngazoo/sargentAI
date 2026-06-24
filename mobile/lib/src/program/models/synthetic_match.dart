// R-0030 — synthetic body-type match wire types.

import 'program_proposal.dart';

/// The three coarse body shapes shown in the picker grid.
enum BodyShape {
  ectomorph,
  mesomorph,
  endomorph;

  String get label => switch (this) {
        BodyShape.ectomorph => 'Lean & Narrow',
        BodyShape.mesomorph => 'Athletic & Broad',
        BodyShape.endomorph => 'Stocky & Solid',
      };

  String get description => switch (this) {
        BodyShape.ectomorph =>
          'Slender frame, narrow shoulders, long limbs, low body mass.',
        BodyShape.mesomorph =>
          'Wide shoulders, muscular build, responds well to training.',
        BodyShape.endomorph =>
          'Stocky build, broad waist, tends to carry more mass.',
      };

  String get value => name;
}

/// The three coarse body-fat bands.
enum FatBand {
  lean,
  moderate,
  bulky;

  String get label => switch (this) {
        FatBand.lean => 'Lean',
        FatBand.moderate => 'Moderate',
        FatBand.bulky => 'Bulky',
      };

  String get sublabel => switch (this) {
        FatBand.lean => 'Visible muscle definition',
        FatBand.moderate => 'Some definition, moderate fat',
        FatBand.bulky => 'Higher body fat, full look',
      };

  String get value => name;
}

/// Response from `POST /match/synthetic`.
class SyntheticMatchResponse {
  const SyntheticMatchResponse({
    required this.shape,
    required this.fatBand,
    required this.proposals,
  });

  final String shape;
  final String fatBand;
  final List<ProgramProposal> proposals;

  factory SyntheticMatchResponse.fromJson(Map<String, dynamic> j) =>
      SyntheticMatchResponse(
        shape: j['shape'] as String,
        fatBand: j['fat_band'] as String,
        proposals: (j['proposals'] as List<dynamic>)
            .map((e) => ProgramProposal.fromJson(e as Map<String, dynamic>))
            .toList(),
      );
}
