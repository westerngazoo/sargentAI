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
  String get setUpYourProfile => 'Configura tu perfil';

  @override
  String get onboardingMissingBodyStats =>
      'Pon tu fecha de nacimiento, estatura y peso para seguir.';

  @override
  String get onboardingMissingGoals =>
      'Elige al menos un objetivo para seguir.';

  @override
  String get back => 'Atrás';

  @override
  String get next => 'Siguiente';

  @override
  String get finish => 'Terminar';

  @override
  String get completeYourProfilePrompt =>
      'Completa tu perfil para personalizar tu entrenamiento.';

  @override
  String get getStarted => 'Empezar';

  @override
  String get dismiss => 'Cerrar';

  @override
  String get aboutYou => 'Sobre ti';

  @override
  String get dateOfBirthLabel => 'Fecha de nacimiento';

  @override
  String get notSet => 'Sin elegir';

  @override
  String get choose => 'Elegir';

  @override
  String get heightCmLabel => 'Estatura (cm)';

  @override
  String get weightKgLabel => 'Peso (kg)';

  @override
  String get dobMissing => 'Elige tu fecha de nacimiento.';

  @override
  String get dobInFuture =>
      'La fecha de nacimiento no puede estar en el futuro.';

  @override
  String ageOutOfRange(int min, int max) {
    return 'la edad debe estar entre $min y $max años';
  }

  @override
  String get heightMissing => 'pon tu estatura';

  @override
  String heightOutOfRange(int min, int max) {
    return 'la estatura debe estar entre $min y $max cm';
  }

  @override
  String get weightMissing => 'pon tu peso';

  @override
  String weightOutOfRange(int min, int max) {
    return 'el peso debe estar entre $min y $max kg';
  }

  @override
  String bodyFatOutOfRange(int min, int max) {
    return 'la grasa corporal debe estar entre $min y $max %';
  }

  @override
  String get yourGoals => 'Tus objetivos';

  @override
  String get pickAtLeastOne => 'Elige al menos uno.';

  @override
  String get goalLoseFat => 'Bajar grasa';

  @override
  String get goalBuildMuscle => 'Ganar músculo';

  @override
  String get goalRecomp => 'Recomposición';

  @override
  String get goalMaintain => 'Mantener';

  @override
  String get goalGainStrength => 'Ganar fuerza';

  @override
  String get optionalDetails => 'Datos opcionales';

  @override
  String get youCanSkipThese => 'Estos te los puedes saltar.';

  @override
  String get sexMale => 'Hombre';

  @override
  String get sexFemale => 'Mujer';

  @override
  String get clear => 'Quitar';

  @override
  String get bodyFatOptionalLabel => '% de grasa corporal (opcional)';

  @override
  String get chooseYourBodyType => 'Elige tu tipo de cuerpo';

  @override
  String get bodyShape => 'Complexión';

  @override
  String get bodyFatLevel => 'Nivel de grasa corporal';

  @override
  String get findMyProgram => 'Buscar mi programa';

  @override
  String get bodyShapeEctomorph => 'Delgado y angosto';

  @override
  String get bodyShapeMesomorph => 'Atlético y ancho';

  @override
  String get bodyShapeEndomorph => 'Robusto y macizo';

  @override
  String get bodyShapeEctomorphDescription =>
      'Cuerpo delgado, hombros angostos, extremidades largas, poca masa.';

  @override
  String get bodyShapeMesomorphDescription =>
      'Hombros anchos, cuerpo musculoso, responde bien al entrenamiento.';

  @override
  String get bodyShapeEndomorphDescription =>
      'Cuerpo robusto, cintura ancha, tiende a cargar más masa.';

  @override
  String get fatBandLean => 'Definido';

  @override
  String get fatBandModerate => 'Medio';

  @override
  String get fatBandBulky => 'Con más grasa';

  @override
  String get fatBandLeanDescription => 'Músculo visible y definido';

  @override
  String get fatBandModerateDescription => 'Algo de definición, grasa moderada';

  @override
  String get fatBandBulkyDescription => 'Más grasa corporal, aspecto lleno';

  @override
  String get yourProgram => 'Tu programa';

  @override
  String get couldNotLoadYourProgram => 'No se pudo cargar tu programa';

  @override
  String get retry => 'Reintentar';

  @override
  String get noProgramYet => 'Todavía no tienes programa';

  @override
  String get training => 'Entrenamiento';

  @override
  String get split => 'Split';

  @override
  String daysPerWeek(int n) {
    return '$n días/semana';
  }

  @override
  String get sessionDuration => 'Duración de la sesión';

  @override
  String minutesShort(int n) {
    return '$n min';
  }

  @override
  String get intensity => 'Intensidad';

  @override
  String get rest => 'Descanso';

  @override
  String get progression => 'Progresión';

  @override
  String get highlightExercises => 'Ejercicios clave';

  @override
  String get nutrition => 'Alimentación';

  @override
  String get approach => 'Enfoque';

  @override
  String get strategy => 'Estrategia';

  @override
  String get meals => 'Comidas';

  @override
  String get protein => 'Proteína';

  @override
  String get carbs => 'Carbos';

  @override
  String get fat => 'Grasa';

  @override
  String grams(int n) {
    return '$n g';
  }

  @override
  String get kcalPerDay => 'kcal / día';

  @override
  String get yourProgramCaps => 'TU PROGRAMA';

  @override
  String kcalShort(int n) {
    return '$n kcal';
  }

  @override
  String doneOfTarget(int done, int target) {
    return '$done/$target';
  }

  @override
  String get getYourProgram => 'Consigue tu programa';

  @override
  String get takeAPhotoForPlan =>
      'Tómate una foto y recibe un plan a tu medida';

  @override
  String get targetPhysiqueCaps => 'FÍSICO OBJETIVO';

  @override
  String get physiqueClassicAesthetic => 'Estética clásica';

  @override
  String get physiqueModernHypertrophy => 'Hipertrofia moderna';

  @override
  String get physiqueLeanMinimalist => 'Minimalista definido';

  @override
  String get physiquePowerbuilder => 'Powerbuilder';

  @override
  String get physiqueHeavyDutyMass => 'Masa heavy-duty';

  @override
  String get physiqueMassMonster => 'Mass monster';

  @override
  String get physiqueClassicAestheticBlurb =>
      'Hombros anchos, cintura chica: la X de la era dorada.';

  @override
  String get physiqueModernHypertrophyBlurb =>
      'Equilibrado, detallado, atlético: proporción antes que puro tamaño.';

  @override
  String get physiqueLeanMinimalistBlurb =>
      'Seco y duro: músculo denso, sin exceso.';

  @override
  String get physiquePowerbuilderBlurb =>
      'Grueso y potente: mezcla de strongman y fisicoculturista.';

  @override
  String get physiqueHeavyDutyMassBlurb =>
      'Masa densa heavy-duty: máximo músculo, cuadrado y fuerte.';

  @override
  String get physiqueMassMonsterBlurb =>
      'Tamaño máximo: la silueta del mass monster.';

  @override
  String get physiqueDefaultBlurb => 'El físico al que apunta tu programa.';

  @override
  String taperTag(String ratio) {
    return 'Taper $ratio:1  ⓘ';
  }

  @override
  String get physiqueMass => 'Masa';

  @override
  String get physiqueLean => 'Definido';

  @override
  String get physiqueBalanced => 'Equilibrado';

  @override
  String get taperByBuild => 'Taper por físico';

  @override
  String get taperTableExplainer =>
      'Relación hombros-cintura de la silueta objetivo. Un número mayor es una V más marcada. Compara estos físicos entre sí; no es una medida de tu cuerpo.';

  @override
  String taperRatio(String ratio) {
    return '$ratio:1';
  }

  @override
  String get thisWeekCaps => 'ESTA SEMANA';

  @override
  String daysOfTarget(int done, int target) {
    return '$done de $target días';
  }

  @override
  String sessionsLoggedOnProgram(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count sesiones registradas en este programa',
      one: '1 sesión registrada en este programa',
    );
    return '$_temp0';
  }

  @override
  String get targetMet => 'Meta cumplida';

  @override
  String percent(int n) {
    return '$n%';
  }

  @override
  String get chooseYourProgram => 'Elige tu programa';

  @override
  String get couldNotLoadProposals => 'No se pudieron cargar las propuestas';

  @override
  String get rankBestMatch => 'Mejor opción';

  @override
  String get rankCloseMatch => 'Segunda opción';

  @override
  String get rankGoodOption => 'Buena opción';

  @override
  String proposalSummaryLine(int days, int kcal) {
    return '$days días/semana · $kcal kcal/día';
  }

  @override
  String intensityLine(String value) {
    return 'Intensidad: $value';
  }

  @override
  String restLine(String value) {
    return 'Descanso: $value';
  }

  @override
  String progressionLine(String value) {
    return 'Progresión: $value';
  }

  @override
  String get chooseThisProgram => 'Elegir este programa';

  @override
  String get selectionNoLongerAvailable =>
      'Esa opción ya no está disponible. Vuelve a cargar.';

  @override
  String get couldNotChooseProgram =>
      'No se pudo elegir el programa. Inténtalo de nuevo.';
}
