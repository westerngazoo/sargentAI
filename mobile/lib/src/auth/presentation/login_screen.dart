import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/network/api_exception.dart';
import '../application/auth_controller.dart';
import 'auth_form.dart';

/// Login screen (AC2/AC9). Drives `AuthController.login`; on success the
/// session state change drives the router redirect to `/home` — there is no
/// imperative navigation here.
class LoginScreen extends ConsumerStatefulWidget {
  const LoginScreen({super.key});

  @override
  ConsumerState<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends ConsumerState<LoginScreen> {
  bool _busy = false;
  String? _error;

  Future<void> _submit(String email, String password) async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await ref.read(authControllerProvider.notifier).login(email, password);
    } on ApiException catch (e) {
      if (mounted) setState(() => _error = e.message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Log in')),
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: AuthForm(
          submitLabel: 'Log in',
          busy: _busy,
          errorText: _error,
          onSubmit: _submit,
          footer: TextButton(
            onPressed: _busy ? null : () => context.go('/register'),
            child: const Text('Create an account'),
          ),
        ),
      ),
    );
  }
}
