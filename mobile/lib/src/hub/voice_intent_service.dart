// R-0032 slice 2 — backend voice intent client. A `navigate` answer is read
// as a typed [VoiceEffect] (SPEC-0061 §2.8); the server's wording stays on
// the model for the wire's sake and is never shown or spoken.

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/network/api_exception.dart';
import '../core/network/dio_provider.dart';
import 'voice_effect.dart';

/// One answer of `POST /voice/intent`.
class VoiceIntentResult {
  const VoiceIntentResult({
    required this.status,
    this.message,
    this.prompt,
    this.effect,
    this.recordId,
  });

  final String status;

  /// The server's wording, English prose. On the wire for older clients;
  /// this one speaks its own line (SPEC-0061 §2.8).
  final String? message;

  /// The server's clarifying question, English prose like [message].
  final String? prompt;

  /// What a `navigate` answer asks for; null on every other status.
  final VoiceEffect? effect;

  final String? recordId;

  factory VoiceIntentResult.fromJson(Map<String, dynamic> json) {
    final status = json['status'] as String;
    return VoiceIntentResult(
      status: status,
      message: json['message'] as String?,
      prompt: json['prompt'] as String?,
      effect: status == 'navigate'
          ? parseVoiceEffect(
              effect: _string(json['effect']),
              route: _string(json['route']),
            )
          : null,
      recordId: json['record_id'] as String?,
    );
  }

  bool get isLoggedNutrition => status == 'logged_nutrition';
  bool get isLoggedWorkout => status == 'logged_workout';
  bool get isClarify => status == 'clarify';
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

/// A field that should be a string; anything else reads as absent.
String? _string(Object? value) => value is String ? value : null;
