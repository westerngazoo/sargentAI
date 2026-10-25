import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../../../l10n/app_localizations.dart';
import '../../core/network/api_error_text.dart';
import '../../core/network/api_exception.dart';
import '../application/session_list_controller.dart';
import '../application/workouts_provider.dart';
import '../domain/workout_session.dart';

/// The home shell's recent-sessions list (AC8) with delete (AC9). Renders
/// [workoutsProvider] via `AsyncValue.when`; the empty backlog shows the AC8
/// empty state. Delete failures are owned by [SessionListController].
class SessionList extends ConsumerWidget {
  const SessionList({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final sessions = ref.watch(workoutsProvider);
    final l10n = AppLocalizations.of(context);
    ref.listen(sessionListControllerProvider, (_, next) {
      final error = next.error;
      if (error != null) {
        ScaffoldMessenger.of(context)
          ..hideCurrentSnackBar()
          ..showSnackBar(
              SnackBar(content: Text(_deleteErrorText(error, l10n))));
      }
    });

    return sessions.when(
      loading: () => const Center(child: CircularProgressIndicator()),
      error: (_, __) => Center(
        child: TextButton(
          onPressed: () => ref.invalidate(workoutsProvider),
          child: Text(l10n.retry),
        ),
      ),
      data: (list) => list.isEmpty
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.self_improvement,
                      size: 56,
                      color: Theme.of(context).colorScheme.outlineVariant,
                    ),
                    const SizedBox(height: 12),
                    Text(l10n.noWorkoutsYet),
                  ],
                ),
              ),
            )
          : ListView.builder(
              padding: const EdgeInsets.fromLTRB(16, 4, 16, 96),
              itemCount: list.length,
              itemBuilder: (_, i) => _SessionTile(session: list[i]),
            ),
    );
  }
}

/// A deleted-already workout is the list's own wording; the rest is the
/// shared fallback.
String _deleteErrorText(ApiException e, AppLocalizations l10n) =>
    e.kind == ApiErrorKind.notFound
        ? l10n.workoutNoLongerExists
        : apiErrorText(e, l10n);

class _SessionTile extends ConsumerWidget {
  const _SessionTile({required this.session});

  final WorkoutSession session;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cs = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context);
    return Card(
      margin: const EdgeInsets.only(bottom: 10),
      child: ListTile(
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
        leading: CircleAvatar(
          backgroundColor: cs.primaryContainer,
          child: Icon(Icons.fitness_center, color: cs.onPrimaryContainer),
        ),
        title: Text(
          _date(session.performedOn, context),
          style: Theme.of(context).textTheme.titleSmall,
        ),
        subtitle: Text(
          l10n.sessionSummary(session.exercises.length, session.setCount),
        ),
        trailing: IconButton(
          tooltip: l10n.delete,
          icon: const Icon(Icons.delete_outline),
          onPressed: () => _confirmDelete(context, ref),
        ),
      ),
    );
  }

  Future<void> _confirmDelete(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        title: Text(l10n.deleteThisWorkout),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(l10n.cancel),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(l10n.delete),
          ),
        ],
      ),
    );
    if (confirmed ?? false) {
      await ref.read(sessionListControllerProvider.notifier).delete(session.id);
    }
  }
}

String _date(DateTime d, BuildContext context) {
  final now = DateTime.now();
  final today = DateTime(now.year, now.month, now.day);
  final diff = today.difference(DateTime(d.year, d.month, d.day)).inDays;
  final l10n = AppLocalizations.of(context);
  if (diff == 0) return l10n.today;
  if (diff == 1) return l10n.yesterday;
  return DateFormat.yMMMd(Localizations.localeOf(context).toString()).format(d);
}
