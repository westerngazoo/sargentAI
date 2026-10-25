// R-0061 AC11 -> SPEC-0061 §2.9: both catalogues are complete. gen-l10n
// reports untranslated messages only at build time, so this test checks the
// same fact from the source files: every key in the Spanish template has an
// English value, and English carries no key the template lacks.

import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

Set<String> _keys(String path) {
  final map = jsonDecode(File(path).readAsStringSync()) as Map<String, dynamic>;
  return map.keys.where((k) => !k.startsWith('@')).toSet();
}

void main() {
  test('app_en.arb has exactly the keys of app_es.arb', () {
    final es = _keys('lib/l10n/app_es.arb');
    final en = _keys('lib/l10n/app_en.arb');
    expect(en.difference(es), isEmpty, reason: 'keys only in English');
    expect(es.difference(en), isEmpty, reason: 'untranslated in English');
  });
}
