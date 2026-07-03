import 'package:flutter/material.dart';

import '../auth/presentation/brand_header.dart';

/// Shown only while the session is [AuthUnknown] — i.e. during the cold-start
/// secure-storage read — so the user never sees a login flash before the
/// stored token is checked (AC3). Resolves to `/login` or `/home` as soon as
/// the restore completes.
class SplashScreen extends StatelessWidget {
  const SplashScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return const Scaffold(
      body: Center(child: BrandHeader(compact: true)),
    );
  }
}
