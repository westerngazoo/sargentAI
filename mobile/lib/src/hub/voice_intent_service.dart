// R-0032 slice 2 — backend voice intent client.

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/network/api_exception.dart';
import '../core/network/dio_provider.dart';

class VoiceIntentResult {
  const VoiceIntentResult({
    required this.status,
    this.message,
    this.prompt,
    this.route,
    this.recordId,
  });

  final String status;
  final String? message;
  final String? prompt;
  final String? route;
  final String? recordId;

  factory VoiceIntentResult.fromJson(Map<String, dynamic> json) =>
      VoiceIntentResult(
        status: json['status'] as String,
        message: json['message'] as String?,
        prompt: json['prompt'] as String?,
        route: json['route'] as String?,
        recordId: json['record_id'] as String?,
      );

  bool get isLoggedNutrition => status == 'logged_nutrition';
  bool get isLoggedWorkout => status == 'logged_workout';
  bool get isClarify => status == 'clarify';
  bool get isNavigate => status == 'navigate';
}

class VoiceIntentService {
  const VoiceIntentService(this._dio);

  final Dio _dio;

  Future<VoiceIntentResult> parse(String transcript) async {
    try {
      final res = await _dio.post<Map<String, dynamic>>(
        '/voice/intent',
        data: {'transcript': transcript},
      );
      return VoiceIntentResult.fromJson(res.data!);
    } on DioException catch (e) {
      throw ApiException.fromDio(e);
    }
  }
}

final voiceIntentServiceProvider = Provider<VoiceIntentService>(
  (ref) => VoiceIntentService(ref.watch(dioProvider)),
);
