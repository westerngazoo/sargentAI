// R-0059 AC1 -> SPEC-0059 §2.3 (slice 59-b): SquatCheckApi posts the request
// to `POST /biomech/squat/range`, parses the 200 into the domain model, and
// turns a 400 into an ApiException naming the field.

import 'package:dio/dio.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/physics/data/squat_check_api.dart';
import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/fakes.dart';
import '../../support/physics_fakes.dart';
import '../../support/profile_fakes.dart';

const _path = '/biomech/squat/range';

void main() {
  late MockDio dio;
  late SquatCheckApi api;

  setUp(() {
    dio = MockDio();
    api = SquatCheckApi(dio);
  });

  test('posts the request and parses the 200 body', () async {
    when(() => dio.post<Map<String, dynamic>>(_path, data: any(named: 'data')))
        .thenAnswer((_) async => Response(
              requestOptions: RequestOptions(path: _path),
              statusCode: 200,
              data: squatGoldenJson(),
            ));

    final r = await api.range(sampleSquatRequest);

    expect(r.end, isA<Bottom>());
    expect(r.frames, hasLength(2));
    verify(() => dio.post<Map<String, dynamic>>(_path,
        data: sampleSquatRequest.toJson())).called(1);
  });

  test('a 400 validation becomes an ApiException naming the field', () async {
    when(() => dio.post<Map<String, dynamic>>(_path, data: any(named: 'data')))
        .thenThrow(dioErrorFlat(400, path: _path, field: 'dorsiflexion_deg'));

    await expectLater(
      api.range(sampleSquatRequest),
      throwsA(isA<ApiException>()
          .having((e) => e.kind, 'kind', ApiErrorKind.badRequest)
          .having((e) => e.field, 'field', 'dorsiflexion_deg')),
    );
  });
}
