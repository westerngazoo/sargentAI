// R-0061 AC11 -> SPEC-0061 §2.9: no literal UI string in a widget. A string
// handed straight to Text(, tooltip:, label:, hintText:, labelText:, title:,
// semanticLabel: or the voice loop's _say( is a string the user sees in one
// language only.
//
// The pattern matches across line breaks, because `dart format` puts a long
// literal on the line after `Text(`. It is a regex smoke test: a string built
// far from its widget slips through, and reviews catch those.
//
// Slices 1b–1d carried a shrinking allowlist of not-yet-migrated folders;
// 1d emptied it, so every literal under lib/src now fails the build.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

final _literal = RegExp(
  r'''(?:\bText\s*\(|\b(?:tooltip|label|hintText|labelText|title|semanticLabel)\s*:\s*(?:const\s+)?(?:Text\s*\()?|\b_say\s*\()\s*(['"])(?:(?!\1).)+\1''',
  dotAll: true,
);

void main() {
  final files = Directory('lib/src')
      .listSync(recursive: true)
      .whereType<File>()
      .where((f) => f.path.endsWith('.dart'))
      .toList();

  test('no literal UI string anywhere under lib/src', () {
    final offenders = <String>[];
    for (final f in files) {
      final src = f.readAsStringSync();
      for (final m in _literal.allMatches(src)) {
        final line = '\n'.allMatches(src.substring(0, m.start)).length + 1;
        offenders.add('${f.path}:$line: ${m.group(0)!.split('\n').first}');
      }
    }
    expect(offenders, isEmpty, reason: offenders.join('\n'));
  });
}
