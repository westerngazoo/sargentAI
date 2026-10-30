// R-0059 AC7 -> SPEC-0059 §2.3 (slice 59-b): "La cuenta" formats the
// server's numbers in the locale's number format and computes nothing. The
// test checks the string, never a sum.

import 'package:fitai/l10n/app_localizations.dart';
import 'package:fitai/src/physics/domain/lever_text.dart';
import 'package:fitai/src/physics/domain/squat_range.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/l10n.dart';
import '../../support/physics_fakes.dart';

void main() {
  final en = lookupAppLocalizations(const Locale('en'));

  test('the upright frame in Spanish, with the decimal point (es_MX)', () {
    expect(
      leverText(squatGolden().frames.first, l10n),
      '824 N × 0.03 m = 23.8 N·m, + 9.7 del cuerpo = 33.5 N·m',
    );
  });

  test('the bottom frame in English', () {
    expect(
      leverText(squatGolden().frames.last, en),
      '824 N × 0.38 m = 311.6 N·m, + 126.9 from the body = 438.4 N·m',
    );
  });

  test('the total is the server hip moment, not a client sum', () {
    final json = squatGoldenJson();
    final frame = (json['frames'] as List).first as Map<String, dynamic>;
    final hip = ((frame['mechanics'] as Map<String, dynamic>)['joints']
        as List)[0] as Map<String, dynamic>;
    hip['moment'] = 50.0; // not 23.8 + 9.7: the text must still say 50
    expect(
      leverText(SquatRangeResult.fromJson(json).frames.first, en),
      '824 N × 0.03 m = 23.8 N·m, + 9.7 from the body = 50.0 N·m',
    );
  });

  test('a bodyweight frame has no lever text', () {
    final json = squatGoldenJson();
    ((json['frames'] as List).first as Map<String, dynamic>).remove('hip');
    expect(leverText(SquatRangeResult.fromJson(json).frames.first, en), isNull);
  });
}
