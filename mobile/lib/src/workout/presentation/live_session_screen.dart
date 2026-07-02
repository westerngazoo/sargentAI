import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../application/session_driver.dart';
import '../application/voice_coach.dart';
import '../domain/exercise_draft.dart';
import '../domain/muscle_activation.dart';
import '../domain/muscle_group.dart';
import '../domain/set_draft.dart';
import 'muscle_map.dart';
import 'preset_exercises.dart';

/// The live in-gym screen — a THIN renderer over [sessionDriverProvider]. All
/// business logic (validation, the state machine) is driver-side; this widget
/// holds no `try/catch`. A `null` draft (deep link or post-finish) redirects
/// home: the driver state, not the route, owns "a session is in progress".
class LiveSessionScreen extends ConsumerWidget {
  const LiveSessionScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(sessionDriverProvider);
    final driver = ref.read(sessionDriverProvider.notifier);
    final coach = ref.watch(voiceCoachProvider);
    final draft = state.draft;

    if (draft == null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (context.mounted) context.go('/home');
      });
      return const Scaffold(body: SizedBox.shrink());
    }

    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) async {
        if (didPop) return;
        final discard = await showDialog<bool>(
          context: context,
          builder: (_) => AlertDialog(
            title: const Text('Discard this workout?'),
            content: const Text('Your logged sets will be lost.'),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(context).pop(false),
                child: const Text('Cancel'),
              ),
              TextButton(
                onPressed: () => Navigator.of(context).pop(true),
                child: const Text('Discard'),
              ),
            ],
          ),
        );
        if (discard ?? false) driver.abandon();
      },
      child: Scaffold(
        appBar: AppBar(
          title: const Text('Workout'),
          actions: [
            IconButton(
              tooltip: coach.enabled ? 'Voice coach off' : 'Voice coach on',
              isSelected: coach.enabled,
              icon: const Icon(Icons.headset_off_outlined),
              selectedIcon: const Icon(Icons.headset_mic),
              onPressed: () {
                final notifier = ref.read(voiceCoachProvider.notifier);
                coach.enabled ? notifier.disable() : notifier.enable();
              },
            ),
            const SizedBox(width: 8),
          ],
        ),
        body: SafeArea(
          child: Column(
            children: [
              if (state.error != null)
                _ErrorBanner(error: state.error!, field: state.errorField),
              Expanded(
                child: ListView(
                  padding: const EdgeInsets.all(16),
                  children: [
                    if (draft.exercises.isNotEmpty)
                      _TargetMusclesCard(
                        exercise: draft.exercises[state.currentExercise
                            .clamp(0, draft.exercises.length - 1)],
                      ),
                    for (var i = 0; i < draft.exercises.length; i++)
                      _ExerciseCard(
                        index: i,
                        exercise: draft.exercises[i],
                        isCurrent: i == state.currentExercise,
                      ),
                    const SizedBox(height: 8),
                    OutlinedButton.icon(
                      onPressed: () => _showAddSheet(context, driver),
                      icon: const Icon(Icons.add),
                      label: const Text('Add exercise'),
                    ),
                  ],
                ),
              ),
              if (coach.enabled) _CoachBar(coach: coach),
              _FinishBar(state: state, driver: driver),
            ],
          ),
        ),
      ),
    );
  }

  void _showAddSheet(BuildContext context, SessionDriver driver) {
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      builder: (_) => _AddExerciseSheet(driver: driver),
    );
  }
}

/// The activated-muscles panel (ported from the-goose-factor): the current
/// exercise's primary movers in olive, assisters in brass, on the anatomy
/// chart.
class _TargetMusclesCard extends StatelessWidget {
  const _TargetMusclesCard({required this.exercise});

  final ExerciseDraft exercise;

  @override
  Widget build(BuildContext context) {
    final activation =
        activationFor(exercise.name, group: exercise.muscleGroup);
    if (activation.isEmpty) return const SizedBox.shrink();
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'TARGET MUSCLES — ${exercise.name.toUpperCase()}',
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                    letterSpacing: 1.1,
                    fontWeight: FontWeight.w700,
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
            ),
            const SizedBox(height: 8),
            MuscleMap(activation: activation),
          ],
        ),
      ),
    );
  }
}

