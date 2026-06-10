import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/network/api_exception.dart';
import '../data/workout_repository.dart';
import 'workouts_provider.dart';

@immutable
class SessionListState {
  const SessionListState({this.submitting = false, this.error});

  final bool submitting;
  final String? error;

  SessionListState copyWith({
    bool? submitting,
    String? error,
    bool clearError = false,
  }) =>
      SessionListState(
        submitting: submitting ?? this.submitting,
        error: clearError ? null : (error ?? this.error),
      );
}

final sessionListControllerProvider =
    NotifierProvider<SessionListController, SessionListState>(
  SessionListController.new,
);

/// Owns `delete(id)` with the failure-as-state pattern — no widget `try/catch`.
class SessionListController extends Notifier<SessionListState> {
  @override
  SessionListState build() => const SessionListState();

  Future<void> delete(String id) async {
    state = state.copyWith(submitting: true, clearError: true);
    try {
      await ref.read(workoutRepositoryProvider).delete(id);
      ref.invalidate(workoutsProvider);
      state = const SessionListState();
    } on ApiException catch (e) {
      if (e.statusCode == 404) {
        // Already gone server-side — the cached list is stale, so refresh it.
        ref.invalidate(workoutsProvider);
        state = state.copyWith(
          submitting: false,
          error: 'that workout no longer exists',
        );
      } else {
        state = state.copyWith(submitting: false, error: e.message);
      }
    }
  }
}
