// R-0014 / SPEC-0014 §2.5 — Riverpod providers for the program feature.

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/user_program.dart';
import '../services/program_service.dart';

/// Fetches (and caches) the caller's current active program.
///
/// Returns `null` when the user has no program yet (404). Any other error
/// is surfaced as an [AsyncError] for the UI to handle.
final currentProgramProvider = FutureProvider<UserProgram?>((ref) {
  final service = ref.watch(programServiceProvider);
  return service.getCurrent();
});
