// SAC8 -> AC8: the sealed AuthState union (AuthUnknown | AuthUnauthenticated |
// AuthAuthenticated(Session)) is the single source of truth the router switches
// on. This pins the union shape so the router's exhaustive switch compiles.
//
// RED until package:fitai/src/auth/domain/auth_state.dart and session.dart
// define these types.

import 'package:fitai/src/auth/domain/auth_state.dart';
import 'package:fitai/src/auth/domain/session.dart';
import 'package:flutter_test/flutter_test.dart';

// A local exhaustive switch — if a new AuthState subtype is ever added without
// updating the router, this fails to compile too (mirrors SPEC-0007 §2.7).
String classify(AuthState s) => switch (s) {
      AuthUnknown() => 'unknown',
      AuthUnauthenticated() => 'unauthenticated',
      AuthAuthenticated() => 'authenticated',
    };

void main() {
  group('SAC8 sealed AuthState', () {
    test('AuthUnknown classifies as unknown', () {
      expect(classify(const AuthUnknown()), 'unknown');
    });

    test('AuthUnauthenticated classifies as unauthenticated', () {
      expect(classify(const AuthUnauthenticated()), 'unauthenticated');
    });

    test('AuthAuthenticated carries a Session', () {
      const session = Session(userId: 'u-1');
      const state = AuthAuthenticated(session);
      expect(classify(state), 'authenticated');
      expect(state.session.userId, 'u-1');
    });
  });
}
