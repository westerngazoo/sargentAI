import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../auth/application/auth_controller.dart';
import '../profile/application/profile_providers.dart';
import '../profile/presentation/profile_prompt.dart';
import '../program/presentation/program_detail_screen.dart';
import '../workout/application/session_driver.dart';
import '../workout/presentation/session_list.dart';

/// The authenticated home. Observes [profileProvider] for the onboarding prompt
/// and liveness probe (401 → logout). [CurrentProgramCard] is always present
/// and manages its own async state independently of the profile load.
///
/// `profileProvider`'s read is the cold-start liveness probe: a restored but
/// expired token yields a `401`, which the shared `AuthInterceptor` 401-sinks
/// → logout → `/login` (SPEC-0007 SAC5). R-0009+ must not reintroduce a
/// redundant `/auth/me` read.
class HomeShell extends ConsumerWidget {
  const HomeShell({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final profile = ref.watch(profileProvider);
    final dismissed = ref.watch(onboardingDismissedProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('fitAI'),
        actions: [
          IconButton(
            icon: const Icon(Icons.mic),
            tooltip: 'Voice hub',
            onPressed: () => context.go('/hub'),
          ),
          TextButton(
            onPressed: () => ref.read(authControllerProvider.notifier).logout(),
            child: const Text('Logout'),
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () {
          ref.read(sessionDriverProvider.notifier).start();
          context.go('/session');
        },
        icon: const Icon(Icons.fitness_center),
        label: const Text('Start workout'),
      ),
      body: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Profile-dependent onboarding prompt — hidden once dismissed or
          // once the profile loads.
          profile.when(
            loading: () => const SizedBox.shrink(),
            error: (_, __) => const SizedBox.shrink(),
            data: (p) => (p == null && !dismissed)
                ? ProfilePrompt(
                    onStart: () => context.go('/onboarding'),
                    onDismiss: () => ref
                        .read(onboardingDismissedProvider.notifier)
                        .state = true,
                  )
                : const SizedBox.shrink(),
          ),
          // Program shortcut — always visible; manages its own async state.
          const CurrentProgramCard(),
          const Expanded(child: SessionList()),
        ],
      ),
    );
  }
}
