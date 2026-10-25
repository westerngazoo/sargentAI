import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../l10n/app_localizations.dart';

import '../../core/network/api_exception.dart';
import '../application/auth_controller.dart';
import 'auth_form.dart';
import 'brand_header.dart';

/// Register screen (AC1/AC9). Drives `AuthController.register` (which
/// auto-logs-in on success). A 400/409 renders a readable inline message and
/// leaves the user unauthenticated on this screen.
class RegisterScreen extends ConsumerStatefulWidget {
  const RegisterScreen({super.key});

  @override
  ConsumerState<RegisterScreen> createState() => _RegisterScreenState();
}

class _RegisterScreenState extends ConsumerState<RegisterScreen> {
  bool _busy = false;
  String? _error;

  Future<void> _submit(String email, String password) async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await ref.read(authControllerProvider.notifier).register(email, password);
    } on ApiException catch (e) {
      // A 409 here can only mean the email is taken. The shared copy for 409
      // is deliberately generic ('that already exists', since every endpoint
      // raises it), so the registration-specific sentence lives on this screen.
      final message = e.statusCode == 409 && mounted
          ? AppLocalizations.of(context).emailAlreadyRegistered
          : e.message;
      if (mounted) setState(() => _error = message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 420),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const BrandHeader(compact: true),
                  const SizedBox(height: 8),
                  Text(
                    l10n.createYourAccount,
                    textAlign: TextAlign.center,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 24),
                  AuthForm(
                    submitLabel: l10n.createAccount,
                    busy: _busy,
                    errorText: _error,
                    onSubmit: _submit,
                    footer: TextButton(
                      onPressed: _busy ? null : () => context.go('/login'),
                      child: Text(l10n.backToLogIn),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
