import 'package:flutter/material.dart';

import '../../../l10n/app_localizations.dart';

/// The dismissible "complete your profile" banner shown on the home shell when
/// the user has no profile yet (AC1). Stateless — the shell owns visibility and
/// the [onDismiss]/[onStart] effects (AC2).
class ProfilePrompt extends StatelessWidget {
  const ProfilePrompt({
    super.key,
    required this.onStart,
    required this.onDismiss,
  });

  final VoidCallback onStart;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Card(
      margin: const EdgeInsets.all(12),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        child: Row(
          children: [
            const Icon(Icons.account_circle_outlined),
            const SizedBox(width: 12),
            Expanded(child: Text(l10n.completeYourProfilePrompt)),
            TextButton(onPressed: onStart, child: Text(l10n.getStarted)),
            IconButton(
              tooltip: l10n.dismiss,
              icon: const Icon(Icons.close),
              onPressed: onDismiss,
            ),
          ],
        ),
      ),
    );
  }
}
