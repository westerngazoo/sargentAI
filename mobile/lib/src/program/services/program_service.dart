// R-0014 / SPEC-0014 §2.5 — typed HTTP client over the program endpoints.

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/dates.dart';
import '../../core/network/api_exception.dart';
import '../../core/network/dio_provider.dart';
import '../models/program_proposal.dart';
import '../models/schedule.dart';
import '../models/synthetic_match.dart';
import '../models/user_program.dart';

/// Typed client over the four R-0014 program endpoints. Every transport or
/// HTTP failure is routed through [ApiException.fromDio] — raw [DioException]s
/// never escape the service layer.
class ProgramService {
  const ProgramService(this._dio);

  final Dio _dio;

  /// `GET /photo-sessions/:id/program-proposals` → top-3 proposals.
  Future<ProposalsResponse> getProposals(String sessionId) async {
    try {
      final res = await _dio.get<Map<String, dynamic>>(
          '/photo-sessions/$sessionId/program-proposals');
      return ProposalsResponse.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }

  /// `POST /programs` — choose an archetype; returns the persisted program.
  /// [startsOn] is the client's local date the plan starts on (SPEC-0061
  /// §2.4); the server accepts it within a day of its own.
  Future<UserProgram> chooseProgram(
    String photoSessionId,
    String archetypeId, {
    required DateTime startsOn,
  }) async {
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/programs',
        data: {
          'photo_session_id': photoSessionId,
          'archetype_id': archetypeId,
          'starts_on': isoDate(startsOn),
        },
      );
      return UserProgram.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }

  /// `GET /programs/me/current` → the active program, or `null` if none.
  Future<UserProgram?> getCurrent() async {
    try {
      final res = await _dio.get<Map<String, dynamic>>('/programs/me/current');
      return UserProgram.fromJson(res.data!);
    } on DioException catch (e) {
      if (e.response?.statusCode == 404) return null;
      throw ApiException.fromDio(e);
    }
  }

  /// `POST /match/synthetic` — get top-3 proposals from body shape + fat band.
  Future<SyntheticMatchResponse> syntheticMatch(
      BodyShape shape, FatBand fatBand) async {
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/match/synthetic',
        data: {'shape': shape.value, 'fat_band': fatBand.value},
      );
      return SyntheticMatchResponse.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }

  /// `POST /programs/synthetic` — choose a proposal from a synthetic match.
  /// [startsOn] as in [chooseProgram].
  Future<UserProgram> chooseSyntheticProgram(
    String archetypeId,
    BodyShape shape,
    FatBand fatBand, {
    required DateTime startsOn,
  }) async {
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/programs/synthetic',
        data: {
          'archetype_id': archetypeId,
          'shape': shape.value,
          'fat_band': fatBand.value,
          'starts_on': isoDate(startsOn),
        },
      );
      return UserProgram.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }

  /// `GET /programs/me/schedule?from&to&today` → the projection over
  /// `[from, to]` (SPEC-0061 §2.4). Dates travel as `YYYY-MM-DD`; [today] is
  /// the client's local date. A range the server rejects (inverted, over 62
  /// days, more than a year ahead) is a 422 and surfaces as an
  /// [ApiException], like every other failure.
  Future<Schedule> getSchedule({
    required DateTime from,
    required DateTime to,
    required DateTime today,
  }) async {
    try {
      final res = await _dio.get<Map<String, dynamic>>(
        '/programs/me/schedule',
        queryParameters: {
          'from': isoDate(from),
          'to': isoDate(to),
          'today': isoDate(today),
        },
      );
      return Schedule.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }

  /// `GET /programs/me` → paginated history.
  Future<ProgramHistoryResponse> getHistory({
    int limit = 20,
    int offset = 0,
  }) async {
    try {
      final res = await _dio.get<Map<String, dynamic>>(
        '/programs/me',
        queryParameters: {'limit': limit, 'offset': offset},
      );
      return ProgramHistoryResponse.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }
}

final programServiceProvider = Provider<ProgramService>(
  (ref) => ProgramService(ref.read(dioProvider)),
);
