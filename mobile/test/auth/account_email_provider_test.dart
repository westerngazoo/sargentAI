// SPEC-0061 §2.6 (slice 3c): accountEmailProvider is keyed on the session's
// user id, so an account switch re-reads GET /auth/me instead of showing the
// previous user's address.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/auth/data/auth_repository.dart';
import 'package:fitai/src/auth/domain/me.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../support/fakes.dart';

void main() {
  late MockAuthRepository repo;
  late StateProvider<String?> userId;

  setUp(() {
    repo = MockAuthRepository();
    userId = StateProvider<String?>((_) => 'u-1');
  });

  ProviderContainer build() {
    final container = ProviderContainer(overrides: [
      authRepositoryProvider.overrideWithValue(repo),
      authUserIdProvider.overrideWith((ref) => ref.watch(userId)),
    ]);
    addTearDown(container.dispose);
    return container;
  }

  test('resolves to the email of GET /auth/me', () async {
    when(() => repo.me())
        .thenAnswer((_) async => const Me(userId: 'u-1', email: 'a@b.com'));
    final container = build();
    expect(await container.read(accountEmailProvider.future), 'a@b.com');
    verify(() => repo.me()).called(1);
  });

  test('an account switch re-reads it', () async {
    when(() => repo.me())
        .thenAnswer((_) async => const Me(userId: 'u-1', email: 'a@b.com'));
    final container = build();
    container.listen(accountEmailProvider, (_, __) {});
    expect(await container.read(accountEmailProvider.future), 'a@b.com');

    when(() => repo.me())
        .thenAnswer((_) async => const Me(userId: 'u-2', email: 'c@d.com'));
    container.read(userId.notifier).state = 'u-2';
    expect(await container.read(accountEmailProvider.future), 'c@d.com');
    verify(() => repo.me()).called(2);
  });
}
