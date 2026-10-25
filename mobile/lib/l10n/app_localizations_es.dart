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
  String get emailLabel => 'Correo';

  @override
  String get passwordLabel => 'Contraseña';

  @override
  String get logIn => 'Entrar';

  @override
  String get createAnAccount => 'Crear una cuenta';

  @override
  String get forgotPassword => '¿Olvidaste tu contraseña?';

  @override
  String get continueWithGoogle => 'Continuar con Google';

  @override
  String get useTestAccount => 'Usar cuenta de prueba';

  @override
  String get createYourAccount => 'Crea tu cuenta';

  @override
  String get createAccount => 'Crear cuenta';

  @override
  String get backToLogIn => 'Volver';

  @override
  String get emailAlreadyRegistered =>
      'Ese correo ya tiene cuenta. Entra con él.';

  @override
  String get resetYourPassword => 'Restablece tu contraseña';

  @override
  String get sendResetLink => 'Enviar link';

  @override
  String get resetLinkSent =>
      'Si ese correo tiene cuenta, el link ya va en camino. Vence en 30 minutos.';

  @override
  String get setANewPassword => 'Pon una contraseña nueva';

  @override
  String get newPasswordLabel => 'Contraseña nueva';

  @override
  String get confirmNewPasswordLabel => 'Confirma la contraseña nueva';

  @override
  String get setNewPassword => 'Guardar contraseña';

  @override
  String get passwordsDoNotMatch => 'Las contraseñas no coinciden.';

  @override
  String get useAtLeast8Characters => 'Usa al menos 8 caracteres.';

  @override
  String get passwordUpdated => 'Contraseña actualizada. Entra con ella.';

  @override
  String get linkNoLongerValid =>
      'Este link ya no sirve. Los links vencen a los 30 minutos y solo sirven una vez.';

  @override
  String get requestANewLink => 'Pedir otro link';

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

  @override
  String get showMonth => 'Ver el mes';

  @override
  String get showWeek => 'Ver la semana';

  @override
  String get findYourProgram => 'Encuentra tu programa';
}
