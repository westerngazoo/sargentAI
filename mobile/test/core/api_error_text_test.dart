// R-0061 AC11 -> SPEC-0061 §2.9: the API error fallbacks live in the
// catalogues. apiErrorText renders an ApiException's kind in the given
// catalogue; the screens keep their endpoint-specific overrides.

import 'package:fitai/l10n/app_localizations.dart';
import 'package:fitai/src/core/network/api_error_text.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/l10n.dart';

void main() {
  test('every kind renders its catalogue entry in Spanish', () {
    expect(apiErrorText(const ApiException(ApiErrorKind.transport), l10n),
        l10n.apiErrorTransport);
    expect(apiErrorText(const ApiException(ApiErrorKind.badRequest), l10n),
        l10n.apiErrorBadRequest);
    expect(apiErrorText(const ApiException(ApiErrorKind.unauthorized), l10n),
        l10n.apiErrorUnauthorized);
    expect(apiErrorText(const ApiException(ApiErrorKind.notFound), l10n),
        l10n.apiErrorNotFound);
    expect(apiErrorText(const ApiException(ApiErrorKind.conflict), l10n),
        l10n.apiErrorConflict);
    expect(apiErrorText(const ApiException(ApiErrorKind.unknown), l10n),
        l10n.apiErrorUnknown);
  });

  test('the English catalogue keeps the original wording', () {
    final en = lookupAppLocalizations(const Locale('en'));
    expect(apiErrorText(const ApiException(ApiErrorKind.transport), en),
        "can't reach the server — retry");
    expect(apiErrorText(const ApiException(ApiErrorKind.badRequest), en),
        'please check your details');
  });
}
