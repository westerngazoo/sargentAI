import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../auth/application/auth_controller.dart';
import '../profile/application/profile_providers.dart';
import '../profile/presentation/profile_prompt.dart';
import '../workout/application/session_driver.dart';
import '../workout/presentation/session_list.dart';

/// The authenticated home (AC11 placeholder). It observes [profileProvider]
/// (`GET /profile/me`) via `AsyncValue.when` — the uniform async idiom (SPEC-0007
/// review Finding 1, paid down here): a `null` profile (404) offers the
/// dismissible onboarding prompt; a present profile shows a summary.
///
/// `profileProvider`'s read is also the cold-start liveness probe: a restored
/// but expired token yields a `401`, which the shared `AuthInterceptor`
/// 401-sinks → logout → `/login` (SPEC-0007 SAC5). R-0009+ must not reintroduce
/// a redundant `/auth/me` read.
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
      body: profile.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (_, __) =>
            _Retry(onRetry: () => ref.invalidate(profileProvider)),
        data: (p) => Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (p == null && !dismissed)
              ProfilePrompt(
                onStart: () => context.go('/onboarding'),
                onDismiss: () =>
                    ref.read(onboardingDismissedProvider.notifier).state = true,
              ),
            const Expanded(child: SessionList()),
          ],
        ),
      ),
    );
  }
}

class _Retry extends StatelessWidget {
  const _Retry({required this.onRetry});

  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text("couldn't load your profile"),
            TextButton(onPressed: onRetry, child: const Text('Retry')),
          ],
        ),
      );
}
