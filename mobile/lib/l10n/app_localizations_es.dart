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

  @override
  String get apiErrorTransport =>
      'No hay conexión con el servidor. Inténtalo de nuevo.';

  @override
  String get apiErrorBadRequest => 'Revisa tus datos.';

  @override
  String get apiErrorUnauthorized => 'Correo o contraseña incorrectos.';

  @override
  String get apiErrorNotFound => 'Eso ya no existe.';

  @override
  String get apiErrorConflict => 'Eso ya existe.';

  @override
  String get apiErrorUnknown => 'Algo salió mal. Inténtalo de nuevo.';

  @override
  String get regionNeck => 'Cuello';

  @override
  String get regionTraps => 'Trapecios';

  @override
  String get regionShoulders => 'Hombros';

  @override
  String get regionChest => 'Pecho';

  @override
  String get regionBiceps => 'Bíceps';

  @override
  String get regionTriceps => 'Tríceps';

  @override
  String get regionForearms => 'Antebrazos';

  @override
  String get regionCore => 'Core';

  @override
  String get regionLats => 'Dorsales';

  @override
  String get regionUpperBack => 'Espalda alta';

  @override
  String get regionLowerBack => 'Lumbares';

  @override
  String get regionQuads => 'Cuádriceps';

  @override
  String get regionHamstrings => 'Isquios';

  @override
  String get regionGlutes => 'Glúteos';

  @override
  String get regionCalves => 'Pantorrillas';

  @override
  String get muscleChest => 'Pecho';

  @override
  String get muscleBack => 'Espalda';

  @override
  String get muscleShoulders => 'Hombros';

  @override
  String get muscleArms => 'Brazos';

  @override
  String get muscleLegs => 'Piernas';

  @override
  String get muscleCore => 'Core';

  @override
  String listAnd(String head, String last) {
    return '$head y $last';
  }

  @override
  String get rejectStartWorkoutFirst => 'Primero empieza un entrenamiento.';

  @override
  String get rejectAddExerciseFirst => 'Primero agrega un ejercicio.';

  @override
  String get rejectEnterExerciseName => 'Escribe el nombre del ejercicio.';

  @override
  String rejectNameTooLong(int max) {
    return 'El nombre puede tener hasta $max caracteres.';
  }

  @override
  String get rejectEnterReps => 'Pon las repeticiones.';

  @override
  String rejectRepsOutOfRange(int min, int max) {
    return 'Las repeticiones van de $min a $max.';
  }

  @override
  String rejectWeightOutOfRange(int max) {
    return 'El peso debe ser mayor que 0 y máximo $max kg.';
  }

  @override
  String rejectRpeOutOfRange(double min, double max) {
    return 'El RPE va de $min a $max, en pasos de 0.5.';
  }

  @override
  String get fieldAreaSets => 'tus series';

  @override
  String get fieldAreaExercise => 'el ejercicio';

  @override
  String get fieldAreaWorkout => 'el entrenamiento';

  @override
  String errorCheckArea(String error, String area) {
    return '$error Revisa $area.';
  }

  @override
  String get discardThisWorkout => '¿Descartar este entrenamiento?';

  @override
  String get yourLoggedSetsWillBeLost =>
      'Se perderán las series que registraste.';

  @override
  String get cancel => 'Cancelar';

  @override
  String get discard => 'Descartar';

  @override
  String get workout => 'Entrenamiento';

  @override
  String get earbudCoachOn => 'Activar coach en audífonos';

  @override
  String get earbudCoachOff => 'Apagar coach en audífonos';

  @override
  String get voiceCoachOn => 'Activar coach de voz';

  @override
  String get voiceCoachOff => 'Apagar coach de voz';

  @override
  String get addExercise => 'Agregar ejercicio';

  @override
  String targetMusclesCaps(String name) {
    return 'MÚSCULOS OBJETIVO — $name';
  }

  @override
  String get stopListening => 'Dejar de escuchar';

  @override
  String get dictateASet => 'Dictar una serie';

  @override
  String get listening => 'Escuchando…';

  @override
  String get coachIdleHint =>
      'Toca el micrófono y di \"listo\": te pregunto repeticiones y kilos. Termina cada respuesta con \"cambio\".';

  @override
  String setLine(int n, String label) {
    return 'Serie $n: $label';
  }

  @override
  String repsCount(int n) {
    return '$n reps';
  }

  @override
  String kg(String value) {
    return '$value kg';
  }

  @override
  String rpeValue(String value) {
    return 'RPE $value';
  }

  @override
  String get repsLabel => 'Reps';

  @override
  String get rpeLabel => 'RPE';

  @override
  String get repeatLastSet => 'Repetir última serie';

  @override
  String get logSet => 'Registrar serie';

  @override
  String get newExercise => 'Nuevo ejercicio';

  @override
  String get add => 'Agregar';

  @override
  String get exerciseNameLabel => 'Nombre del ejercicio';

  @override
  String get commonLifts => 'Ejercicios comunes';

  @override
  String get muscleGroupOptional => 'Grupo muscular (opcional)';

  @override
  String exerciseWithGroup(String name, String group) {
    return '$name · $group';
  }

  @override
  String get noWorkoutsYet =>
      'Aún no hay entrenamientos. Toca Empezar para registrar el primero.';

  @override
  String sessionSummary(int exercises, int sets) {
    return '$exercises ejercicios · $sets series';
  }

  @override
  String get delete => 'Borrar';

  @override
  String get deleteThisWorkout => '¿Borrar este entrenamiento?';

  @override
  String get yesterday => 'Ayer';

  @override
  String get workoutNoLongerExists => 'Ese entrenamiento ya no existe.';

  @override
  String get startWorkout => 'Empezar entrenamiento';

  @override
  String get coachSayDoneHandsFree => 'Di listo cuando termines la serie.';

  @override
  String get coachSayDoneKeyMic =>
      'Activa el micrófono y di listo cuando termines la serie.';

  @override
  String coachTargetSuffix(String label) {
    return ' — objetivo $label';
  }

  @override
  String coachPlanLoaded(int count, String first, String prompt) {
    return 'Plan cargado: $count ejercicios. Primero: $first. $prompt';
  }

  @override
  String get coachNoPlan =>
      'Coach de voz activo. No hay plan: agrega un ejercicio y dicta tus series.';

  @override
  String coachOnCurrent(String name, String prompt) {
    return 'Coach de voz activo. Ejercicio actual: $name. $prompt';
  }

  @override
  String get coachNoCurrentExercise => 'ninguno';

  @override
  String get voiceInputUnavailable => 'Aquí no hay entrada de voz.';

  @override
  String get coachHowManyReps => '¿Cuántas repeticiones?';

  @override
  String get coachHowManyKilos => '¿Cuántos kilos? O di peso corporal.';

  @override
  String coachLoggedRestUp(String logged) {
    return 'Registrado: $logged. Descansa.';
  }

  @override
  String coachAtKilos(String weight) {
    return 'con $weight kilos';
  }

  @override
  String coachNextUp(String next) {
    return 'Sigue: $next.';
  }

  @override
  String get coachLastExercise =>
      'Ese era el último ejercicio. Di terminar entrenamiento para guardar.';

  @override
  String get coachSaving => 'Guardando tu entrenamiento.';

  @override
  String get coachSaved => 'Entrenamiento guardado. ¡Listo!';

  @override
  String get coachGoodHowManyReps => 'Bien. ¿Cuántas repeticiones?';

  @override
  String get coachStandingBy =>
      'En espera. Toca el micrófono cuando estés listo.';

  @override
  String get coachDidNotCatch =>
      'No te entendí. Di por ejemplo: diez repeticiones con sesenta kilos.';

  @override
  String get coachJustReps => 'Solo el número de repeticiones.';

  @override
  String get coachKilosAsNumber => 'Los kilos en número, o di peso corporal.';

  @override
  String coachLoggedNextSet(String logged) {
    return 'Registrado: $logged. Descansa y di listo cuando termines la siguiente serie.';
  }

  @override
  String ttsNext(String name) {
    return 'Sigue: $name.';
  }

  @override
  String ttsNextSets(String name, int sets, int reps) {
    return 'Sigue: $name. $sets series de $reps repeticiones.';
  }

  @override
  String ttsNextSetsWeight(String name, int sets, int reps, String weight) {
    return 'Sigue: $name. $sets series de $reps repeticiones con $weight kg.';
  }

  @override
  String ttsSetGo(int n) {
    return 'Serie $n. Vamos.';
  }

  @override
  String ttsSetOfGo(int n, int total, int reps) {
    return 'Serie $n de $total. $reps repeticiones. Vamos.';
  }

  @override
  String ttsSetOfWeightGo(int n, int total, int reps, String weight) {
    return 'Serie $n de $total. $reps repeticiones con $weight kg. Vamos.';
  }

  @override
  String get ttsRest => 'Descansa.';

  @override
  String get ttsWorkoutDone => 'Entrenamiento terminado.';

  @override
  String get ttsSeedFirstSet => 'Registra tu primera serie en pantalla.';

  @override
  String get sergeantVoiceUnavailable =>
      'Aquí no hay entrada de voz. Toca una opción.';

  @override
  String get sergeantGreeting =>
      'Aquí el sargento. Di: empezar entrenamiento, ver plan, registrar comida. Termina cada orden con cambio.';

  @override
  String get sergeantRogerOut => 'Recibido. Corto.';

  @override
  String get sergeantLogged => 'Registrado.';

  @override
  String get sergeantTellMeMore => 'Dime más.';

  @override
  String get sergeantRoger => 'Recibido.';

  @override
  String get sergeantDidNotCatch => 'No te entendí.';

  @override
  String get sergeantStandingBy => 'En espera.';

  @override
  String get sergeantAskMacros =>
      'Dime los gramos de proteína, carbos y grasa, una porción como 200 gramos de pechuga de pollo, o un preset, como protein shake.';

  @override
  String get sergeantStartingSession => 'Empezando tu sesión.';

  @override
  String get sergeantNoPlanYet =>
      'Aún no hay plan. Vamos a buscar tu tipo de cuerpo.';

  @override
  String get sergeantOpeningBodyMatch => 'Abriendo el tipo de cuerpo.';

  @override
  String get sergeantRecentActivity => 'Tu actividad reciente.';

  @override
  String get sergeantOpeningProfile => 'Abriendo tu perfil.';

  @override
  String get sergeantSayOptions =>
      'Di: empezar entrenamiento, ver plan o registrar comida.';

  @override
  String sergeantDidNotGet(String transcript) {
    return 'No entendí \"$transcript\". Di empezar entrenamiento, ver plan o registrar comida.';
  }

  @override
  String get sergeantMacrosExample =>
      'Gramos de proteína, carbos y grasa. Por ejemplo: 40 de proteína, 60 de carbos, 20 de grasa.';

  @override
  String get sergeantMealLogged => 'Comida registrada';

  @override
  String sergeantLoggedLabel(String label) {
    return 'Registré $label';
  }

  @override
  String sergeantMealSummary(
      String what, String protein, String carbs, String fat, int kcal) {
    return '$what: $protein de proteína, $carbs de carbos, $fat de grasa. $kcal calorías. ¿Qué más?';
  }

  @override
  String get sergeantCouldNotSaveMeal =>
      'No pude guardar la comida. Inténtalo en un momento.';

  @override
  String sergeantLookingUp(String food) {
    return 'Buscando $food.';
  }

  @override
  String sergeantCouldNotFind(String food) {
    return 'No encontré $food. Dime los gramos de proteína, carbos y grasa.';
  }

  @override
  String sergeantGramsOf(String grams, String food) {
    return '$grams gramos de $food';
  }

  @override
  String sergeantPlanSummary(String split, int days, int kcal) {
    return 'Tu plan: $split. $days días a la semana, $kcal calorías.';
  }

  @override
  String get hubWorkout => 'Entrenar';

  @override
  String get hubMeal => 'Comida';

  @override
  String get hubProgram => 'Programa';

  @override
  String get hubBodyMatch => 'Tipo de cuerpo';

  @override
  String get hubHistory => 'Historial';

  @override
  String get hubProfile => 'Perfil';

  @override
  String get hubIdleHint =>
      'Toca el micrófono y habla. Termina cada orden con \"cambio\".';

  @override
  String get progress => 'Progreso';

  @override
  String get logout => 'Salir';

  @override
  String get logMeasurement => 'Registrar medida';

  @override
  String get bodyComposition => 'Composición corporal';

  @override
  String get strength => 'Fuerza';

  @override
  String get trainingBalance => 'Balance de entrenamiento';

  @override
  String get couldNotLoadPullToRetry =>
      'No se pudo cargar. Desliza para reintentar.';

  @override
  String get logACoupleOfMeasurements =>
      'Registra un par de medidas para ver tu tendencia.';

  @override
  String get bodyFatPercentLabel => '% de grasa';

  @override
  String get leanMass => 'Masa magra';

  @override
  String get weight => 'Peso';

  @override
  String get bodyFat => 'Grasa corporal';

  @override
  String overTime(String label) {
    return '$label en el tiempo';
  }

  @override
  String get logAFewWeightedSessions =>
      'Registra unas sesiones con peso para ver subir tu fuerza.';

  @override
  String get estimatedOneRmAllLifts => '1RM estimado · todos los ejercicios';

  @override
  String estimatedOneRm(String name) {
    return '1RM estimado · $name';
  }

  @override
  String get allLifts => 'Todos';

  @override
  String gainSinceTracking(String gain) {
    return '$gain kg desde que lo registras';
  }

  @override
  String get e1rmLabel => '1RM est.';

  @override
  String get sessionVolume => 'Volumen por sesión';

  @override
  String get volume => 'Volumen';

  @override
  String get prBadge => 'PR';

  @override
  String get coachSuggestions => 'Sugerencias del coach';

  @override
  String lastWeeks(int n) {
    return 'últimas $n sem';
  }

  @override
  String get onTrack => 'Vas bien. Sigue igual.';

  @override
  String get actionCaps => 'ACCIÓN';

  @override
  String get measurementLogged => 'Medida registrada';

  @override
  String get weighInHint =>
      'Pésate. El % de grasa es opcional, pero alimenta la tendencia.';

  @override
  String get bodyFatPercentInputLabel => 'Grasa (%)';

  @override
  String get saveMeasurement => 'Guardar medida';

  @override
  String get mealLogged => 'Comida registrada';

  @override
  String get logAMeal => 'Registrar una comida';

  @override
  String get proteinGramsLabel => 'Proteína (g)';

  @override
  String get carbsGramsLabel => 'Carbos (g)';

  @override
  String get fatGramsLabel => 'Grasa (g)';

  @override
  String kcalLoggedForToday(String kcal) {
    return '≈ $kcal kcal · se registra hoy';
  }

  @override
  String get saveMeal => 'Guardar comida';

  @override
  String valueWithUnit(String value, String unit) {
    return '$value$unit';
  }

  @override
  String get workoutSessionChannel => 'Sesión de entrenamiento';
}
