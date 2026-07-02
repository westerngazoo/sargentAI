// SAC1/SAC2 -> AC1/AC2 (provider layer):
//   * profileProvider (FutureProvider<Profile?>) resolves to the parsed Profile
//     on 200 and to `null` when getMe() reports no profile (404 already mapped
//     to null in the data layer) — the single source of truth for "has a
//     profile?" that drives the home prompt;
//   * onboardingDismissedProvider is a session StateProvider<bool> defaulting
//     to false; dismissing flips it to true (not persisted — AC2).
//
// RED until package:fitai/src/profile/application/profile_providers.dart
// defines profileProvider, profileRepositoryProvider, and
// onboardingDismissedProvider.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/profile/application/profile_providers.dart';
import 'package:fitai/src/profile/domain/profile.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/profile_fakes.dart';

void main() {
  late MockProfileRepository repo;

  ProviderContainer makeContainer() {
    final container = ProviderContainer(
      overrides: [
        profileRepositoryProvider.overrideWithValue(repo),
        authUserIdProvider.overrideWith((_) => 'u-test'),
      ],
    );
    addTearDown(container.dispose);
    return container;
  }

  setUp(() {
    repo = MockProfileRepository();
  });

  group('SAC1 profileProvider', () {
    test('resolves to the parsed Profile when one exists (200)', () async {
      when(() => repo.getMe())
          .thenAnswer((_) async => sampleProfile(userId: 'u-1'));
      final container = makeContainer();
      final Profile? p = await container.read(profileProvider.future);
      expect(p, isNotNull);
      expect(p!.userId, 'u-1');
    });

    test('resolves to null when there is no profile (404 already mapped)',
        () async {
      when(() => repo.getMe()).thenAnswer((_) async => null);
      final container = makeContainer();
      expect(await container.read(profileProvider.future), isNull);
    });

    test('propagates a non-404 failure as an error AsyncValue', () async {
      when(() => repo.getMe()).thenThrow(StateError('boom')); // any error type
      final container = makeContainer();
      await expectLater(
        container.read(profileProvider.future),
        throwsA(anything),
      );
    });
  });

  group('SAC2 onboardingDismissedProvider', () {
    test('defaults to false (the prompt is offered)', () {
      final container = makeContainer();
      expect(container.read(onboardingDismissedProvider), isFalse);
    });

    test('dismissing flips it to true for the session', () {
      final container = makeContainer();
      container.read(onboardingDismissedProvider.notifier).state = true;
      expect(container.read(onboardingDismissedProvider), isTrue);
    });
  });
}
