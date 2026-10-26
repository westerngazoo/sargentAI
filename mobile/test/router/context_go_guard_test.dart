// R-0061 AC1 -> SPEC-0061 §2.5: nothing replaces the main screen without a
// way back. `context.go` replaces the stack, so outside the router, the
// screens before sign-in and onboarding it is a trap. A source scan fails on
// any other call site.
//
// The allowlist of folders that still used it shrank with slice 3a (the
// panels took program and measurements off it) and slice 5 (focus mode), and
// slice 3b emptied it with the hub, so every call site outside [allowed] now
// fails the build. [allowed] never grows.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// Paths where `context.go(` is the right tool.
const allowed = <String>{
  'lib/src/router/',
  'lib/src/auth/presentation/',
  'lib/src/profile/presentation/onboarding_screen.dart',
};

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
      final src = f.readAsStringSync();
      for (final m in go.allMatches(src)) {
        final line = '\n'.allMatches(src.substring(0, m.start)).length + 1;
        offenders.add('${f.path}:$line');
      }
    }
    expect(offenders, isEmpty, reason: offenders.join('\n'));
  });
}