/// The voice-coach strip: mic to dictate a set ("10 reps at 100 kilos",
/// "next", "finish workout") and the coach's last line for screen-on use.
class _CoachBar extends ConsumerWidget {
  const _CoachBar({required this.coach});

  final VoiceCoachState coach;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cs = Theme.of(context).colorScheme;
    return Container(
      margin: const EdgeInsets.fromLTRB(16, 0, 16, 8),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        color: cs.primaryContainer,
        borderRadius: BorderRadius.circular(18),
      ),
      child: Row(
        children: [
          IconButton.filled(
            tooltip: coach.listening ? 'Stop listening' : 'Dictate a set',
            icon: Icon(coach.listening ? Icons.stop : Icons.mic),
            style: IconButton.styleFrom(
              backgroundColor: coach.listening ? cs.error : cs.primary,
              foregroundColor: cs.onPrimary,
            ),
            onPressed: () => ref.read(voiceCoachProvider.notifier).dictate(),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              coach.listening
                  ? (coach.transcript.isEmpty
                      ? 'Listening…'
                      : '“${coach.transcript}”')
                  : (coach.coachLine.isEmpty
                      ? 'Tap the mic and say "done" — I will ask your reps '
                          'and kilos. End every answer with "over".'
                      : coach.coachLine),
              style: Theme.of(context)
                  .textTheme
                  .bodySmall
                  ?.copyWith(color: cs.onPrimaryContainer),
            ),
          ),
        ],
      ),
    );
  }
}

/// The finish-failure banner. When the backend named a field, it appends the
/// area of the screen to check (`fieldArea`), so the message is field-aware —
/// the context the R-0027 voice transport will speak back.
class _ErrorBanner extends StatelessWidget {
  const _ErrorBanner({required this.error, this.field});

  final String error;
  final String? field;

  @override
  Widget build(BuildContext context) {
    final area = fieldArea(field);
    final message = area == null ? error : '$error — check $area';
    return Container(
      width: double.infinity,
      color: Theme.of(context).colorScheme.errorContainer,
      padding: const EdgeInsets.all(12),
      child: Text(
        message,
        style: TextStyle(color: Theme.of(context).colorScheme.onErrorContainer),
      ),
    );
  }
}

class _ExerciseCard extends ConsumerWidget {
  const _ExerciseCard({
    required this.index,
    required this.exercise,
    required this.isCurrent,
  });

  final int index;
  final ExerciseDraft exercise;
  final bool isCurrent;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final driver = ref.read(sessionDriverProvider.notifier);
    return Card(
      child: InkWell(
        onTap: isCurrent ? null : () => driver.selectExercise(index),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                exercise.muscleGroup == null
                    ? exercise.name
                    : '${exercise.name} · ${exercise.muscleGroup!.wire}',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              for (var s = 0; s < exercise.sets.length; s++)
                Text('Set ${s + 1}: ${_setLabel(exercise.sets[s])}'),
              if (isCurrent) const _SetEntry(),
            ],
          ),
        ),
      ),
    );
  }
}

String _setLabel(SetDraft s) {
  final parts = <String>['${s.reps} reps'];
  if (s.weightKg != null) parts.add('${_fmt(s.weightKg!)} kg');
  if (s.rpe != null) parts.add('RPE ${_fmt(s.rpe!)}');
  return parts.join(' · ');
}

String _fmt(double n) =>
    n == n.roundToDouble() ? n.toInt().toString() : n.toString();

class _SetEntry extends ConsumerStatefulWidget {
  const _SetEntry();

  @override
  ConsumerState<_SetEntry> createState() => _SetEntryState();
}

class _SetEntryState extends ConsumerState<_SetEntry> {
  final _reps = TextEditingController();
  final _weight = TextEditingController();
  final _rpe = TextEditingController();
  String? _error;

  @override
  void dispose() {
    _reps.dispose();
    _weight.dispose();
    _rpe.dispose();
    super.dispose();
  }

  void _repeat() {
    final last = ref.read(sessionDriverProvider).lastSet;
    if (last == null) return;
    _reps.text = last.reps.toString();
    _weight.text = last.weightKg == null ? '' : _fmt(last.weightKg!);
    _rpe.text = last.rpe == null ? '' : _fmt(last.rpe!);
    setState(() {});
  }

