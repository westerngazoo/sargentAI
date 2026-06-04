import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../auth/application/auth_controller.dart';
import '../auth/data/auth_repository.dart';
import '../core/network/api_exception.dart';

/// The authenticated home — a deliberate placeholder (AC11). It fetches the
/// current user from `GET /auth/me` (AC4) and offers Logout (AC6). A 401 from
/// `me()` (e.g. a restored-but-expired token, AC5) logs the user out; the
/// router then redirects to `/login`. R-0008+ replace the body with real
/// feature navigation.
class HomeShell extends ConsumerStatefulWidget {
  const HomeShell({super.key});

  @override
  ConsumerState<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends ConsumerState<HomeShell> {
  String? _userId;
  String? _error;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final id = await ref.read(authRepositoryProvider).me();
      if (!mounted) return;
      setState(() {
        _userId = id;
        _loading = false;
      });
    } on ApiException catch (e) {
      if (e.statusCode == 401) {
        await ref.read(authControllerProvider.notifier).logout();
        return; // the router redirect takes over.
      }
      if (!mounted) return;
      setState(() {
        _error = e.message;
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
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
      body: Center(child: _body()),
    );
  }

  Widget _body() {
    if (_loading) return const CircularProgressIndicator();
    if (_error != null) return Text(_error!);
    return Text('Signed in as $_userId');
  }
}
