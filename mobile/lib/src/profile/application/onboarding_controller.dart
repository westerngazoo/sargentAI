import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/network/api_exception.dart';
import '../domain/goal.dart';
import '../domain/profile_draft.dart';
import '../domain/sex.dart';
import 'profile_providers.dart';

/// The wizard has three steps: body stats (0), goals (1), optional details (2).
const int onboardingStepCount = 3;

/// Immutable wizard state. Failure is **data here** (`error`/`errorStep`), never
/// a thrown exception (architect finding 3); `done` is the success signal the
/// screen `ref.listen`s to navigate.
@immutable
class OnboardingState {
  const OnboardingState({
    this.step = 0,
    this.draft = const ProfileDraft(),
    this.submitting = false,
    this.error,
    this.errorStep,
    this.done = false,
  });

  final int step;
  final ProfileDraft draft;
  final bool submitting;
  final String? error;
  final int? errorStep;
  final bool done;

  OnboardingState copyWith({
    int? step,
    ProfileDraft? draft,
    bool? submitting,
    String? error,
    int? errorStep,
    bool? done,
    bool clearError = false,
    bool clearErrorStep = false,
  }) =>
      OnboardingState(
        step: step ?? this.step,
        draft: draft ?? this.draft,
        submitting: submitting ?? this.submitting,
        error: clearError ? null : (error ?? this.error),
        errorStep: clearErrorStep ? null : (errorStep ?? this.errorStep),
        done: done ?? this.done,
      );
}

final onboardingControllerProvider =
    NotifierProvider<OnboardingController, OnboardingState>(
  OnboardingController.new,
);

class OnboardingController extends Notifier<OnboardingState> {
  @override
  OnboardingState build() => const OnboardingState();

  void setBodyStats({DateTime? dob, int? height, double? weight}) {
    state = state.copyWith(
      draft: state.draft
          .copyWith(dateOfBirth: dob, heightCm: height, weightKg: weight),
    );
  }

  void toggleGoal(Goal g) {
    final goals = Set<Goal>.of(state.draft.goals);
    if (!goals.add(g)) goals.remove(g);
    state = state.copyWith(draft: state.draft.copyWith(goals: goals));
  }

  void setOptional({
    Sex? sex,
    double? bodyFat,
    bool clearSex = false,
    bool clearBodyFat = false,
  }) {
    state = state.copyWith(
      draft: state.draft.copyWith(
        sex: sex,
        bodyFatPercentage: bodyFat,
        clearSex: clearSex,
        clearBodyFat: clearBodyFat,
      ),
    );
  }

  void next() {
    if (state.step < onboardingStepCount - 1) {
      state = state.copyWith(step: state.step + 1);
    }
  }

  void back() {
    if (state.step > 0) state = state.copyWith(step: state.step - 1);
  }

  /// PUT the profile, then on success refresh [profileProvider] (so the shell
  /// re-reads and drops the prompt) and set `done`. On failure, record the
  /// message + target step on the state — no throw, draft untouched (AC8).
  Future<void> submit() async {
    final req = state.draft.toRequest(DateTime.now());
    if (req == null) return; // guard: never submit invalid (no-op)
    state = state.copyWith(
        submitting: true, clearError: true, clearErrorStep: true);
    try {
      await ref.read(profileRepositoryProvider).putMe(req);
      ref.invalidate(profileProvider);
      await ref.read(profileProvider.future); // re-read so /home shows it (AC7)
      state = state.copyWith(submitting: false, done: true);
    } on ApiException catch (e) {
      final target = _stepFor(e.field);
      state = state.copyWith(
        submitting: false,
        error: e.message,
        errorStep: target,
        step: target ?? state.step, // jump the wizard to the offending step
      );
    }
  }

  /// Map a backend `400` field to the step that owns it.
  int? _stepFor(String? field) => switch (field) {
        'date_of_birth' || 'height_cm' || 'weight_kg' => 0,
        'body_fat_percentage' => 2,
        _ => null,
      };
}
