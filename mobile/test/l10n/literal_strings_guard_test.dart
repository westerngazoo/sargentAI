// R-0061 AC11 -> SPEC-0061 §2.9: no literal UI string in a widget. A string
// handed straight to Text(, tooltip:, label:, hintText:, labelText:, title:,
// semanticLabel: or the voice loop's _say( is a string the user sees in one
// language only.
//
// The pattern matches across line breaks, because `dart format` puts a long
// literal on the line after `Text(`. It is a regex smoke test: a string built
// far from its widget slips through, and reviews catch those.
//
// The allowlist names the folders slices 1b–1d have not migrated yet. It
// shrinks slice by slice; a folder that no longer holds a literal must be
// removed from it, or this test fails, so it cannot go stale.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// Feature folders under lib/src still waiting for slices 1b–1d.
const notYetMigrated = <String>{
  'hub',
  'measurements',
  'nutrition',
  'workout',
};

final _literal = RegExp(
  r'''(?:\bText\s*\(|\b(?:tooltip|label|hintText|labelText|title|semanticLabel)\s*:\s*(?:const\s+)?(?:Text\s*\()?|\b_say\s*\()\s*(['"])(?:(?!\1).)+\1''',
  dotAll: true,
);

String _folderOf(String path) {
  final parts = path.split('/');
  final i = parts.indexOf('src');
  return i >= 0 && i + 1 < parts.length ? parts[i + 1] : '';
}

void main() {
  final files = Directory('lib/src')
      .listSync(recursive: true)
      .whereType<File>()
      .where((f) => f.path.endsWith('.dart'))
      .toList();

  test('no literal UI string outside the not-yet-migrated folders', () {
    final offenders = <String>[];
    for (final f in files) {
      if (notYetMigrated.contains(_folderOf(f.path))) continue;
      final src = f.readAsStringSync();
      for (final m in _literal.allMatches(src)) {
        final line = '\n'.allMatches(src.substring(0, m.start)).length + 1;
        offenders.add('${f.path}:$line: ${m.group(0)!.split('\n').first}');
      }
    }
    expect(offenders, isEmpty, reason: offenders.join('\n'));
  });

  test('every allowlisted folder still holds a literal (else remove it)', () {
    for (final folder in notYetMigrated) {
      final hasLiteral = files
          .where((f) => _folderOf(f.path) == folder)
          .any((f) => _literal.hasMatch(f.readAsStringSync()));
      expect(hasLiteral, isTrue,
          reason: '$folder has no literal left: drop it from notYetMigrated');
    }
  });
}
