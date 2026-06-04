// SAC7 -> AC7: API base URL is configurable at build time, dev default
// 'http://localhost:8080', no hard-coded production URL.
//
// RED until package:fitai/src/core/config/app_config.dart defines AppConfig,
// and dio_provider.dart wires it into Dio.options.baseUrl.

import 'package:dio/dio.dart';
import 'package:fitai/src/core/config/app_config.dart';
import 'package:fitai/src/core/network/dio_provider.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('SAC7 AppConfig.apiBaseUrl', () {
    // No --dart-define is passed under `flutter test`, so the default applies.
    test('defaults to the local backend when API_BASE_URL is unset', () {
      expect(AppConfig.apiBaseUrl, 'http://localhost:8080');
    });

    test('compiles in no production URL by default', () {
      expect(AppConfig.apiBaseUrl, isNot(contains('https://')));
    });

    test('dioProvider builds a Dio whose baseUrl honours AppConfig', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final Dio dio = container.read(dioProvider);
      expect(dio.options.baseUrl, AppConfig.apiBaseUrl);
    });
  });
}
