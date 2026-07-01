// R-0032 (slice 1) — widget tests for the voice hub.
//
// Covers: ring renders all six options + mic; tapping an option navigates;
// a final speech transcript routes through the intent parser to the same
// destination; unavailable speech shows the fallback hint.

import 'package:fitai/src/hub/speech_input.dart';
import 'package:fitai/src/hub/voice_hub_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

/// Scripted speech engine: emits [transcript] as final on listen().
class FakeSpeechInput implements SpeechInput {
  FakeSpeechInput({this.available = true, this.transcript = ''});

  final bool available;
  final String transcript;

  @override
  Future<bool> initialize() async => available;

  @override
  Future<void> listen(OnTranscript onTranscript) async {
    onTranscript(transcript, true);
  }

  @override
  Future<void> stop() async {}
}

Widget _app(SpeechInput speech) {
  final router = GoRouter(
    initialLocation: '/hub',
    routes: [
      GoRoute(path: '/hub', builder: (_, __) => const VoiceHubScreen()),
      GoRoute(
        path: '/home',
        builder: (_, __) => const Scaffold(body: Text('home-sentinel')),
      ),
      GoRoute(
        path: '/programs/current',
        builder: (_, __) => const Scaffold(body: Text('program-sentinel')),
      ),
      GoRoute(
        path: '/programs/get',
        builder: (_, __) => const Scaffold(body: Text('picker-sentinel')),
      ),
      GoRoute(
        path: '/onboarding',
        builder: (_, __) => const Scaffold(body: Text('profile-sentinel')),
      ),
    ],
  );
  return ProviderScope(
    overrides: [speechInputProvider.overrideWithValue(speech)],
    child: MaterialApp.router(routerConfig: router),
  );
}

void main() {
  testWidgets('ring renders all six options and the mic', (tester) async {
    await tester.pumpWidget(_app(FakeSpeechInput()));
    await tester.pumpAndSettle();

    for (final label in [
      'Workout',
      'Meal',
      'Program',
      'Body match',
      'History',
      'Profile',
    ]) {
      expect(find.text(label), findsOneWidget);
    }
    expect(find.byIcon(Icons.mic), findsOneWidget);
  });

  testWidgets('tapping Program navigates to /programs/current', (tester) async {
    await tester.pumpWidget(_app(FakeSpeechInput()));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Program'));
    await tester.pumpAndSettle();

    expect(find.text('program-sentinel'), findsOneWidget);
  });

  testWidgets('tapping Body match navigates to /programs/get', (tester) async {
    await tester.pumpWidget(_app(FakeSpeechInput()));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Body match'));
    await tester.pumpAndSettle();

    expect(find.text('picker-sentinel'), findsOneWidget);
  });

  testWidgets('tapping Meal opens the quick-log sheet', (tester) async {
    await tester.pumpWidget(_app(FakeSpeechInput()));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Meal'));
    await tester.pumpAndSettle();

    expect(find.text('Log a meal'), findsOneWidget);
    expect(find.text('Save meal'), findsOneWidget);
  });

  testWidgets('dictating "show my program" navigates like the tap',
      (tester) async {
    await tester
        .pumpWidget(_app(FakeSpeechInput(transcript: 'show my program')));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.mic));
    await tester.pumpAndSettle();

    expect(find.text('program-sentinel'), findsOneWidget);
  });

  testWidgets('dictating a meal with macros prefills the sheet',
      (tester) async {
    await tester.pumpWidget(_app(FakeSpeechInput(
        transcript: 'log a meal 40 grams of protein 60 carbs 20 fat')));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.mic));
    await tester.pumpAndSettle();

    expect(find.text('Log a meal'), findsOneWidget);
    expect(find.widgetWithText(TextField, '40'), findsOneWidget);
    expect(find.widgetWithText(TextField, '60'), findsOneWidget);
    expect(find.widgetWithText(TextField, '20'), findsOneWidget);
  });

  testWidgets('unavailable speech shows the fallback hint', (tester) async {
    await tester.pumpWidget(_app(FakeSpeechInput(available: false)));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.mic));
    await tester.pumpAndSettle();

    expect(find.textContaining('not available'), findsOneWidget);
  });

  testWidgets('unknown dictation shows a hint and stays on the hub',
      (tester) async {
    await tester.pumpWidget(
        _app(FakeSpeechInput(transcript: 'purple monkey dishwasher')));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.mic));
    await tester.pumpAndSettle();

    expect(find.textContaining('purple monkey dishwasher'), findsOneWidget);
    expect(find.byIcon(Icons.mic), findsOneWidget);
  });
}
