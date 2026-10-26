import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/dates.dart';
import '../../core/network/api_exception.dart';
import '../data/workout_repository.dart';
import '../domain/draft_rejection.dart';
import '../domain/exercise_draft.dart';
import '../domain/muscle_group.dart';
import '../domain/plan_seed.dart';
import '../domain/preset_exercises.dart';
import '../domain/save_notice.dart';
import '../domain/session_draft.dart';
import '../domain/set_draft.dart';
import 'workouts_provider.dart';

/// The in-progress session as explicit state. `canFinish`/`lastSet` are derived
/// reactively (pure functions of the fields); failure is **data** here
/// (`error`, with the offending field inside it), never a thrown exception.
@immutable
class SessionDriverState {
  const SessionDriverState({
    this.draft,
    this.seed,
    this.performedOn,
    this.currentExercise = 0,
    this.submitting = false,
    this.error,
    this.done = false,
    this.notice,
  });

  final SessionDraft? draft;

  /// The plan day this session is for (SPEC-0061 §2.7); null for a free
  /// session.
  final PlanSeed? seed;

  /// While in progress: the day a late log is for, or `null` for today at
  /// finish time. Once [done]: the day the session was saved on.
  final DateTime? performedOn;
  final int currentExercise;
  final bool submitting;
  final ApiException? error;
  final bool done;

  /// Set with [done] when the server refused the plan link and the session
  /// was saved as free instead (§2.4).
  final SaveNotice? notice;

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
    ApiException? error,
    bool clearError = false,
  }) =>
      SessionDriverState(
        draft: draft ?? this.draft,
        seed: seed,
        performedOn: performedOn,
        currentExercise: currentExercise ?? this.currentExercise,
        submitting: submitting ?? this.submitting,
        error: clearError ? null : (error ?? this.error),
        done: done,
        notice: notice,
      );
}

/// The refusals of a plan link the client saves around (SPEC-0061 §2.4);
/// null for any other failure.
SaveNotice? _refusedPlanLink(ApiException e) =>
    switch ((e.statusCode, e.reason)) {
      (409, 'program_not_current') => SaveNotice.programNotCurrent,
      (422, 'plan_unavailable') => SaveNotice.planUnavailable,
      _ => null,
    };

final sessionDriverProvider =
    NotifierProvider<SessionDriver, SessionDriverState>(SessionDriver.new);

/// The R-0027 seam: a widget-independent state machine. A voice transport calls
/// exactly this API; a rejected call's returned [DraftRejection] is what it
/// speaks back, through the catalogue.
class SessionDriver extends Notifier<SessionDriverState> {
  @override
  SessionDriverState build() => const SessionDriverState();

  /// Opens an in-progress session. With a [seed] it is a plan session,
  /// pre-filled with the seed's exercises (names only, no sets; the group
  /// comes from the preset catalogue, as the voice coach resolves it, so
  /// per-muscle volume sees the lift); without one it is free. [performedOn]
  /// is the day it is logged on (R-0061 AC5 "Registrar tarde"); today when
  /// omitted.
  void start({PlanSeed? seed, DateTime? performedOn}) =>
      state = SessionDriverState(
        draft: SessionDraft(exercises: [
          for (final name in seed?.exercises ?? const <String>[])
            ExerciseDraft(name: name, muscleGroup: presetGroupFor(name)),
        ]),
        seed: seed,
        currentExercise: 0,
        performedOn: performedOn,
      );

  /// Validates and appends a new exercise, selecting it. Returns `null` on
  /// accept or the typed reason on reject (driver = single enforcement
  /// point); the reject does NOT touch `state.error`.
  DraftRejection? addExercise(String name, {MuscleGroup? group}) {
    final d = state.draft;
    if (d == null) return DraftRejection.noSession;
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
  DraftRejection? logSet(SetDraft set) {
    final d = state.draft;
    if (d == null) return DraftRejection.noSession;
    final i = state.currentExercise;
    if (i < 0 || i >= d.exercises.length) return DraftRejection.noExercise;
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
  /// untouched, so a retry re-submits the same content. Every way to finish
  /// (button, voice, earbud) ends here, and `done` is what focus mode pops on
  /// (SPEC-0061 §2.7).
  Future<void> finish() async {
    final d = state.draft;
    if (d == null || !d.canFinish) return;
    // The chosen day for a late log, else the local calendar date, at the edge.
    final performedOn = state.performedOn ?? DateTime.now();
    final req = d.toRequest(performedOn, plan: state.seed);
    if (req == null) return;
    state = state.copyWith(submitting: true, clearError: true);
    try {
      final notice = await _save(req);
      ref.invalidate(workoutsProvider);
      await ref.read(workoutsProvider.future);
      state = SessionDriverState(
        done: true, // draft cleared
        performedOn: dateOnly(performedOn),
        notice: notice,
      );
    } on ApiException catch (e) {
      state = state.copyWith(submitting: false, error: e);
    }
  }

  /// Saves [req]. A plan link the server refuses is dropped and the session
  /// saved again as free, returning why (SPEC-0061 §2.4), so a finished
  /// session is never lost; any other failure propagates.
  Future<SaveNotice?> _save(SessionRequest req) async {
    final repo = ref.read(workoutRepositoryProvider);
    try {
      await repo.create(req);
      return null;
    } on ApiException catch (e) {
      final notice = req.plan == null ? null : _refusedPlanLink(e);
      if (notice == null) rethrow;
      await repo.create(req.asFree());
      return notice;
    }
  }

  void abandon() => state = const SessionDriverState();
}
