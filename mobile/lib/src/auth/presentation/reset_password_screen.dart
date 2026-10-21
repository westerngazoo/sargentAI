import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/network/api_exception.dart';
import '../data/auth_repository.dart';
import 'brand_header.dart';

/// Where the emailed link lands: `/reset-password?token=<token>` (R-0060 AC7).
///
/// The token is read once, then the route is replaced without it, as a
/// history *replace*, so it leaves the address bar and the tab's Back stack.
/// `replace` keeps the page key, so this state, and the token it captured,
/// survives (SPEC-0060 §2.7).
class ResetPasswordScreen extends ConsumerStatefulWidget {
  const ResetPasswordScreen({super.key, this.token});

  final String? token;

  @override
  ConsumerState<ResetPasswordScreen> createState() =>
      _ResetPasswordScreenState();
}

class _ResetPasswordScreenState extends ConsumerState<ResetPasswordScreen> {
  final _password = TextEditingController();
  final _confirmation = TextEditingController();
  late final String? _token;
  late bool _linkInvalid;
  bool _busy = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    // Captured here, once: after the replace below, `widget.token` is null.
    _token = widget.token;
    _linkInvalid = _token?.isEmpty ?? true;
    if (!_linkInvalid) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        // `neglect` makes the browser replace the history entry instead of
        // pushing a new one, so Back does not lead to the token
        // (architect review of #153, finding 2).
        if (mounted) {
          Router.neglect(context, () => context.replace('/reset-password'));
        }
      });
    }
  }

  @override
  void dispose() {
    _password.dispose();
    _confirmation.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (_password.text != _confirmation.text) {
      setState(() => _error = "The passwords don't match.");
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await ref
          .read(authRepositoryProvider)
          .resetPassword(token: _token!, password: _password.text);
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Password updated. Log in with it.')),
      );
      context.go('/login');
    } on ApiException catch (e) {
      if (!mounted) return;
      setState(() {
        if (e.statusCode == 401) {
          _linkInvalid = true;
        } else if (e.field == 'password') {
          _error = 'Use at least 8 characters.';
        } else {
          _error = e.message;
        }
      });
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
                  const BrandHeader(compact: true),
                  const SizedBox(height: 8),
                  Text(
                    'Set a new password',
                    textAlign: TextAlign.center,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 24),
                  if (_linkInvalid)
                    ..._invalidLink(context)
                  else
                    ..._form(context),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  List<Widget> _form(BuildContext context) => [
        TextField(
          controller: _password,
          obscureText: true,
          decoration: const InputDecoration(labelText: 'New password'),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _confirmation,
          obscureText: true,
          decoration: const InputDecoration(labelText: 'Confirm new password'),
        ),
        if (_error != null) ...[
          const SizedBox(height: 12),
          Text(
            _error!,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        ],
        const SizedBox(height: 20),
        ElevatedButton(
          onPressed: _busy ? null : _submit,
          child: _busy
              ? const SizedBox(
                  height: 20,
                  width: 20,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Set new password'),
        ),
      ];

  List<Widget> _invalidLink(BuildContext context) => [
        const Text(
          'This link is no longer valid. '
          'Links expire after 30 minutes and work once.',
          textAlign: TextAlign.center,
        ),
        const SizedBox(height: 20),
        ElevatedButton(
          onPressed: () => context.go('/forgot-password'),
          child: const Text('Request a new link'),
        ),
      ];
}
