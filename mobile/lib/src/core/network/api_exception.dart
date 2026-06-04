/// The single, user-safe error type the UI renders (SPEC-0007 §2.6, AC9).
///
/// A raw `DioException`/stack trace never reaches a screen — the data layer
/// translates transport and HTTP failures into this typed shape:
/// * [statusCode] is the HTTP status, or `null` for a transport/timeout error
///   (the AC9 retryable case);
/// * [field] names the offending request field when the backend reports one
///   (the `{"error":{"field":"…"}}` body on a 400);
/// * [message] is always safe to display verbatim.
class ApiException implements Exception {
  const ApiException(this.message, {this.statusCode, this.field});

  final String message;
  final int? statusCode;
  final String? field;

  @override
  String toString() => 'ApiException($statusCode, $field): $message';
}
