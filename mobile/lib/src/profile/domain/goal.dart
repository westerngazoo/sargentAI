/// A training goal. Wire tokens mirror `core::profile::Goal`
/// (`#[serde(rename_all = "snake_case")]`) exactly — the client must emit these
/// so the R-0003 `PUT /profile/me` accepts them (AC11: no backend change).
enum Goal {
  loseFat('lose_fat'),
  buildMuscle('build_muscle'),
  recomp('recomp'),
  maintain('maintain'),
  gainStrength('gain_strength');

  const Goal(this.wire);

  /// The snake_case token the backend serializes.
  final String wire;

  /// Parse a backend token back to its [Goal].
  static Goal fromWire(String wire) => values.firstWhere((g) => g.wire == wire);
}
