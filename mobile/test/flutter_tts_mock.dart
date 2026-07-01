import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void setupFlutterTtsMock() {
  const MethodChannel ttsChannel = MethodChannel('flutter_tts');
  const MethodChannel audioSessionChannel = MethodChannel('com.ryanheise.audio_session');
  const MethodChannel justAudioChannel = MethodChannel('com.ryanheise.just_audio.methods');
  const MethodChannel audioServiceChannel = MethodChannel('com.ryanheise.audio_service');

  TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
      .setMockMethodCallHandler(ttsChannel, (MethodCall methodCall) async {
    return 1;
  });

  TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
      .setMockMethodCallHandler(audioSessionChannel, (MethodCall methodCall) async {
    return null;
  });

  TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
      .setMockMethodCallHandler(justAudioChannel, (MethodCall methodCall) async {
    return {};
  });

  TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
      .setMockMethodCallHandler(audioServiceChannel, (MethodCall methodCall) async {
    return null;
  });
}
