// Shared test doubles for the measurements feature (R-0034, R-0017): the
// typed client the Medidas form and the Progreso panel read through.

import 'package:fitai/src/measurements/models/measurement.dart';
import 'package:fitai/src/measurements/services/measurement_service.dart';
import 'package:mocktail/mocktail.dart';

/// Mock [MeasurementService] — stubs list() / create().
class MockMeasurementService extends Mock implements MeasurementService {}

/// A parsed measurement, as `POST /measurements` returns it.
Measurement sampleMeasurement({
  DateTime? measuredOn,
  double weightKg = 80,
  double? bodyFatPercentage,
}) =>
    Measurement(
      measuredOn: measuredOn ?? DateTime(2026, 10, 25),
      weightKg: weightKg,
      bodyFatPercentage: bodyFatPercentage,
    );
