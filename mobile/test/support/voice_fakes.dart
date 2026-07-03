// Shared voice fakes: a scriptable speech engine, a recording TTS, and a
// nutrition-service mock for voice-logged meals.

import 'package:fitai/src/hub/speech_input.dart';
import 'package:fitai/src/hub/voice_output.dart';
import 'package:fitai/src/nutrition/models/food_info.dart';
import 'package:fitai/src/nutrition/models/nutrition_log.dart';
import 'package:fitai/src/nutrition/services/nutrition_service.dart';
import 'package:mocktail/mocktail.dart';

class MockNutritionService extends Mock implements NutritionService {}

NutritionLog sampleNutritionLog({
  double proteinG = 40,
  double carbsG = 60,
  double fatG = 20,
}) =>
    NutritionLog(
      id: 'nut-uuid-001',
      performedOn: '2026-07-02',
      proteinG: proteinG,
      carbsG: carbsG,
      fatG: fatG,
      calories: 4 * proteinG + 4 * carbsG + 9 * fatG,
    );

/// Emits queued transcripts, one final result per `listen()` call.
class ScriptedSpeechInput implements SpeechInput {
  ScriptedSpeechInput(this.transcripts, {this.available = true});

  final List<String> transcripts;
  final bool available;
  int _next = 0;

  @override
  Future<bool> initialize() async => available;

  @override
  Future<void> listen(OnTranscript onTranscript) async {
    final t = _next < transcripts.length ? transcripts[_next++] : '';
    await onTranscript(t, true);
  }

  @override
  Future<void> stop() async {}
}

/// Records everything spoken so tests can assert on announcements.
class RecordingVoiceOutput implements VoiceOutput {
  final List<String> spoken = [];

  @override
  Future<bool> initialize() async => true;

  @override
  Future<void> speak(String text) async => spoken.add(text);

  @override
  Future<void> stop() async {}
}

FoodInfo sampleFoodInfo({
  String name = 'Chicken, breast, grilled',
  double proteinG = 31,
  double carbsG = 0,
  double fatG = 3.5,
  double kcal = 165,
}) =>
    FoodInfo(
      name: name,
      proteinGPer100g: proteinG,
      carbsGPer100g: carbsG,
      fatGPer100g: fatG,
      kcalPer100g: kcal,
    );
