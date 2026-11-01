// R-0059 AC6 -> SPEC-0059 §2.4 (slice 59-c2a): no catalogue entry the squat
// check uses calls a torque safe or dangerous, in either language. Read from
// the ARB files, so a new `squat*` entry is covered without editing this.

import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

final _forbidden = RegExp(
  r'\b(segur[oa]s?|insegur[oa]s?|peligr\w*|lesi[oó]n\w*|lesion\w*|riesgo\w*|'
  r'daño\w*|dañin\w*|safe\w*|unsafe|danger\w*|injur\w*|risk\w*|harm\w*)\b',
  caseSensitive: false,
  unicode: true,
);

void main() {
  for (final lang in ['es', 'en']) {
    test('$lang: no squat entry makes a safety claim', () {
      final arb =
          jsonDecode(File('lib/l10n/app_$lang.arb').readAsStringSync())
              as Map<String, dynamic>;
      final entries = arb.entries.where(
        (e) => e.key.startsWith('squat') && e.value is String,
      );
      expect(entries, isNotEmpty);
      for (final e in entries) {
        expect(
          _forbidden.hasMatch(e.value as String),
          isFalse,
          reason: '${e.key}: ${e.value}',
        );
      }
    });
  }

  test('the list catches a claim', () {
    expect(_forbidden.hasMatch('Una postura peligrosa'), isTrue);
    expect(_forbidden.hasMatch('A safe depth'), isTrue);
  });
}
