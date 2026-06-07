import 'package:dio/dio.dart';

/// The single, user-safe error type the UI renders (SPEC-0007 §2.6, AC9).
///
/// A raw `DioException`/stack trace never reaches a screen — the data layer
/// translates transport and HTTP failures into this typed shape via
/// [ApiException.fromDio]:
/// * [statusCode] is the HTTP status, or `null` for a transport/timeout error
///   (the AC9 retryable case);
/// * [field] names the offending request field when the backend reports one —
///   read from the backend's **flat** body `{"error":"<kind>","field":"<name>"}`
///   (`backend/crates/api/src/error.rs`, SPEC-0008 §2.7);
/// * [message] is always safe to display verbatim.
class ApiException implements Exception {
  const ApiException(this.message, {this.statusCode, this.field});

  /// Translate a [DioException] into the user-safe shape. The single
  /// error-parsing authority shared by every data-layer client (auth, profile,
  /// and future loggers). Reads `field` from the flat top-level `data["field"]`
  /// — never from a nested `{"error":{...}}` shape (the backend never sends one;
  /// the old R-0007 parser assumed it and left `field` perpetually null).
  factory ApiException.fromDio(DioException e) {
    final response = e.response;
    if (response == null) {
      // No HTTP response → transport/timeout: the AC9 retryable case.
      return const ApiException("can't reach the server — retry");
    }
    final data = response.data;
    final field = data is Map ? data['field'] as String? : null;
    return ApiException(
      _defaultMessage(response.statusCode),
      statusCode: response.statusCode,
      field: field,
    );
  }

  final String message;
  final int? statusCode;
  final String? field;

  static String _defaultMessage(int? status) => switch (status) {
        400 => 'please check your details',
        401 => 'invalid email or password',
        409 => 'that email is already registered',
        _ => 'something went wrong — please try again',
      };

  @override
  String toString() => 'ApiException($statusCode, $field): $message';
}
