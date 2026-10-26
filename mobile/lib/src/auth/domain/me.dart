/// What `GET /auth/me` says about the caller (SPEC-0061 §2.6, slice 3c):
/// the account's id and its normalized email. The id is what the session
/// already knows; the email is what the Cuenta panel shows.
class Me {
  const Me({required this.userId, required this.email});

  final String userId;
  final String email;
}
