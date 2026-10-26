import 'package:dio/dio.dart';

/// What an API call failed on, as the UI needs to tell them apart. The
/// backend's body carries tokens (`{"error":"validation","field":"..."}`,
/// `backend/crates/api/src/error.rs`), never prose, so the kind is derived
/// from the HTTP status and the text comes from the catalogue
/// (`apiErrorText`, R-0061 AC11).
enum ApiErrorKind {
  /// No HTTP response at all — timeout or no network. The AC9 retryable case.
  transport,

  /// 400: the request was malformed or a field failed validation.
  badRequest,

  /// 401: the token is missing, expired or revoked.
  unauthorized,

  /// 404: the resource does not exist or is not the caller's.
  notFound,

  /// 409: the request collides with current state.
  conflict,

  /// Any other HTTP status.
  unknown,
}

/// The single, user-safe error type the UI renders (SPEC-0007 §2.6, AC9).
///
/// A raw `DioException`/stack trace never reaches a screen — the data layer
/// translates transport and HTTP failures into this typed shape via
/// [ApiException.fromDio]:
/// * [kind] is what went wrong, localized by `apiErrorText`;
/// * [statusCode] is the HTTP status, or `null` for a transport/timeout error
///   (diagnostics only: `kind` drives every decision);
/// * [field] names the offending request field when the backend reports one —
///   read from the backend's **flat** body `{"error":"<kind>","field":"<name>"}`
///   (`backend/crates/api/src/error.rs`, SPEC-0008 §2.7);
/// * [reason] is the fixed token a 409 or 422 carries beside the kind
///   (`{"error":"conflict","reason":"program_not_current"}`), for the few
///   callers that act on one (SPEC-0061 §2.4).
class ApiException implements Exception {
  const ApiException(this.kind, {this.statusCode, this.field, this.reason});

  /// Translate a [DioException] into the user-safe shape. The single
  /// error-parsing authority shared by every data-layer client (auth, profile,
  /// and future loggers). Reads `field` from the flat top-level `data["field"]`
  /// — never from a nested `{"error":{...}}` shape (the backend never sends one;
  /// the old R-0007 parser assumed it and left `field` perpetually null).
  factory ApiException.fromDio(DioException e) {
    final response = e.response;
    if (response == null) return const ApiException(ApiErrorKind.transport);
    final data = response.data;
    final field = data is Map ? data['field'] as String? : null;
    final reason = data is Map ? data['reason'] as String? : null;
    return ApiException(
      _kindOf(response.statusCode),
      statusCode: response.statusCode,
      field: field,
      reason: reason,
    );
  }

  final ApiErrorKind kind;
  final int? statusCode;
  final String? field;
  final String? reason;

  /// The same codes are raised by every endpoint, so the kinds are generic;
  /// anything endpoint-specific is the screen's call (register's 409 reads
  /// "that email is already registered", a second meal's 409 must not).
  static ApiErrorKind _kindOf(int? status) => switch (status) {
        400 => ApiErrorKind.badRequest,
        401 => ApiErrorKind.unauthorized,
        404 => ApiErrorKind.notFound,
        409 => ApiErrorKind.conflict,
        _ => ApiErrorKind.unknown,
      };

  @override
  String toString() => 'ApiException($kind, $statusCode, $field, $reason)';
}
