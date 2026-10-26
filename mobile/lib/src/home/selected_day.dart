import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/dates.dart';

/// The day the main screen shows (SPEC-0061 §2.5). Date only; defaults to
/// today. Panels never replace the route, so it survives them (R-0061 AC1).
final selectedDayProvider =
    StateProvider<DateTime>((_) => dateOnly(DateTime.now()));
