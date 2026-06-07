import '../domain/profile.dart';
import '../domain/profile_draft.dart';
import 'profile_api.dart';

/// The seam the providers + onboarding controller depend on (mirrors the auth
/// repository). Keeps the controller one hop from transport.
class ProfileRepository {
  const ProfileRepository(this._api);

  final ProfileApi _api;

  Future<Profile?> getMe() => _api.getMe();

  Future<Profile> putMe(ProfileRequest req) => _api.putMe(req);
}
