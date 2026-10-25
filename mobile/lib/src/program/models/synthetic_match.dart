// R-0030 — synthetic body-type match wire types.

import 'program_proposal.dart';

/// The three coarse body shapes shown in the picker grid. Their names and
/// descriptions are catalogue strings (`presentation/body_type_names.dart`).
enum BodyShape {
  ectomorph,
  mesomorph,
  endomorph;

  String get value => name;
}

/// The three coarse body-fat bands. Names and descriptions as for [BodyShape].
enum FatBand {
  lean,
  moderate,
  bulky;

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
