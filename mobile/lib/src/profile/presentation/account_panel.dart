import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../l10n/app_localizations.dart';
import '../../auth/application/auth_controller.dart';
import '../application/profile_providers.dart';

/// The Cuenta panel's body (SPEC-0061 §2.6): what the app knows of the
/// account — the profile's height and weight — then *Editar perfil*, which
/// opens the onboarding wizard as the hub's Profile did, and sign-out, moved
/// here from the profile menu. Without a profile it offers to set one up.
///
/// No name or email: no endpoint returns them (`GET /auth/me` carries the id
/// only), so they wait on the backend.
class AccountPanel extends ConsumerWidget {
  const AccountPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final profile = ref.watch(profileProvider).valueOrNull;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (profile != null) ...[
          Text(
            l10n.accountProfileSummary(profile.heightCm, profile.weightKg),
            style: Theme.of(context).textTheme.bodyLarge,
          ),
          const SizedBox(height: 16),
        ],
        FilledButton.icon(
          key: const Key('edit-profile'),
          icon: const Icon(Icons.edit_outlined),
          label:
              Text(profile == null ? l10n.setUpYourProfile : l10n.editProfile),
          onPressed: () => _editProfile(context),
        ),
        const SizedBox(height: 12),
        OutlinedButton.icon(
          key: const Key('sign-out'),
          icon: const Icon(Icons.logout),
          label: Text(l10n.signOut),
          onPressed: () => ref.read(authControllerProvider.notifier).logout(),
        ),
      ],
    );
  }

  /// Closes the panel first, so back from the wizard lands on the day.
  void _editProfile(BuildContext context) {
    final router = GoRouter.of(context);
    Navigator.of(context).pop();
    router.push('/onboarding');
  }
}
