// SPEC-0059 §2.5 — the lift a physics check models, as the server names it.

/// A lift with a physics model this client can draw. Which exercise has one
/// is the server's call (`biomech::model_for`), never a client string list.
/// Only the squat today; a value is added here when its check is built.
enum Lift {
  squat;

  /// The lift for a wire token, or null for `null` or any token this client
  /// cannot draw — `bench` and `deadlift` included, until they have a model
  /// here: the exercise reads as "no model yet" (*Pronto*) instead of
  /// breaking the whole schedule.
  static Lift? fromWire(String? token) => switch (token) {
        'squat' => Lift.squat,
        _ => null,
      };
}
