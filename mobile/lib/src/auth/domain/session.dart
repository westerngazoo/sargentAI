/// The authenticated session identity (SPEC-0007 §2.4). Carried by
/// `AuthAuthenticated`; the only thing the shell needs to know about "who".
class Session {
  const Session({required this.userId});

  final String userId;
}
