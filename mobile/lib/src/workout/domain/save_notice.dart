/// Why a finished plan session was saved as a free workout instead
/// (SPEC-0061 §2.4): the server refused the plan link, the driver dropped it
/// and saved again, and the screen tells the user from the catalogue. A
/// finished session is never lost.
enum SaveNotice {
  /// `409 program_not_current`: the program changed on another device
  /// mid-session.
  programNotCurrent,

  /// `422 plan_unavailable`: the current program's archetype has no cycle in
  /// the library.
  planUnavailable,
}