  void _log() {
    final set = SetDraft(
      reps: int.tryParse(_reps.text.trim()),
      weightKg: _weight.text.trim().isEmpty
          ? null
          : double.tryParse(_weight.text.trim()),
      rpe: _rpe.text.trim().isEmpty ? null : double.tryParse(_rpe.text.trim()),
    );
    final error = ref.read(sessionDriverProvider.notifier).logSet(set);
    setState(() => _error = error);
    if (error == null) {
      _reps.clear();
      _weight.clear();
      _rpe.clear();
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _reps,
                keyboardType: TextInputType.number,
                decoration: const InputDecoration(labelText: 'Reps'),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: TextField(
                controller: _weight,
                keyboardType:
                    const TextInputType.numberWithOptions(decimal: true),
                decoration: const InputDecoration(labelText: 'Weight (kg)'),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: TextField(
                controller: _rpe,
                keyboardType:
                    const TextInputType.numberWithOptions(decimal: true),
                decoration: const InputDecoration(labelText: 'RPE'),
              ),
            ),
          ],
        ),
        if (_error != null)
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(
              _error!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        Row(
          children: [
            TextButton(
              onPressed: _repeat,
              child: const Text('Repeat last set'),
            ),
            const Spacer(),
            FilledButton(onPressed: _log, child: const Text('Log set')),
          ],
        ),
      ],
    );
  }
}

class _FinishBar extends StatelessWidget {
  const _FinishBar({required this.state, required this.driver});

  final SessionDriverState state;
  final SessionDriver driver;

  @override
  Widget build(BuildContext context) {
    final enabled = state.canFinish && !state.submitting;
    return Padding(
      padding: const EdgeInsets.all(16),
      child: SizedBox(
        width: double.infinity,
        child: FilledButton(
          onPressed: enabled ? driver.finish : null,
          child: state.submitting
              ? const SizedBox(
                  height: 18,
                  width: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Finish'),
        ),
      ),
    );
  }
}

class _AddExerciseSheet extends StatefulWidget {
  const _AddExerciseSheet({required this.driver});

  final SessionDriver driver;

  @override
  State<_AddExerciseSheet> createState() => _AddExerciseSheetState();
}

class _AddExerciseSheetState extends State<_AddExerciseSheet> {
  final _name = TextEditingController();
  MuscleGroup? _group;
  String? _error;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  void _pickPreset(PresetExercise preset) {
    _name.text = preset.name;
    setState(() => _group = preset.group);
  }

  void _add() {
    final error = widget.driver.addExercise(_name.text, group: _group);
    if (error != null) {
      setState(() => _error = error);
      return;
    }
    Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.only(
        left: 16,
        right: 16,
        top: 16,
        bottom: MediaQuery.of(context).viewInsets.bottom + 16,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Fixed header — the confirm action stays on-screen regardless of how
          // far the preset list scrolls.
          Row(
            children: [
              Expanded(
                child: Text('New exercise',
                    style: Theme.of(context).textTheme.titleLarge),
              ),
              FilledButton(onPressed: _add, child: const Text('Add')),
            ],
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _name,
            decoration: const InputDecoration(labelText: 'Exercise name'),
            onChanged: (_) => setState(() {}),
          ),
          // Target-muscle preview for the picked/typed lift.
          Builder(builder: (context) {
            final activation = activationFor(_name.text, group: _group);
            if (activation.isEmpty) return const SizedBox.shrink();
            return Padding(
              padding: const EdgeInsets.only(top: 10),
              child: MuscleMap(activation: activation),
            );
          }),
          if (_error != null)
            Padding(
              padding: const EdgeInsets.only(top: 4),
              child: Text(
                _error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ),
          const SizedBox(height: 12),
          ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 240),
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Text('Common lifts'),
                  Wrap(
                    spacing: 8,
                    children: [
                      for (final preset in presetExercises)
                        ActionChip(
                          label: Text(preset.name),
                          onPressed: () => _pickPreset(preset),
                        ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  const Text('Muscle group (optional)'),
                  Wrap(
                    spacing: 8,
                    children: [
                      for (final group in MuscleGroup.values)
                        ChoiceChip(
                          label: Text(group.wire),
                          selected: _group == group,
                          onSelected: (_) => setState(
                              () => _group = _group == group ? null : group),
                        ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
