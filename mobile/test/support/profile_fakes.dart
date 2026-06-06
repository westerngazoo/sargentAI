// Shared test doubles + helpers for the R-0008 onboarding suite.
//
// Authored by the qa agent in step 3 (test planning) — BEFORE any
// `lib/src/profile` production code exists. These imports resolve to the exact
// class/provider surface SPEC-0008 §2/§3/§6 names, so the suite is RED until
// step-5 implementation creates them, then turns GREEN with no test edits.
//
// Targeted production symbols (all under package:fitai/src/profile/...):
//   domain/goal.dart            -> Goal (lose_fat/build_muscle/recomp/maintain/gain_strength)
//   domain/sex.dart             -> Sex  (male/female — LOWERCASE wire tokens)
//   domain/profile.dart         -> Profile.fromJson
//   domain/profile_draft.dart   -> ProfileDraft, ProfileRequest
//   data/profile_api.dart       -> ProfileApi(Dio): getMe()/putMe()
//   data/profile_repository.dart-> ProfileRepository, profileRepositoryProvider
//   application/profile_providers.dart    -> profileProvider, onboardingDismissedProvider
//   application/onboarding_controller.dart-> OnboardingController, OnboardingState
//
// Plus the corrected shared parser in core/network/api_exception.dart:
//   ApiException.fromDio(DioException) reading the FLAT backend body
//   {"error":"<kind>","field":"<name>"}.

import 'package:dio/dio.dart';
import 'package:fitai/src/profile/data/profile_api.dart';
import 'package:fitai/src/profile/data/profile_repository.dart';
import 'package:fitai/src/profile/domain/goal.dart';
import 'package:fitai/src/profile/domain/profile.dart';
import 'package:fitai/src/profile/domain/profile_draft.dart';
import 'package:mocktail/mocktail.dart';

/// Mock typed profile client (SPEC-0008 §3.2): getMe()/putMe().
class MockProfileApi extends Mock implements ProfileApi {}

/// Mock repository the home prompt + onboarding controller depend on.
class MockProfileRepository extends Mock implements ProfileRepository {}

/// Registers the non-primitive fallback `mocktail` needs to match `putMe(any())`
/// — a real [ProfileRequest] built from a valid draft.
void registerProfileFallbacks() {
  registerFallbackValue(
    ProfileDraft(
      dateOfBirth: DateTime(1996, 6, 6),
      heightCm: 180,
      weightKg: 82.5,
      goals: const {Goal.buildMuscle},
    ).toRequest(DateTime(2026, 6, 6))!,
  );
}

/// A `ProfileResponse`-shaped JSON body (the GET/PUT /profile/me 200 body).
/// Keys mirror `backend/crates/api/src/profile/handlers.rs::ProfileResponse`
/// exactly; `sex`/`body_fat_percentage` are nullable optionals.
Map<String, dynamic> profileResponseJson({
  String userId = 'user-123',
  String dateOfBirth = '1990-05-20',
  int age = 35,
  int heightCm = 180,
  double weightKg = 82.5,
  String? sex,
  double? bodyFatPercentage,
  List<String> goals = const ['build_muscle'],
  String createdAt = '2026-06-06T00:00:00Z',
  String updatedAt = '2026-06-06T00:00:00Z',
}) =>
    <String, dynamic>{
      'user_id': userId,
      'date_of_birth': dateOfBirth,
      'age': age,
      'height_cm': heightCm,
      'weight_kg': weightKg,
      'sex': sex,
      'body_fat_percentage': bodyFatPercentage,
      'goals': goals,
      'created_at': createdAt,
      'updated_at': updatedAt,
    };

/// Parsed `Profile` convenience for provider-override / mock returns.
Profile sampleProfile({
  String userId = 'user-123',
  List<String> goals = const ['build_muscle'],
}) =>
    Profile.fromJson(profileResponseJson(userId: userId, goals: goals));

/// A DioException carrying the backend FLAT error body
/// `{"error":"<kind>","field":"<name>"}` (SPEC-0008 §2.7,
/// `backend/crates/api/src/error.rs`). `field` is present only for the
/// `validation` (400) kind.
DioException dioErrorFlat(
  int status, {
  String path = '/profile/me',
  String error = 'validation',
  String? field,
}) {
  final req = RequestOptions(path: path);
  final body = <String, dynamic>{'error': error};
  if (field != null) body['field'] = field;
  return DioException(
    requestOptions: req,
    type: DioExceptionType.badResponse,
    response: Response<dynamic>(
      requestOptions: req,
      statusCode: status,
      data: body,
    ),
  );
}

/// A transport/timeout DioException (no response) — the AC8 retryable path.
DioException dioTransport({String path = '/profile/me'}) {
  final req = RequestOptions(path: path);
  return DioException(
    requestOptions: req,
    type: DioExceptionType.connectionTimeout,
  );
}
