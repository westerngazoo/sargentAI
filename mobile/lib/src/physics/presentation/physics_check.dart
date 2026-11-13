// SPEC-0059 §2.5 — the one entry point the day region opens: the check for
// a lift the server says is modelled.

import 'package:flutter/widgets.dart';

import '../domain/lift.dart';
import 'squat_check.dart';

/// The physics check for [lift]. A lift gains a case here when its check is
/// built; the screens that open it do not change.
class PhysicsCheck extends StatelessWidget {
  const PhysicsCheck({super.key, required this.lift});

  final Lift lift;

  @override
  Widget build(BuildContext context) => switch (lift) {
        Lift.squat => const SquatCheck(),
      };
}
