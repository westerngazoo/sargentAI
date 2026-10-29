// SPEC-0059 §2.5 — the lift a physics check models, as the server names it.

/// A lift with a physics model. Mirrors `core::technique::Lift`; which
/// exercise has one is the server's call (`physics_for`), never a client
/// string list.
enum Lift {
  squat,
  bench,
  deadlift;

  /// The lift for a wire token, or null for `null` or a token this client
  /// does not know: a lift added on the server later reads as "no model yet"
  /// (*Pronto*) instead of breaking the whole schedule.
  static Lift? fromWire(String? token) => switch (token) {
        'squat' => Lift.squat,
        'bench' => Lift.bench,
        'deadlift' => Lift.deadlift,
        _ => null,
      };
}
