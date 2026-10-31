import 'package:flutter/material.dart';

import 'coach_card.dart';
import 'log_measurement_form.dart';

/// The Medidas panel's body (SPEC-0061 §2.6): today's weigh-in form, then
/// the coach's suggestions, which read the same logs.
class MeasurementsPanel extends StatelessWidget {
  const MeasurementsPanel({super.key});

  @override
  Widget build(BuildContext context) => const Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          LogMeasurementForm(),
          SizedBox(height: 24),
          CoachCard(),
        ],
      );
}
