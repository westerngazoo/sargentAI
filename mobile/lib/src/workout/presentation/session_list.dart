import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

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
    ref.listen(sessionListControllerProvider, (_, next) {
      final error = next.error;
      if (error != null) {
        ScaffoldMessenger.of(context)
          ..hideCurrentSnackBar()
          ..showSnackBar(SnackBar(content: Text(error)));
      }
    });

    return sessions.when(
      loading: () => const Center(child: CircularProgressIndicator()),
      error: (_, __) => Center(
        child: TextButton(
          onPressed: () => ref.invalidate(workoutsProvider),
          child: const Text('Retry'),
        ),
      ),
      data: (list) => list.isEmpty
          ? const Center(
              child: Padding(
                padding: EdgeInsets.all(24),
                child:
                    Text('no workouts yet — tap start to log your first one'),
              ),
            )
          : ListView.builder(
              itemCount: list.length,
              itemBuilder: (_, i) => _SessionTile(session: list[i]),
            ),
    );
  }
}

class _SessionTile extends ConsumerWidget {
  const _SessionTile({required this.session});

  final WorkoutSession session;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return ListTile(
      title: Text(_date(session.performedOn)),
      subtitle: Text(
        '${session.exercises.length} exercises · ${session.setCount} sets',
      ),
      trailing: IconButton(
        tooltip: 'Delete',
        icon: const Icon(Icons.delete_outline),
        onPressed: () => _confirmDelete(context, ref),
      ),
    );
  }

  Future<void> _confirmDelete(BuildContext context, WidgetRef ref) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        title: const Text('Delete this workout?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (confirmed ?? false) {
      await ref.read(sessionListControllerProvider.notifier).delete(session.id);
    }
  }
}

String _date(DateTime d) => '${d.year.toString().padLeft(4, '0')}-'
    '${d.month.toString().padLeft(2, '0')}-'
    '${d.day.toString().padLeft(2, '0')}';
