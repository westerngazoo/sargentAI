import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../application/onboarding_controller.dart';
import 'steps/body_stats_step.dart';
import 'steps/goals_step.dart';
import 'steps/optional_details_step.dart';

/// The multi-step onboarding wizard (AC3). It holds no `try/catch`: it
/// `ref.listen`s the controller and reacts — navigating to `/home` on `done`
/// (the screen is the SOLE owner of the `go`, AC7) and rendering `state.error`
/// inline otherwise (AC8). The controller jumps `step` to a 400's offending
/// field, so this screen only renders the current step.
class OnboardingScreen extends ConsumerWidget {
  const OnboardingScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref.listen<OnboardingState>(onboardingControllerProvider, (prev, next) {
      if (next.done && !(prev?.done ?? false)) {
        context.go('/home');
      }
    });

    final state = ref.watch(onboardingControllerProvider);
    final ctrl = ref.read(onboardingControllerProvider.notifier);

    return Scaffold(
      appBar: AppBar(title: const Text('Set up your profile')),
      body: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            LinearProgressIndicator(
              value: (state.step + 1) / onboardingStepCount,
            ),
            if (state.error != null)
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
                child: Text(
                  state.error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
            Expanded(
              child: SingleChildScrollView(
                padding: const EdgeInsets.all(16),
                child: switch (state.step) {
                  0 => const BodyStatsStep(),
                  1 => const GoalsStep(),
                  _ => const OptionalDetailsStep(),
                },
              ),
            ),
            _Controls(state: state, ctrl: ctrl),
          ],
        ),
      ),
    );
  }
}

class _Controls extends StatelessWidget {
  const _Controls({required this.state, required this.ctrl});

  final OnboardingState state;
  final OnboardingController ctrl;

  @override
  Widget build(BuildContext context) {
    final isLast = state.step == onboardingStepCount - 1;
    final canAdvance = switch (state.step) {
      0 => state.draft.bodyStatsValidOn(DateTime.now()),
      1 => state.draft.goalsValid,
      _ => true,
    };

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Row(
        children: [
          if (state.step > 0)
            TextButton(
              onPressed: state.submitting ? null : ctrl.back,
              child: const Text('Back'),
            ),
          const Spacer(),
          if (!isLast)
            FilledButton(
              onPressed: canAdvance ? ctrl.next : null,
              child: const Text('Next'),
            )
          else
            FilledButton(
              onPressed: state.submitting ? null : ctrl.submit,
              child: state.submitting
                  ? const SizedBox(
                      height: 18,
                      width: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Text('Finish'),
            ),
        ],
      ),
    );
  }
}
