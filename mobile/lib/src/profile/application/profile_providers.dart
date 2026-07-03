import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../auth/application/auth_controller.dart';
import '../../core/network/dio_provider.dart';
import '../data/profile_api.dart';
import '../data/profile_repository.dart';
import '../domain/profile.dart';

final profileApiProvider =
    Provider<ProfileApi>((ref) => ProfileApi(ref.read(dioProvider)));

final profileRepositoryProvider = Provider<ProfileRepository>(
  (ref) => ProfileRepository(ref.read(profileApiProvider)),
);

/// The single source of truth for "does this user have a profile?" — drives the
/// home prompt (`null` → no profile → offer onboarding) and is the cold-start
/// liveness probe (a `401` here 401-sinks via the shared interceptor,
/// preserving SPEC-0007 SAC5).
final profileProvider = FutureProvider<Profile?>((ref) {
  ref.watch(authUserIdProvider); // account switch drops the cache
  return ref.read(profileRepositoryProvider).getMe();
});

/// Session-only dismissal of the prompt (not persisted — AC2).
final onboardingDismissedProvider = StateProvider<bool>((_) => false);
