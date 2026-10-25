import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../l10n/app_localizations.dart';
import '../../core/network/api_error_text.dart';
import '../../core/network/api_exception.dart';
import '../application/earbud_coach.dart';
import '../application/session_driver.dart';
import '../application/voice_coach.dart';
import '../domain/draft_rejection.dart';
import '../domain/exercise_draft.dart';
import '../domain/muscle_activation.dart';
import '../domain/muscle_group.dart';
import '../domain/preset_exercises.dart';
import '../domain/set_draft.dart';
import '../application/draft_rejection_text.dart';
import 'muscle_map.dart';
import '../application/muscle_names.dart';

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
    final l10n = AppLocalizations.of(context);

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
            title: Text(l10n.discardThisWorkout),
            content: Text(l10n.yourLoggedSetsWillBeLost),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(context).pop(false),
                child: Text(l10n.cancel),
              ),
              TextButton(
                onPressed: () => Navigator.of(context).pop(true),
                child: Text(l10n.discard),
              ),
            ],
          ),
        );
        if (discard ?? false) driver.abandon();
      },
      child: Scaffold(
        appBar: AppBar(
          title: Text(l10n.workout),
          actions: [
            Consumer(builder: (context, ref, _) {
              // Watch the coach provider so that it is instantiated and kept alive
              ref.watch(earbudCoachProvider);
              final earbudOn = ref.watch(earbudModeProvider);
              return IconButton(
                tooltip: earbudOn ? l10n.earbudCoachOff : l10n.earbudCoachOn,
                isSelected: earbudOn,
                icon: const Icon(Icons.headphones_outlined),
                selectedIcon: const Icon(Icons.headphones),
                onPressed: () {
                  ref.read(earbudModeProvider.notifier).state = !earbudOn;
                },
              );
            }),
            IconButton(
              tooltip: coach.enabled ? l10n.voiceCoachOff : l10n.voiceCoachOn,
              isSelected: coach.enabled,
              icon: const Icon(Icons.headset_off_outlined),
              selectedIcon: const Icon(Icons.headset_mic),
              onPressed: () {
                final notifier = ref.read(voiceCoachProvider.notifier);
                if (coach.enabled) {
                  notifier.disable();
                } else {
                  // Mutual exclusivity
                  ref.read(earbudModeProvider.notifier).state = false;
                  notifier.enable();
                }
              },
            ),
            const SizedBox(width: 8),
          ],
        ),
        body: SafeArea(
          child: Column(
            children: [
              if (state.error != null) _ErrorBanner(error: state.error!),
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
                      label: Text(l10n.addExercise),
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
    final l10n = AppLocalizations.of(context);
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              l10n.targetMusclesCaps(exercise.name.toUpperCase()),
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
    final l10n = AppLocalizations.of(context);
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
            tooltip: coach.listening ? l10n.stopListening : l10n.dictateASet,
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
                      ? l10n.listening
                      : '“${coach.transcript}”')
                  : (coach.coachLine.isEmpty
                      ? l10n.coachIdleHint
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
/// area of the screen to check (`_fieldArea`), so the message is field-aware —
/// the context the R-0027 voice transport will speak back.
class _ErrorBanner extends StatelessWidget {
  const _ErrorBanner({required this.error});

  final ApiException error;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final text = apiErrorText(error, l10n);
    final area = _fieldArea(error.field, l10n);
    final message = area == null ? text : l10n.errorCheckArea(text, area);
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

/// Maps a backend `400 {field}` to the area of the live screen that owns it;
/// `null` when the field maps to no specific area.
String? _fieldArea(String? field, AppLocalizations l10n) => switch (field) {
      'reps' || 'weight_kg' || 'rpe' => l10n.fieldAreaSets,
      'name' || 'muscle_group' => l10n.fieldAreaExercise,
      'exercises' || 'sets' || 'performed_on' => l10n.fieldAreaWorkout,
      _ => null,
    };

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
    final l10n = AppLocalizations.of(context);
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
                    : l10n.exerciseWithGroup(
                        exercise.name,
                        muscleGroupName(exercise.muscleGroup!, l10n),
                      ),
                style: Theme.of(context).textTheme.titleMedium,
              ),
              for (var s = 0; s < exercise.sets.length; s++)
                Text(l10n.setLine(s + 1, _setLabel(exercise.sets[s], l10n))),
              if (isCurrent) const _SetEntry(),
            ],
          ),
        ),
      ),
    );
  }
}

String _setLabel(SetDraft s, AppLocalizations l10n) {
  final parts = <String>[l10n.repsCount(s.reps!)];
  if (s.weightKg != null) parts.add(l10n.kg(_fmt(s.weightKg!)));
  if (s.rpe != null) parts.add(l10n.rpeValue(_fmt(s.rpe!)));
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
  DraftRejection? _error;

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
    final l10n = AppLocalizations.of(context);
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
                decoration: InputDecoration(labelText: l10n.repsLabel),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: TextField(
                controller: _weight,
                keyboardType:
                    const TextInputType.numberWithOptions(decimal: true),
                decoration: InputDecoration(labelText: l10n.weightKgLabel),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: TextField(
                controller: _rpe,
                keyboardType:
                    const TextInputType.numberWithOptions(decimal: true),
                decoration: InputDecoration(labelText: l10n.rpeLabel),
              ),
            ),
          ],
        ),
        if (_error != null)
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(
              draftRejectionText(_error!, l10n),
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        Row(
          children: [
            TextButton(
              onPressed: _repeat,
              child: Text(l10n.repeatLastSet),
            ),
            const Spacer(),
            FilledButton(onPressed: _log, child: Text(l10n.logSet)),
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
              : Text(AppLocalizations.of(context).finish),
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
  DraftRejection? _error;

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
    final l10n = AppLocalizations.of(context);
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
                child: Text(l10n.newExercise,
                    style: Theme.of(context).textTheme.titleLarge),
              ),
              FilledButton(onPressed: _add, child: Text(l10n.add)),
            ],
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _name,
            decoration: InputDecoration(labelText: l10n.exerciseNameLabel),
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
                draftRejectionText(_error!, l10n),
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
                  Text(l10n.commonLifts),
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
                  Text(l10n.muscleGroupOptional),
                  Wrap(
                    spacing: 8,
                    children: [
                      for (final group in MuscleGroup.values)
                        ChoiceChip(
                          label: Text(muscleGroupName(group, l10n)),
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
