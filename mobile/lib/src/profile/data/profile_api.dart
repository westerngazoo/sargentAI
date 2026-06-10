import 'package:dio/dio.dart';

import '../../core/network/api_exception.dart';
import '../domain/profile.dart';
import '../domain/profile_draft.dart';

/// Typed client over `GET`/`PUT /profile/me` (R-0003). A `404` from [getMe]
/// means "no profile yet" (→ `null`), not an error; every other transport/HTTP
/// failure becomes an [ApiException] via the shared flat-body parser.
class ProfileApi {
  const ProfileApi(this._dio);

  final Dio _dio;

  Future<Profile?> getMe() async {
    try {
      final res = await _dio.get<Map<String, dynamic>>('/profile/me');
      return Profile.fromJson(res.data!);
    } on DioException catch (e) {
      if (e.response?.statusCode == 404) return null;
      throw ApiException.fromDio(e);
    }
  }

  Future<Profile> putMe(ProfileRequest req) async {
    try {
      final res = await _dio.put<Map<String, dynamic>>(
        '/profile/me',
        data: req.toJson(),
      );
      return Profile.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e); // carries statusCode + field for AC8
    }
  }
}
