// SAC3/SAC7/SAC8/SAC9 -> AC3/AC7/AC8/AC9 (wizard state machine):
//   * draft lives in the controller and survives step navigation (SAC3);
//   * submit() PUTs the profile, refreshes profileProvider, sets done=true on
//     200 (SAC7) — the screen ref.listens `done` and navigates (not here);
//   * a 400 sets error + errorStep (field -> step), draft intact (SAC8);
//   * a transport error sets a retryable error, draft intact (SAC8);
//   * a 401 surfaces as an error but does NOT navigate — the shared
//     AuthInterceptor already sinks the session (SAC8);
//   * failure is DATA on OnboardingState, never a thrown exception (SAC9):
//     submit() does not rethrow.
//
// RED until package:fitai/src/profile/application/onboarding_controller.dart
// defines OnboardingController/OnboardingState/onboardingControllerProvider and
// package:fitai/src/profile/application/profile_providers.dart defines the
// profile/repository providers.

import 'package:fitai/src/auth/application/auth_controller.dart';
import 'package:fitai/src/core/network/api_exception.dart';
import 'package:fitai/src/profile/application/onboarding_controller.dart';
import 'package:fitai/src/profile/application/profile_providers.dart';
import 'package:fitai/src/profile/domain/goal.dart';
import 'package:fitai/src/profile/domain/sex.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';

import '../../support/profile_fakes.dart';

void main() {
  setUpAll(registerProfileFallbacks);

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

  OnboardingController controllerOf(ProviderContainer c) =>
      c.read(onboardingControllerProvider.notifier);

  // Fills the draft with valid required fields so submit() reaches the network.
  void enterValidProfile(OnboardingController c) {
    c.setBodyStats(
      dob: DateTime(1996, 6, 6),
      height: 180,
      weight: 82.5,
    );
    c.toggleGoal(Goal.buildMuscle);
  }

  setUp(() {
    repo = MockProfileRepository();
    // After a successful PUT the controller refreshes profileProvider, which
    // re-reads getMe(); stub it so the refresh resolves.
    when(() => repo.getMe()).thenAnswer((_) async => sampleProfile());
  });

  group('SAC3 wizard navigation + draft survival', () {
    test('starts at step 0 with an empty draft', () {
      final c = controllerOf(makeContainer());
      final s = c.state;
      expect(s.step, 0);
      expect(s.draft.goals, isEmpty);
      expect(s.done, isFalse);
      expect(s.submitting, isFalse);
    });

    test('next/back move the step within bounds and never go negative', () {
      final c = controllerOf(makeContainer());
      c.next();
      expect(c.state.step, 1);
      c.back();
      expect(c.state.step, 0);
      c.back(); // already at 0 — clamped
      expect(c.state.step, 0);
    });

    test('body-stats entered in step 1 persist after navigating away and back',
        () {
      final c = controllerOf(makeContainer());
      c.setBodyStats(dob: DateTime(1990, 1, 1), height: 175, weight: 70.0);
      c.next(); // to goals
      c.toggleGoal(Goal.recomp);
      c.next(); // to optional
      c.back();
      c.back(); // back to body stats
      expect(c.state.draft.heightCm, 175);
      expect(c.state.draft.weightKg, 70.0);
      expect(c.state.draft.dateOfBirth, DateTime(1990, 1, 1));
      expect(c.state.draft.goals, contains(Goal.recomp));
    });

    test('toggleGoal adds then removes a goal', () {
      final c = controllerOf(makeContainer());
      c.toggleGoal(Goal.loseFat);
      expect(c.state.draft.goals, contains(Goal.loseFat));
      c.toggleGoal(Goal.loseFat);
      expect(c.state.draft.goals, isNot(contains(Goal.loseFat)));
    });

    test('setOptional can set then clear sex without touching body stats', () {
      final c = controllerOf(makeContainer());
      c.setBodyStats(dob: DateTime(1990, 1, 1), height: 175, weight: 70.0);
      c.setOptional(sex: Sex.male, bodyFat: 20.0);
      expect(c.state.draft.sex, Sex.male);
      expect(c.state.draft.bodyFatPercentage, 20.0);
      c.setOptional(clearSex: true);
      expect(c.state.draft.sex, isNull);
      expect(c.state.draft.heightCm, 175, reason: 'body stats untouched');
    });
  });

  group('SAC7 submit success', () {
    test('a 200 PUT sets done=true and clears submitting/error', () async {
      when(() => repo.putMe(any())).thenAnswer((_) async => sampleProfile());
      final container = makeContainer();
      final c = controllerOf(container);
      enterValidProfile(c);

      await c.submit();

      expect(c.state.done, isTrue);
      expect(c.state.submitting, isFalse);
      expect(c.state.error, isNull);
      verify(() => repo.putMe(any())).called(1);
    });

    test('submit refreshes the profile so home sees it before navigation',
        () async {
      when(() => repo.putMe(any())).thenAnswer((_) async => sampleProfile());
      final container = makeContainer();
      final c = controllerOf(container);
      enterValidProfile(c);

      await c.submit();

      // The refresh re-read the profile (getMe) so the prompt is gone on /home.
      verify(() => repo.getMe()).called(greaterThanOrEqualTo(1));
    });

    test('submit is a no-op when required fields are invalid (no PUT)',
        () async {
      final container = makeContainer();
      final c = controllerOf(container);
      // No body stats, no goals -> toRequest() returns null.
      await c.submit();
      verifyNever(() => repo.putMe(any()));
      expect(c.state.done, isFalse);
    });
  });

  group('SAC8 submit failure is DATA on the state (no rethrow, no data loss)',
      () {
    test('a 400{field} sets error + errorStep and keeps the draft intact',
        () async {
      when(() => repo.putMe(any())).thenThrow(
        const ApiException('please check your details',
            statusCode: 400, field: 'height_cm'),
      );
      final container = makeContainer();
      final c = controllerOf(container);
      enterValidProfile(c);
      final before = c.state.draft;

      await c.submit(); // must NOT throw

      expect(c.state.error, isNotNull);
      expect(c.state.errorStep, 0, reason: 'height_cm -> body-stats step');
      expect(c.state.done, isFalse);
      expect(c.state.submitting, isFalse);
      expect(c.state.draft, before, reason: 'no data loss');
    });

    test('a 400 on body_fat_percentage jumps to the optional step (2)',
        () async {
      when(() => repo.putMe(any())).thenThrow(
        const ApiException('out of range',
            statusCode: 400, field: 'body_fat_percentage'),
      );
      final c = controllerOf(makeContainer());
      enterValidProfile(c);

      await c.submit();

      expect(c.state.errorStep, 2);
    });

    test('a transport error sets a retryable message, draft intact', () async {
      when(() => repo.putMe(any())).thenThrow(
        const ApiException("can't reach the server — retry"),
      );
      final c = controllerOf(makeContainer());
      enterValidProfile(c);
      final before = c.state.draft;

      await c.submit();

      expect(c.state.error, contains('retry'));
      expect(c.state.done, isFalse);
      expect(c.state.draft, before);
    });

    test('a 401 surfaces as state error but never sets done (no navigation)',
        () async {
      when(() => repo.putMe(any())).thenThrow(
        const ApiException('unauthorized', statusCode: 401),
      );
      final c = controllerOf(makeContainer());
      enterValidProfile(c);

      await c.submit();

      expect(c.state.done, isFalse);
    });
  });
}
