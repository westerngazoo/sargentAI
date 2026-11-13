// SPEC-0059 §2.3 — the squat check's inputs, the server's answer, and the
// playback between the frames it returned (display only, R-0059 AC7).

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/network/api_exception.dart';
import '../../core/network/dio_provider.dart';
import '../../profile/application/profile_providers.dart';
import '../data/squat_check_api.dart';
import '../domain/playback.dart';
import '../domain/squat_range.dart';

/// The [SquatCheckApi] over the app's authenticated [dioProvider].
final squatCheckApiProvider =
    Provider<SquatCheckApi>((ref) => SquatCheckApi(ref.read(dioProvider)));

/// The lifter's ankle dorsiflexion, degrees. No default (SPEC-0045
/// §2.6.1.1): null until the lifter types it, then kept for the session
/// only, until R-0058 stores it.
final squatDorsiflexionProvider = StateProvider<double?>((_) => null);

/// The bar load chosen for the check, kg; null is bodyweight only.
final squatLoadKgProvider = StateProvider<double?>((_) => null);

/// The squat check's result for the current inputs: null until the
/// dorsiflexion and the profile are known, an error carrying the
/// [ApiException] when the server refuses the request.
final squatCheckControllerProvider =
    AsyncNotifierProvider<SquatCheckController, SquatRangeResult?>(
  SquatCheckController.new,
);

/// Asks the server for the range once the inputs are known: height and mass
/// from the profile (`height_cm / 100`), limb lengths null until R-0058 (the
/// table fills them), [squatDorsiflexionProvider] and [squatLoadKgProvider].
///
/// The state is null while the dorsiflexion or the profile is missing, and an
/// error carrying the [ApiException] when the server refuses the request.
class SquatCheckController extends AsyncNotifier<SquatRangeResult?> {
  @override
  Future<SquatRangeResult?> build() async {
    final dorsiflexion = ref.watch(squatDorsiflexionProvider);
    final barKg = ref.watch(squatLoadKgProvider);
    final profile = await ref.watch(profileProvider.future);
    if (dorsiflexion == null || profile == null) return null;
    return ref.read(squatCheckApiProvider).range(SquatRangeRequest(
          heightM: profile.heightCm / 100,
          massKg: profile.weightKg,
          dorsiflexionDeg: dorsiflexion,
          barKg: barKg,
        ));
  }

  /// The playback frame at [progress] over the current result's frames
  /// ([playbackAt]); null with no result or no frames.
  PlaybackFrame? frameAt(double progress) =>
      playbackAt(state.valueOrNull?.frames ?? const [], progress);
}
