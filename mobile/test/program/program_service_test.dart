// SPEC-0061 §2.4 (slice 2d): ProgramService.getSchedule maps
// `GET /programs/me/schedule?from&to&today` over the shared Dio. Dates go
// on the wire as `YYYY-MM-DD`; every failure is an ApiException.

import 'package:dio/dio.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/program/services/program_service.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';
import '../support/profile_fakes.dart';
import '../support/program_fakes.dart';

Response<T> _resp<T>(String path, int status, T data) {
  final req = RequestOptions(path: path);
  return Response<T>(requestOptions: req, statusCode: status, data: data);
}

void main() {
  setUpAll(registerFallbacks);

  late MockDio dio;
  late ProgramService service;

  setUp(() {
    dio = MockDio();
    service = ProgramService(dio);
  });

  group('getSchedule', () {
    test('sends the three dates as YYYY-MM-DD and parses the body', () async {
      when(() => dio.get<Map<String, dynamic>>(
            '/programs/me/schedule',
            queryParameters: any(named: 'queryParameters'),
          )).thenAnswer((_) async => _resp(
            '/programs/me/schedule',
            200,
            scheduleJson(days: [
              dayJson('2026-03-05', state: 'planned', cycleIndex: 1),
            ]),
          ));

      final s = await service.getSchedule(
        from: DateTime(2026, 3, 1, 9, 30),
        to: DateTime(2026, 4, 5),
        today: DateTime(2026, 3, 5),
      );

      final captured = verify(() => dio.get<Map<String, dynamic>>(
            '/programs/me/schedule',
            queryParameters: captureAny(named: 'queryParameters'),
          )).captured.single as Map<String, dynamic>;
      expect(captured, {
        'from': '2026-03-01',
        'to': '2026-04-05',
        'today': '2026-03-05',
      });
      expect(s.stateOn(DateTime(2026, 3, 5)), isNotNull);
    });

    test('a 422 (range_too_far_ahead) becomes an ApiException', () async {
      when(() => dio.get<Map<String, dynamic>>(
                '/programs/me/schedule',
                queryParameters: any(named: 'queryParameters'),
              ))
          .thenThrow(dioErrorFlat(422,
              path: '/programs/me/schedule', error: 'range_too_far_ahead'));

      await expectLater(
        service.getSchedule(
          from: DateTime(9999, 1, 1),
          to: DateTime(9999, 1, 31),
          today: DateTime(2026, 3, 5),
        ),
        throwsA(isA<ApiException>()),
      );
    });

    test('a transport failure becomes an ApiException, never a DioException',
        () async {
      when(() => dio.get<Map<String, dynamic>>(
            '/programs/me/schedule',
            queryParameters: any(named: 'queryParameters'),
          )).thenThrow(dioTransport(path: '/programs/me/schedule'));

      await expectLater(
        service.getSchedule(
          from: DateTime(2026, 3, 1),
          to: DateTime(2026, 3, 31),
          today: DateTime(2026, 3, 5),
        ),
        throwsA(isA<ApiException>()
            .having((e) => e.kind, 'kind', ApiErrorKind.transport)),
      );
    });
  });
}
