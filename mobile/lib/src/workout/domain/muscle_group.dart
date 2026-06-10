/// A muscle group. Wire tokens mirror `core::workout::MuscleGroup`
/// (`#[serde(rename_all = "snake_case")]`) exactly — the client must emit these
/// so `POST /workouts` accepts them (AC12: no backend change).
enum MuscleGroup {
  chest('chest'),
  back('back'),
  shoulders('shoulders'),
  arms('arms'),
  legs('legs'),
  core('core');

  const MuscleGroup(this.wire);

  /// The token the backend serializes.
  final String wire;

  /// Parse a backend token back to its [MuscleGroup].
  static MuscleGroup fromWire(String wire) =>
      values.firstWhere((m) => m.wire == wire);
}
