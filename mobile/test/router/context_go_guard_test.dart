// R-0061 AC1 -> SPEC-0061 §2.5: nothing replaces the main screen without a
// way back. `context.go` replaces the stack, so outside the router, the
// screens before sign-in and onboarding it is a trap. A source scan fails on
// any other call site.
//
// The allowlist of folders that still use it shrank with slice 3a (the
// panels took program and measurements off it) and slice 5 (focus mode); the
// hub leaves with 3b. A folder that no longer calls it must be removed, or
// the second test fails.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// Paths where `context.go(` is the right tool.
const allowed = <String>{
  'lib/src/router/',
  'lib/src/auth/presentation/',
  'lib/src/profile/presentation/onboarding_screen.dart',
};

/// Feature folders slice 3b has not migrated to push/pop/showPanel.
const notYetMigrated = <String>{'hub'};

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
  final go = RegExp(r'\bcontext\.go\(');

  test('context.go only where it is allowed', () {
    final offenders = <String>[];
    for (final f in files) {
      if (allowed.any(f.path.startsWith)) continue;
      if (notYetMigrated.contains(_folderOf(f.path))) continue;
      final src = f.readAsStringSync();
      for (final m in go.allMatches(src)) {
        final line = '\n'.allMatches(src.substring(0, m.start)).length + 1;
        offenders.add('${f.path}:$line');
      }
    }
    expect(offenders, isEmpty, reason: offenders.join('\n'));
  });

  test('every not-yet-migrated folder still calls context.go', () {
    for (final folder in notYetMigrated) {
      final still = files
          .where((f) => _folderOf(f.path) == folder)
          .any((f) => go.hasMatch(f.readAsStringSync()));
      expect(still, isTrue,
          reason: '$folder no longer calls context.go: drop it from the list');
    }
  });
}
