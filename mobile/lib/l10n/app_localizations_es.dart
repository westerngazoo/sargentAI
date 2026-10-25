// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get appTagline =>
      'La física de tu levantamiento: medida, no adivinada.';

  @override
  String get profileMenu => 'Tu perfil';

  @override
  String get menuProgram => 'Programa';

  @override
  String get menuMeasurements => 'Medidas';

  @override
  String get menuProgress => 'Progreso';

  @override
  String get signOut => 'Cerrar sesión';

  @override
  String get dictate => 'Dictar';

  @override
  String get today => 'Hoy';

  @override
  String get dayDoneHeadline => 'Entrenaste';

  @override
  String get dayFreeHeadline => 'Sesión libre';

  @override
  String get startSession => 'Empezar sesión';

  @override
  String get logLate => 'Registrar tarde';

  @override
  String get viewLogged => 'Ver lo registrado';

  @override
  String get nothingLogged => 'Nada registrado este día.';

  @override
  String get nutritionToday => 'Alimentación · hoy';

  @override
  String get addMeal => '+ Comida';
}
