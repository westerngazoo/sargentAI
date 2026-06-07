/// Biological sex. Wire tokens mirror `core::profile::Sex`
/// (`#[serde(rename_all = "lowercase")]`): `male` / `female` — **lowercase**,
/// NOT snake_case (AC11: no backend change).
enum Sex {
  male('male'),
  female('female');

  const Sex(this.wire);

  /// The lowercase token the backend serializes.
  final String wire;

  /// Parse a backend token back to its [Sex].
  static Sex fromWire(String wire) => values.firstWhere((s) => s.wire == wire);
}
