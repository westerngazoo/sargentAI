import '../../../l10n/app_localizations.dart';
import 'api_exception.dart';

/// The catalogue fallback for an [ApiException] (R-0061 AC11). Generic by
/// design: a screen that knows more about its endpoint (register's 409, a
/// deleted workout's 404) checks `e.kind` first and falls through to this.
String apiErrorText(ApiException e, AppLocalizations l10n) => switch (e.kind) {
      ApiErrorKind.transport => l10n.apiErrorTransport,
      ApiErrorKind.badRequest => l10n.apiErrorBadRequest,
      ApiErrorKind.unauthorized => l10n.apiErrorUnauthorized,
      ApiErrorKind.notFound => l10n.apiErrorNotFound,
      ApiErrorKind.conflict => l10n.apiErrorConflict,
      ApiErrorKind.unknown => l10n.apiErrorUnknown,
    };
