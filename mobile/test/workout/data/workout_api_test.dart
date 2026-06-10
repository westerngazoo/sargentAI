// SAC5/SAC6/SAC8/SAC9/SAC12 -> AC5/AC6/AC8/AC9/AC12 (data layer): WorkoutApi
// maps the R-0004 wire over the shared Dio —
//   list():   GET    /workouts      -> 200 List<WorkoutSession> (server order)
//   create(): POST   /workouts      -> 201 WorkoutSession, body = req.toJson()
//   delete(): DELETE /workouts/:id  -> 204 void; 404 -> ApiException(404)
// Every transport/HTTP failure goes through the single parsing authority
// ApiException.fromDio (flat backend body) — never a raw DioException.
// Only `/workouts[/:id]` paths are called (SAC12).
//
// RED until package:fitai/src/workout/data/workout_api.dart defines
// WorkoutApi(Dio) with list()/create()/delete().

import 'package:dio/dio.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/workout/data/workout_api.dart';
import 'package:fitai/src/workout/domain/exercise_draft.dart';
import 'package:fitai/src/workout/domain/muscle_group.dart';
import 'package:fitai/src/workout/domain/session_draft.dart';
import 'package:fitai/src/workout/domain/set_draft.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/fakes.dart';
import '../../support/profile_fakes.dart';
import '../../support/workout_fakes.dart';

Response<T> _resp<T>(String path, int status, T data) {
  final req = RequestOptions(path: path);
  return Response<T>(requestOptions: req, statusCode: status, data: data);
}

SessionRequest _validRequest() => SessionDraft(exercises: [
      ExerciseDraft(
        name: 'Bench press',
        muscleGroup: MuscleGroup.chest,
        sets: const [SetDraft(reps: 8, weightKg: 80, rpe: 8.5)],
      ),
    ]).toRequest(DateTime(2026, 6, 10))!;

void main() {
  setUpAll(registerFallbacks);
  setUpAll(registerWorkoutFallbacks);

  late MockDio dio;
  late WorkoutApi api;

  setUp(() {
    dio = MockDio();
    api = WorkoutApi(dio);
  });

  group('SAC8 list', () {
    test('200 parses the sessions preserving server order', () async {
      when(() => dio.get<List<dynamic>>('/workouts')).thenAnswer(
        (_) async => _resp('/workouts', 200, <dynamic>[
          sessionResponseJson(id: 's-new', performedOn: '2026-06-10'),
          sessionResponseJson(id: 's-old', performedOn: '2026-06-01'),
        ]),
      );
      final sessions = await api.list();
      expect(sessions.map((s) => s.id), ['s-new', 's-old']);
    });

    test('a 401 becomes ApiException(401) (interceptor owns the sink)',
        () async {
      when(() => dio.get<List<dynamic>>('/workouts')).thenThrow(
        dioErrorFlat(401, path: '/workouts', error: 'unauthorized'),
      );
      await expectLater(api.list(), throwsA(isApiExceptionWithStatus(401)));
    });

    test('a transport error becomes a retryable ApiException (null status)',
        () async {
      when(() => dio.get<List<dynamic>>('/workouts'))
          .thenThrow(dioTransport(path: '/workouts'));
      await expectLater(api.list(), throwsA(isApiExceptionWithStatus(null)));
    });
  });

  group('SAC5/SAC6 create', () {
    test('201 parses the created session and sends exactly req.toJson()',
        () async {
      when(() => dio.post<Map<String, dynamic>>('/workouts',
          data: any(named: 'data'))).thenAnswer(
        (_) async => _resp('/workouts', 201, sessionResponseJson(id: 's-new')),
      );
      final session = await api.create(_validRequest());
      expect(session.id, 's-new');

      final sent = verify(() => dio.post<Map<String, dynamic>>('/workouts',
          data: captureAny(named: 'data'))).captured.single;
      expect(sent, _validRequest().toJson(),
          reason: 'the wire body is the request JSON, nothing more');
    });

    test('a 400 carries statusCode 400 AND the offending field (AC6)',
        () async {
      when(() => dio.post<Map<String, dynamic>>('/workouts',
              data: any(named: 'data')))
          .thenThrow(dioErrorFlat(400, path: '/workouts', field: 'rpe'));
      await expectLater(
        api.create(_validRequest()),
        throwsA(isA<ApiException>()
            .having((e) => e.statusCode, 'statusCode', 400)
            .having((e) => e.field, 'field', 'rpe')),
      );
    });

    test('a transport error -> retryable ApiException (null statusCode)',
        () async {
      when(() => dio.post<Map<String, dynamic>>('/workouts',
          data: any(named: 'data'))).thenThrow(dioTransport(path: '/workouts'));
      await expectLater(
        api.create(_validRequest()),
        throwsA(isApiExceptionWithStatus(null)),
      );
    });

    test('a 401 -> ApiException(401) (no widget ever sees a DioException)',
        () async {
      when(() => dio.post<Map<String, dynamic>>('/workouts',
              data: any(named: 'data')))
          .thenThrow(
              dioErrorFlat(401, path: '/workouts', error: 'unauthorized'));
      await expectLater(
        api.create(_validRequest()),
        throwsA(isApiExceptionWithStatus(401)),
      );
    });
  });

  group('SAC9 delete', () {
    test('204 completes as void', () async {
      when(() => dio.delete<void>('/workouts/s-1')).thenAnswer(
        (_) async => Response<void>(
          requestOptions: RequestOptions(path: '/workouts/s-1'),
          statusCode: 204,
        ),
      );
      await expectLater(api.delete('s-1'), completes);
      verify(() => dio.delete<void>('/workouts/s-1')).called(1);
    });

    test('a foreign/missing id surfaces the backend 404 as ApiException(404)',
        () async {
      when(() => dio.delete<void>('/workouts/s-gone')).thenThrow(
        dioErrorFlat(404, path: '/workouts/s-gone', error: 'not_found'),
      );
      await expectLater(
        api.delete('s-gone'),
        throwsA(isApiExceptionWithStatus(404)),
      );
    });

    test('a transport error -> retryable ApiException (null statusCode)',
        () async {
      when(() => dio.delete<void>('/workouts/s-1'))
          .thenThrow(dioTransport(path: '/workouts/s-1'));
      await expectLater(
        api.delete('s-1'),
        throwsA(isApiExceptionWithStatus(null)),
      );
    });
  });
}
