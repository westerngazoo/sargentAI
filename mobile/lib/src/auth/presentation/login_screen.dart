import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/dev_login.dart';
import '../../core/google_sign_in_config.dart';
import '../../core/network/api_exception.dart';
import '../application/auth_controller.dart';
import '../data/google_sign_in_seam.dart';
import 'auth_form.dart';
import 'brand_header.dart';

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

  Future<void> _googleSignIn() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final idToken = await ref.read(googleSignInSeamProvider).signInForIdToken();
      if (idToken == null) return;
      await ref.read(authControllerProvider.notifier).loginWithGoogle(idToken);
    } on ApiException catch (e) {
      if (mounted) setState(() => _error = e.message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
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
                  const BrandHeader(),
                  const SizedBox(height: 32),
                  AuthForm(
                    submitLabel: 'Log in',
                    busy: _busy,
                    errorText: _error,
                    onSubmit: _submit,
                    footer: TextButton(
                      onPressed: _busy ? null : () => context.go('/register'),
                      child: const Text('Create an account'),
                    ),
                  ),
                  if (GoogleSignInConfig.enabled) ...[
                    const SizedBox(height: 12),
                    OutlinedButton.icon(
                      icon: const Icon(Icons.login),
                      label: const Text('Continue with Google'),
                      onPressed: _busy ? null : _googleSignIn,
                    ),
                  ],
                  if (DevLogin.enabled) ...[
                    const SizedBox(height: 4),
                    OutlinedButton.icon(
                      icon: const Icon(Icons.bug_report_outlined, size: 18),
                      label: const Text('Use test account'),
                      onPressed: _busy
                          ? null
                          : () => _submit(DevLogin.email, DevLogin.password),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
