// R-0014 / SPEC-0014 §2.5 — typed HTTP client over the program endpoints.

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/network/api_exception.dart';
import '../../core/network/dio_provider.dart';
import '../models/program_proposal.dart';
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
      final res = await _dio
          .get<List<dynamic>>('/photo-sessions/$sessionId/program-proposals');
      final list = (res.data ?? const <dynamic>[])
          .map((e) => ProgramProposal.fromJson(e as Map<String, dynamic>))
          .toList();
      return ProposalsResponse(proposals: list);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }

  /// `POST /programs` — choose an archetype; returns the persisted program.
  Future<UserProgram> chooseProgram(
      String photoSessionId, String archetypeId) async {
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/programs',
        data: {
          'photo_session_id': photoSessionId,
          'archetype_id': archetypeId,
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
