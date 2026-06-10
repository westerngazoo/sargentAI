import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/network/api_exception.dart';
import '../data/workout_repository.dart';
import '../domain/exercise_draft.dart';
import '../domain/muscle_group.dart';
import '../domain/session_draft.dart';
import '../domain/set_draft.dart';
import 'workouts_provider.dart';

/// The in-progress session as explicit state. `canFinish`/`lastSet` are derived
/// reactively (pure functions of the fields); failure is **data** here
/// (`error`/`errorField`), never a thrown exception.
@immutable
class SessionDriverState {
  const SessionDriverState({
    this.draft,
    this.currentExercise = 0,
    this.submitting = false,
    this.error,
    this.errorField,
    this.done = false,
  });

  final SessionDraft? draft;
  final int currentExercise;
  final bool submitting;
  final String? error;
  final String? errorField;
  final bool done;

  bool get canFinish => draft?.canFinish ?? false;

  /// The last set of the **current** exercise (the repeat-last-set source).
  SetDraft? get lastSet {
    final d = draft;
    if (d == null ||
        currentExercise < 0 ||
        currentExercise >= d.exercises.length) {
      return null;
    }
    final sets = d.exercises[currentExercise].sets;
    return sets.isEmpty ? null : sets.last;
  }

  SessionDriverState copyWith({
    SessionDraft? draft,
    int? currentExercise,
    bool? submitting,
    String? error,
    String? errorField,
    bool? done,
    bool clearError = false,
  }) =>
      SessionDriverState(
        draft: draft ?? this.draft,
        currentExercise: currentExercise ?? this.currentExercise,
        submitting: submitting ?? this.submitting,
        error: clearError ? null : (error ?? this.error),
        errorField: clearError ? null : (errorField ?? this.errorField),
        done: done ?? this.done,
      );
}

final sessionDriverProvider =
    NotifierProvider<SessionDriver, SessionDriverState>(SessionDriver.new);

/// The R-0027 seam: a widget-independent state machine. A voice transport calls
/// exactly this API; a rejected call's returned message is what it speaks back.
class SessionDriver extends Notifier<SessionDriverState> {
  @override
  SessionDriverState build() => const SessionDriverState();

  void start() => state =
      const SessionDriverState(draft: SessionDraft(), currentExercise: 0);

  /// Validates and appends a new exercise, selecting it. Returns `null` on
  /// accept or a user-safe reason on reject (driver = single enforcement
  /// point); the reject does NOT touch `state.error`.
  String? addExercise(String name, {MuscleGroup? group}) {
    final d = state.draft;
    if (d == null) return 'start a workout first';
    final candidate = ExerciseDraft(name: name.trim(), muscleGroup: group);
    final invalid = candidate.nameError();
    if (invalid != null) return invalid;
    final exercises = [...d.exercises, candidate];
    state = state.copyWith(
      draft: SessionDraft(exercises: exercises),
      currentExercise: exercises.length - 1,
    );
    return null;
  }

  /// Validates and appends a set to the current exercise. Returns `null` on
  /// accept or a reason on reject (same contract as [addExercise]).
  String? logSet(SetDraft set) {
    final d = state.draft;
    if (d == null) return 'start a workout first';
    final i = state.currentExercise;
    if (i < 0 || i >= d.exercises.length) return 'add an exercise first';
    final invalid = set.repsError() ?? set.weightError() ?? set.rpeError();
    if (invalid != null) return invalid;
    final exercises = [...d.exercises];
    exercises[i] = exercises[i].copyWith(sets: [...exercises[i].sets, set]);
    state = state.copyWith(draft: SessionDraft(exercises: exercises));
    return null;
  }

  /// Switch the current exercise; out-of-range is a no-op (clamp).
  void selectExercise(int index) {
    final d = state.draft;
    if (d == null || index < 0 || index >= d.exercises.length) return;
    state = state.copyWith(currentExercise: index);
  }

  /// `POST /workouts`, then re-read the list BEFORE flipping `done` (so home
  /// shows the session on arrival). Failure is data on the state; the draft is
  /// untouched, so a retry re-submits the same content.
  Future<void> finish() async {
    final d = state.draft;
    if (d == null || !d.canFinish) return;
    final req = d.toRequest(DateTime.now()); // local calendar date, at the edge
    if (req == null) return;
    state = state.copyWith(submitting: true, clearError: true);
    try {
      await ref.read(workoutRepositoryProvider).create(req);
      ref.invalidate(workoutsProvider);
      await ref.read(workoutsProvider.future);
      state = const SessionDriverState(done: true); // draft cleared
    } on ApiException catch (e) {
      state = state.copyWith(
        submitting: false,
        error: e.message,
        errorField: e.field,
      );
    }
  }

  void abandon() => state = const SessionDriverState();
}
