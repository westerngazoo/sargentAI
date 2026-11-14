// R-0061 e2e: the one screen built around a calendar, driven through the
// real FitAiApp — its router, theme, locale rule, AuthInterceptor and every
// repository — against an in-memory backend at the Dio transport edge (no
// device, no real server), as login_to_home_test.dart does for R-0007.
//
//   AC4  the calendar marks today and the planned day the server projects;
//   AC5  the selected day shows its plan and one action, Empezar sesión;
//   AC1  a panel opens over the screen and closes back to it, day kept;
//   AC10 Empezar sesión pushes focus mode seeded with the plan day; finishing
//        returns to the day, now shown as done;
//   AC6  the client asks the server for the projection with its own today,
//        and the finished session carries the plan link that advances it;
//   AC11 the app follows a Spanish device;
//   AC12 focus mode is dark although the device theme is light.
//
// Run: `flutter test integration_test/one_screen_calendar_test.dart`.

import 'dart:convert';
import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:fitai/app.dart';
import 'package:fitai/src/core/network/dio_provider.dart';
import 'package:fitai/src/core/storage/token_store.dart';
import 'package:fitai/src/home/calendar.dart';
import 'package:fitai/src/home/main_screen.dart';
import 'package:fitai/src/hub/speech_input.dart';
import 'package:fitai/src/hub/voice_output.dart';
import 'package:fitai/src/program/presentation/program_panel.dart';
import 'package:fitai/src/workout/presentation/live_session_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:mocktail/mocktail.dart';

import '../test/support/l10n.dart';
import '../test/support/profile_fakes.dart';
import '../test/support/program_fakes.dart';
import '../test/support/voice_fakes.dart';
import '../test/support/workout_fakes.dart';

class _MockSecureStorage extends Mock implements FlutterSecureStorage {}

/// The backend, in memory, at Dio's transport edge. It answers the reads the
/// one screen makes and the workout save, records every request, and 404s
/// anything else, as an absent resource would.
class _FakeBackend implements HttpClientAdapter {
  _FakeBackend(this.today);

  final DateTime today;
  final requests = <RequestOptions>[];

  /// Set by a saved plan session: the server then reports today as done.
  Map<String, dynamic>? saved;

  DateTime get tomorrow => DateTime(today.year, today.month, today.day + 1);

  /// A two-day cycle: legs (one exercise, so one set finishes it), then rest.
  Map<String, dynamic> _schedule() => scheduleJson(
        startsOn: isoDate(today),
        cycle: [
          trainingDayJson(exercises: ['Back Squat']),
          restDayJson,
        ],
        days: [
          saved == null
              ? dayJson(isoDate(today), state: 'planned', cycleIndex: 0)
              : dayJson(isoDate(today), state: 'done', completed: [0]),
          dayJson(isoDate(tomorrow), state: 'rest', cycleIndex: 1),
        ],
        sessionIds: saved == null
            ? const {}
            : {
                isoDate(today): ['sess-e2e']
              },
      );

  Object? _answer(RequestOptions o) => switch ((o.method, o.path)) {
        ('GET', '/auth/me') => {'user_id': 'e2e-user', 'email': 'e2e@b.com'},
        ('GET', '/profile/me') => profileResponseJson(userId: 'e2e-user'),
        ('GET', '/programs/me/current') => userProgramJson(),
        ('GET', '/programs/me/schedule') => _schedule(),
        ('GET', '/workouts') => [if (saved != null) saved],
        ('POST', '/workouts') => saved = sessionResponseJson(
            id: 'sess-e2e',
            userId: 'e2e-user',
            performedOn: isoDate(today),
            exercises: [exerciseResponseJson(name: 'Back Squat')],
          ),
        _ => null,
      };

  @override
  Future<ResponseBody> fetch(RequestOptions options,
      Stream<Uint8List>? requestStream, Future<void>? cancelFuture) async {
    requests.add(options);
    final body = _answer(options);
    final status = body == null ? 404 : (options.method == 'POST' ? 201 : 200);
    return ResponseBody.fromString(
      jsonEncode(body ??
          {
            'error': {'code': 'not_found', 'message': 'not found'}
          }),
      status,
      headers: {
        Headers.contentTypeHeader: [Headers.jsonContentType],
      },
    );
  }

  @override
  void close({bool force = false}) {}

  Iterable<RequestOptions> sent(String method, String path) =>
      requests.where((r) => r.method == method && r.path == path);
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  final now = DateTime.now();
  final today = DateTime(now.year, now.month, now.day);

