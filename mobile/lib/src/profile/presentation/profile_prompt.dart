import 'package:flutter/material.dart';

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
    return Card(
      margin: const EdgeInsets.all(12),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        child: Row(
          children: [
            const Icon(Icons.account_circle_outlined),
            const SizedBox(width: 12),
            const Expanded(
              child: Text(
                'Complete your profile to personalize your training.',
              ),
            ),
            TextButton(onPressed: onStart, child: const Text('Get started')),
            IconButton(
              tooltip: 'Dismiss',
              icon: const Icon(Icons.close),
              onPressed: onDismiss,
            ),
          ],
        ),
      ),
    );
  }
}