  /// Cold start of the real app on a Spanish phone with a stored token.
  Future<_FakeBackend> boot(WidgetTester tester) async {
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    tester.platformDispatcher.localesTestValue = const [Locale('es', 'MX')];
    tester.platformDispatcher.platformBrightnessTestValue = Brightness.light;
    addTearDown(tester.view.reset);
    addTearDown(tester.platformDispatcher.clearAllTestValues);

    final backend = _FakeBackend(today);
    final storage = _MockSecureStorage();
    final store = <String, String>{
      'fitai.jwt': jsonEncode({
        'jwt': 'jwt.e2e',
        'userId': 'e2e-user',
        'expiresAt': '2099-01-01T00:00:00.000Z',
      }),
    };
    when(() => storage.read(key: any(named: 'key')))
        .thenAnswer((inv) async => store[inv.namedArguments[#key]]);
    when(() => storage.delete(key: any(named: 'key'))).thenAnswer((inv) async {
      store.remove(inv.namedArguments[#key]);
    });

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          // The production client — base options and AuthInterceptor —
          // over the in-memory backend.
          dioProvider.overrideWith((ref) {
            final dio = Dio(BaseOptions(baseUrl: 'http://e2e.invalid'))
              ..httpClientAdapter = backend;
            dio.interceptors.add(AuthInterceptor(ref));
            return dio;
          }),
          tokenStoreProvider.overrideWithValue(TokenStore(storage)),
          speechInputProvider.overrideWithValue(ScriptedSpeechInput(const [])),
          voiceOutputProvider.overrideWithValue(RecordingVoiceOutput()),
        ],
        child: const FitAiApp(),
      ),
    );
    await tester.pumpAndSettle();
    return backend;
  }

  Finder cell(DateTime day) =>
      find.byKey(ValueKey('day-${day.toIso8601String()}'));

  testWidgets('R-0061: the day, a panel, focus mode and back, end to end',
      (tester) async {
    final backend = await boot(tester);

    // AC11: a Spanish device gets the Spanish catalogue.
    expect(find.byType(MainScreen), findsOneWidget);
    expect(find.text(l10n.startSession), findsOneWidget);

    // AC6: the projection comes from the server, asked with the client's
    // today and the stored token.
    final read = backend.sent('GET', '/programs/me/schedule').first;
    expect(read.queryParameters['today'], isoDate(today));
    expect(read.headers['Authorization'], 'Bearer jwt.e2e');

    // AC4: today is marked and selected, and planned.
    final todayCell = tester.widget<DayCell>(cell(today));
    expect(todayCell.isToday, isTrue);
    expect(todayCell.isSelected, isTrue);
    expect(find.byKey(const Key('mark-planned')), findsOneWidget);

    // AC5: the planned day is named by its focus and lists its plan.
    expect(find.text(l10n.muscleLegs), findsOneWidget);
    expect(find.text('Back Squat'), findsOneWidget);

    // AC1: a panel opens over the screen and closes back to it, day kept.
    await tester.tap(find.byKey(const Key('profile-menu')));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.menuProgram));
    await tester.pumpAndSettle();
    expect(find.byType(ProgramPanel), findsOneWidget);
    await tester.tap(find.byKey(const Key('panel-close')));
    await tester.pumpAndSettle();
    expect(find.byType(ProgramPanel), findsNothing);
    expect(tester.widget<DayCell>(cell(today)).isSelected, isTrue);

    // AC10: Empezar sesión pushes focus mode, seeded with the plan day.
    await tester.tap(find.byKey(const Key('primary-action')));
    await tester.pumpAndSettle();
    expect(find.byType(LiveSessionScreen), findsOneWidget);
    expect(find.textContaining('Back Squat'), findsWidgets);

    await tester.enterText(find.widgetWithText(TextField, l10n.repsLabel), '5');
    await tester.tap(find.text(l10n.logSet));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.finish).first);
    await tester.pumpAndSettle();

    // AC6: the saved session is linked to the plan day it completes.
    final post = backend.sent('POST', '/workouts').single;
    final body = post.data as Map<String, dynamic>;
    expect(body['user_program_id'], 'prog-uuid-001');
    expect(body['plan_day_index'], 0);
    expect(body['performed_on'], isoDate(today));

    // AC10: back on the one screen, today shown as done.
    expect(find.byType(LiveSessionScreen), findsNothing);
    expect(find.byType(MainScreen), findsOneWidget);
    expect(find.text(l10n.viewLogged), findsOneWidget);
    expect(find.byKey(const Key('mark-done')), findsOneWidget);
    expect(find.byKey(const Key('mark-planned')), findsNothing);
  });

  testWidgets('R-0061 AC12: focus mode is dark under a light device theme',
      (tester) async {
    await boot(tester);
    expect(Theme.of(tester.element(find.byType(MainScreen))).brightness,
        Brightness.light);

    await tester.tap(find.byKey(const Key('primary-action')));
    await tester.pumpAndSettle();

    final scaffold = find.descendant(
        of: find.byType(LiveSessionScreen), matching: find.byType(Scaffold));
    expect(
        Theme.of(tester.element(scaffold.first)).brightness, Brightness.dark);
  });
}
