import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_es.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
      : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
    delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
  ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('es')
  ];

  /// Brand tagline under the badge on the screens before sign-in.
  ///
  /// In es, this message translates to:
  /// **'La física de tu levantamiento: medida, no adivinada.'**
  String get appTagline;

  /// No description provided for @emailLabel.
  ///
  /// In es, this message translates to:
  /// **'Correo'**
  String get emailLabel;

  /// No description provided for @passwordLabel.
  ///
  /// In es, this message translates to:
  /// **'Contraseña'**
  String get passwordLabel;

  /// No description provided for @logIn.
  ///
  /// In es, this message translates to:
  /// **'Entrar'**
  String get logIn;

  /// No description provided for @createAnAccount.
  ///
  /// In es, this message translates to:
  /// **'Crear una cuenta'**
  String get createAnAccount;

  /// No description provided for @forgotPassword.
  ///
  /// In es, this message translates to:
  /// **'¿Olvidaste tu contraseña?'**
  String get forgotPassword;

  /// No description provided for @continueWithGoogle.
  ///
  /// In es, this message translates to:
  /// **'Continuar con Google'**
  String get continueWithGoogle;

  /// No description provided for @useTestAccount.
  ///
  /// In es, this message translates to:
  /// **'Usar cuenta de prueba'**
  String get useTestAccount;

  /// No description provided for @createYourAccount.
  ///
  /// In es, this message translates to:
  /// **'Crea tu cuenta'**
  String get createYourAccount;

  /// No description provided for @createAccount.
  ///
  /// In es, this message translates to:
  /// **'Crear cuenta'**
  String get createAccount;

  /// No description provided for @backToLogIn.
  ///
  /// In es, this message translates to:
  /// **'Volver'**
  String get backToLogIn;

  /// No description provided for @emailAlreadyRegistered.
  ///
  /// In es, this message translates to:
  /// **'Ese correo ya tiene cuenta. Entra con él.'**
  String get emailAlreadyRegistered;

  /// No description provided for @resetYourPassword.
  ///
  /// In es, this message translates to:
  /// **'Restablece tu contraseña'**
  String get resetYourPassword;

  /// No description provided for @sendResetLink.
  ///
  /// In es, this message translates to:
  /// **'Enviar link'**
  String get sendResetLink;

  /// No description provided for @resetLinkSent.
  ///
  /// In es, this message translates to:
  /// **'Si ese correo tiene cuenta, el link ya va en camino. Vence en 30 minutos.'**
  String get resetLinkSent;

  /// No description provided for @setANewPassword.
  ///
  /// In es, this message translates to:
  /// **'Pon una contraseña nueva'**
  String get setANewPassword;

  /// No description provided for @newPasswordLabel.
  ///
  /// In es, this message translates to:
  /// **'Contraseña nueva'**
  String get newPasswordLabel;

  /// No description provided for @confirmNewPasswordLabel.
  ///
  /// In es, this message translates to:
  /// **'Confirma la contraseña nueva'**
  String get confirmNewPasswordLabel;

  /// No description provided for @setNewPassword.
  ///
  /// In es, this message translates to:
  /// **'Guardar contraseña'**
  String get setNewPassword;

  /// No description provided for @passwordsDoNotMatch.
  ///
  /// In es, this message translates to:
  /// **'Las contraseñas no coinciden.'**
  String get passwordsDoNotMatch;

  /// No description provided for @useAtLeast8Characters.
  ///
  /// In es, this message translates to:
  /// **'Usa al menos 8 caracteres.'**
  String get useAtLeast8Characters;

  /// No description provided for @passwordUpdated.
  ///
  /// In es, this message translates to:
  /// **'Contraseña actualizada. Entra con ella.'**
  String get passwordUpdated;

  /// No description provided for @linkNoLongerValid.
  ///
  /// In es, this message translates to:
  /// **'Este link ya no sirve. Los links vencen a los 30 minutos y solo sirven una vez.'**
  String get linkNoLongerValid;

  /// No description provided for @requestANewLink.
  ///
  /// In es, this message translates to:
  /// **'Pedir otro link'**
  String get requestANewLink;

  /// No description provided for @setUpYourProfile.
  ///
  /// In es, this message translates to:
  /// **'Configura tu perfil'**
  String get setUpYourProfile;

  /// No description provided for @onboardingMissingBodyStats.
  ///
  /// In es, this message translates to:
  /// **'Pon tu fecha de nacimiento, estatura y peso para seguir.'**
  String get onboardingMissingBodyStats;

  /// No description provided for @onboardingMissingGoals.
  ///
  /// In es, this message translates to:
  /// **'Elige al menos un objetivo para seguir.'**
  String get onboardingMissingGoals;

  /// No description provided for @back.
  ///
  /// In es, this message translates to:
  /// **'Atrás'**
  String get back;

  /// No description provided for @next.
  ///
  /// In es, this message translates to:
  /// **'Siguiente'**
  String get next;

  /// No description provided for @finish.
  ///
  /// In es, this message translates to:
  /// **'Terminar'**
  String get finish;

  /// No description provided for @completeYourProfilePrompt.
  ///
  /// In es, this message translates to:
  /// **'Completa tu perfil para personalizar tu entrenamiento.'**
  String get completeYourProfilePrompt;

  /// No description provided for @getStarted.
  ///
  /// In es, this message translates to:
  /// **'Empezar'**
  String get getStarted;

  /// Tooltip of the X that hides the complete-your-profile banner.
  ///
  /// In es, this message translates to:
  /// **'Cerrar'**
  String get dismiss;

  /// No description provided for @aboutYou.
  ///
  /// In es, this message translates to:
  /// **'Sobre ti'**
  String get aboutYou;

  /// No description provided for @dateOfBirthLabel.
  ///
  /// In es, this message translates to:
  /// **'Fecha de nacimiento'**
  String get dateOfBirthLabel;

  /// Shown in the date-of-birth field before a date is picked.
  ///
  /// In es, this message translates to:
  /// **'Sin elegir'**
  String get notSet;

  /// No description provided for @choose.
  ///
  /// In es, this message translates to:
  /// **'Elegir'**
  String get choose;

  /// No description provided for @heightCmLabel.
  ///
  /// In es, this message translates to:
  /// **'Estatura (cm)'**
  String get heightCmLabel;

  /// No description provided for @weightKgLabel.
  ///
  /// In es, this message translates to:
  /// **'Peso (kg)'**
  String get weightKgLabel;

  /// No description provided for @dobMissing.
  ///
  /// In es, this message translates to:
  /// **'Elige tu fecha de nacimiento.'**
  String get dobMissing;

  /// No description provided for @dobInFuture.
  ///
  /// In es, this message translates to:
  /// **'La fecha de nacimiento no puede estar en el futuro.'**
  String get dobInFuture;

  /// No description provided for @ageOutOfRange.
  ///
  /// In es, this message translates to:
  /// **'la edad debe estar entre {min} y {max} años'**
  String ageOutOfRange(int min, int max);

  /// No description provided for @heightMissing.
  ///
  /// In es, this message translates to:
  /// **'pon tu estatura'**
  String get heightMissing;

  /// No description provided for @heightOutOfRange.
  ///
  /// In es, this message translates to:
  /// **'la estatura debe estar entre {min} y {max} cm'**
  String heightOutOfRange(int min, int max);

  /// No description provided for @weightMissing.
  ///
  /// In es, this message translates to:
  /// **'pon tu peso'**
  String get weightMissing;

  /// No description provided for @weightOutOfRange.
  ///
  /// In es, this message translates to:
  /// **'el peso debe estar entre {min} y {max} kg'**
  String weightOutOfRange(int min, int max);

  /// No description provided for @bodyFatOutOfRange.
  ///
  /// In es, this message translates to:
  /// **'la grasa corporal debe estar entre {min} y {max} %'**
  String bodyFatOutOfRange(int min, int max);

  /// No description provided for @yourGoals.
  ///
  /// In es, this message translates to:
  /// **'Tus objetivos'**
  String get yourGoals;

  /// No description provided for @pickAtLeastOne.
  ///
  /// In es, this message translates to:
  /// **'Elige al menos uno.'**
  String get pickAtLeastOne;

  /// No description provided for @goalLoseFat.
  ///
  /// In es, this message translates to:
  /// **'Bajar grasa'**
  String get goalLoseFat;

  /// No description provided for @goalBuildMuscle.
  ///
  /// In es, this message translates to:
  /// **'Ganar músculo'**
  String get goalBuildMuscle;

  /// No description provided for @goalRecomp.
  ///
  /// In es, this message translates to:
  /// **'Recomposición'**
  String get goalRecomp;

  /// No description provided for @goalMaintain.
  ///
  /// In es, this message translates to:
  /// **'Mantener'**
  String get goalMaintain;

  /// No description provided for @goalGainStrength.
  ///
  /// In es, this message translates to:
  /// **'Ganar fuerza'**
  String get goalGainStrength;

  /// No description provided for @optionalDetails.
  ///
  /// In es, this message translates to:
  /// **'Datos opcionales'**
  String get optionalDetails;

  /// No description provided for @youCanSkipThese.
  ///
  /// In es, this message translates to:
  /// **'Estos te los puedes saltar.'**
  String get youCanSkipThese;

  /// No description provided for @sexMale.
  ///
  /// In es, this message translates to:
  /// **'Hombre'**
  String get sexMale;

  /// No description provided for @sexFemale.
  ///
  /// In es, this message translates to:
  /// **'Mujer'**
  String get sexFemale;

  /// No description provided for @clear.
  ///
  /// In es, this message translates to:
  /// **'Quitar'**
  String get clear;

  /// No description provided for @bodyFatOptionalLabel.
  ///
  /// In es, this message translates to:
  /// **'% de grasa corporal (opcional)'**
  String get bodyFatOptionalLabel;

  /// No description provided for @chooseYourBodyType.
  ///
  /// In es, this message translates to:
  /// **'Elige tu tipo de cuerpo'**
  String get chooseYourBodyType;

  /// No description provided for @bodyShape.
  ///
  /// In es, this message translates to:
  /// **'Complexión'**
  String get bodyShape;

  /// No description provided for @bodyFatLevel.
  ///
  /// In es, this message translates to:
  /// **'Nivel de grasa corporal'**
  String get bodyFatLevel;

  /// No description provided for @findMyProgram.
  ///
  /// In es, this message translates to:
  /// **'Buscar mi programa'**
  String get findMyProgram;

  /// No description provided for @bodyShapeEctomorph.
  ///
  /// In es, this message translates to:
  /// **'Delgado y angosto'**
  String get bodyShapeEctomorph;

  /// No description provided for @bodyShapeMesomorph.
  ///
  /// In es, this message translates to:
  /// **'Atlético y ancho'**
  String get bodyShapeMesomorph;

  /// No description provided for @bodyShapeEndomorph.
  ///
  /// In es, this message translates to:
  /// **'Robusto y macizo'**
  String get bodyShapeEndomorph;

  /// No description provided for @bodyShapeEctomorphDescription.
  ///
  /// In es, this message translates to:
  /// **'Cuerpo delgado, hombros angostos, extremidades largas, poca masa.'**
  String get bodyShapeEctomorphDescription;

  /// No description provided for @bodyShapeMesomorphDescription.
  ///
  /// In es, this message translates to:
  /// **'Hombros anchos, cuerpo musculoso, responde bien al entrenamiento.'**
  String get bodyShapeMesomorphDescription;

  /// No description provided for @bodyShapeEndomorphDescription.
  ///
  /// In es, this message translates to:
  /// **'Cuerpo robusto, cintura ancha, tiende a cargar más masa.'**
  String get bodyShapeEndomorphDescription;

  /// No description provided for @fatBandLean.
  ///
  /// In es, this message translates to:
  /// **'Definido'**
  String get fatBandLean;

  /// No description provided for @fatBandModerate.
  ///
  /// In es, this message translates to:
  /// **'Medio'**
  String get fatBandModerate;

  /// No description provided for @fatBandBulky.
  ///
  /// In es, this message translates to:
  /// **'Con más grasa'**
  String get fatBandBulky;

  /// No description provided for @fatBandLeanDescription.
  ///
  /// In es, this message translates to:
  /// **'Músculo visible y definido'**
  String get fatBandLeanDescription;

  /// No description provided for @fatBandModerateDescription.
  ///
  /// In es, this message translates to:
  /// **'Algo de definición, grasa moderada'**
  String get fatBandModerateDescription;

  /// No description provided for @fatBandBulkyDescription.
  ///
  /// In es, this message translates to:
  /// **'Más grasa corporal, aspecto lleno'**
  String get fatBandBulkyDescription;

  /// No description provided for @yourProgram.
  ///
  /// In es, this message translates to:
  /// **'Tu programa'**
  String get yourProgram;

  /// No description provided for @couldNotLoadYourProgram.
  ///
  /// In es, this message translates to:
  /// **'No se pudo cargar tu programa'**
  String get couldNotLoadYourProgram;

  /// No description provided for @retry.
  ///
  /// In es, this message translates to:
  /// **'Reintentar'**
  String get retry;

  /// No description provided for @noProgramYet.
  ///
  /// In es, this message translates to:
  /// **'Todavía no tienes programa'**
  String get noProgramYet;

  /// No description provided for @training.
  ///
  /// In es, this message translates to:
  /// **'Entrenamiento'**
  String get training;

  /// Row label: how the training days divide the body. Lifters use the English word.
  ///
  /// In es, this message translates to:
  /// **'Split'**
  String get split;

  /// No description provided for @daysPerWeek.
  ///
  /// In es, this message translates to:
  /// **'{n} días/semana'**
  String daysPerWeek(int n);

  /// No description provided for @sessionDuration.
  ///
  /// In es, this message translates to:
  /// **'Duración de la sesión'**
  String get sessionDuration;

  /// No description provided for @minutesShort.
  ///
  /// In es, this message translates to:
  /// **'{n} min'**
  String minutesShort(int n);

  /// No description provided for @intensity.
  ///
  /// In es, this message translates to:
  /// **'Intensidad'**
  String get intensity;

  /// No description provided for @rest.
  ///
  /// In es, this message translates to:
  /// **'Descanso'**
  String get rest;

  /// No description provided for @progression.
  ///
  /// In es, this message translates to:
  /// **'Progresión'**
  String get progression;

  /// No description provided for @highlightExercises.
  ///
  /// In es, this message translates to:
  /// **'Ejercicios clave'**
  String get highlightExercises;

  /// No description provided for @nutrition.
  ///
  /// In es, this message translates to:
  /// **'Alimentación'**
  String get nutrition;

  /// No description provided for @approach.
  ///
  /// In es, this message translates to:
  /// **'Enfoque'**
  String get approach;

  /// No description provided for @strategy.
  ///
  /// In es, this message translates to:
  /// **'Estrategia'**
  String get strategy;

  /// No description provided for @meals.
  ///
  /// In es, this message translates to:
  /// **'Comidas'**
  String get meals;

  /// No description provided for @protein.
  ///
  /// In es, this message translates to:
  /// **'Proteína'**
  String get protein;

  /// No description provided for @carbs.
  ///
  /// In es, this message translates to:
  /// **'Carbos'**
  String get carbs;

  /// No description provided for @fat.
  ///
  /// In es, this message translates to:
  /// **'Grasa'**
  String get fat;

  /// No description provided for @grams.
  ///
  /// In es, this message translates to:
  /// **'{n} g'**
  String grams(int n);

  /// No description provided for @kcalPerDay.
  ///
  /// In es, this message translates to:
  /// **'kcal / día'**
  String get kcalPerDay;

  /// Eyebrow label on the target-physique card, set in capitals.
  ///
  /// In es, this message translates to:
  /// **'FÍSICO OBJETIVO'**
  String get targetPhysiqueCaps;

  /// No description provided for @physiqueClassicAesthetic.
  ///
  /// In es, this message translates to:
  /// **'Estética clásica'**
  String get physiqueClassicAesthetic;

  /// No description provided for @physiqueModernHypertrophy.
  ///
  /// In es, this message translates to:
  /// **'Hipertrofia moderna'**
  String get physiqueModernHypertrophy;

  /// No description provided for @physiqueLeanMinimalist.
  ///
  /// In es, this message translates to:
  /// **'Minimalista definido'**
  String get physiqueLeanMinimalist;

  /// No description provided for @physiquePowerbuilder.
  ///
  /// In es, this message translates to:
  /// **'Powerbuilder'**
  String get physiquePowerbuilder;

  /// No description provided for @physiqueHeavyDutyMass.
  ///
  /// In es, this message translates to:
  /// **'Masa heavy-duty'**
  String get physiqueHeavyDutyMass;

  /// No description provided for @physiqueMassMonster.
  ///
  /// In es, this message translates to:
  /// **'Mass monster'**
  String get physiqueMassMonster;

  /// No description provided for @physiqueClassicAestheticBlurb.
  ///
  /// In es, this message translates to:
  /// **'Hombros anchos, cintura chica: la X de la era dorada.'**
  String get physiqueClassicAestheticBlurb;

  /// No description provided for @physiqueModernHypertrophyBlurb.
  ///
  /// In es, this message translates to:
  /// **'Equilibrado, detallado, atlético: proporción antes que puro tamaño.'**
  String get physiqueModernHypertrophyBlurb;

  /// No description provided for @physiqueLeanMinimalistBlurb.
  ///
  /// In es, this message translates to:
  /// **'Seco y duro: músculo denso, sin exceso.'**
  String get physiqueLeanMinimalistBlurb;

  /// No description provided for @physiquePowerbuilderBlurb.
  ///
  /// In es, this message translates to:
  /// **'Grueso y potente: mezcla de strongman y fisicoculturista.'**
  String get physiquePowerbuilderBlurb;

  /// No description provided for @physiqueHeavyDutyMassBlurb.
  ///
  /// In es, this message translates to:
  /// **'Masa densa heavy-duty: máximo músculo, cuadrado y fuerte.'**
  String get physiqueHeavyDutyMassBlurb;

  /// No description provided for @physiqueMassMonsterBlurb.
  ///
  /// In es, this message translates to:
  /// **'Tamaño máximo: la silueta del mass monster.'**
  String get physiqueMassMonsterBlurb;

  /// No description provided for @physiqueDefaultBlurb.
  ///
  /// In es, this message translates to:
  /// **'El físico al que apunta tu programa.'**
  String get physiqueDefaultBlurb;

  /// Tappable tag; ratio is the shoulder-to-waist figure already formatted (e.g. 2.6).
  ///
  /// In es, this message translates to:
  /// **'Taper {ratio}:1  ⓘ'**
  String taperTag(String ratio);

  /// No description provided for @physiqueMass.
  ///
  /// In es, this message translates to:
  /// **'Masa'**
  String get physiqueMass;

  /// No description provided for @physiqueLean.
  ///
  /// In es, this message translates to:
  /// **'Definido'**
  String get physiqueLean;

  /// No description provided for @physiqueBalanced.
  ///
  /// In es, this message translates to:
  /// **'Equilibrado'**
  String get physiqueBalanced;

  /// No description provided for @taperByBuild.
  ///
  /// In es, this message translates to:
  /// **'Taper por físico'**
  String get taperByBuild;

  /// No description provided for @taperTableExplainer.
  ///
  /// In es, this message translates to:
  /// **'Relación hombros-cintura de la silueta objetivo. Un número mayor es una V más marcada. Compara estos físicos entre sí; no es una medida de tu cuerpo.'**
  String get taperTableExplainer;

  /// No description provided for @taperRatio.
  ///
  /// In es, this message translates to:
  /// **'{ratio}:1'**
  String taperRatio(String ratio);

  /// Eyebrow label on the weekly progress card, set in capitals.
  ///
  /// In es, this message translates to:
  /// **'ESTA SEMANA'**
  String get thisWeekCaps;

  /// No description provided for @daysOfTarget.
  ///
  /// In es, this message translates to:
  /// **'{done} de {target} días'**
  String daysOfTarget(int done, int target);

  /// No description provided for @sessionsLoggedOnProgram.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 sesión registrada en este programa} other{{count} sesiones registradas en este programa}}'**
  String sessionsLoggedOnProgram(int count);

  /// No description provided for @targetMet.
  ///
  /// In es, this message translates to:
  /// **'Meta cumplida'**
  String get targetMet;

  /// No description provided for @percent.
  ///
  /// In es, this message translates to:
  /// **'{n}%'**
  String percent(int n);

  /// No description provided for @chooseYourProgram.
  ///
  /// In es, this message translates to:
  /// **'Elige tu programa'**
  String get chooseYourProgram;

  /// No description provided for @couldNotLoadProposals.
  ///
  /// In es, this message translates to:
  /// **'No se pudieron cargar las propuestas'**
  String get couldNotLoadProposals;

  /// Chip on the first of the three proposed programs.
  ///
  /// In es, this message translates to:
  /// **'Mejor opción'**
  String get rankBestMatch;

  /// No description provided for @rankCloseMatch.
  ///
  /// In es, this message translates to:
  /// **'Segunda opción'**
  String get rankCloseMatch;

  /// No description provided for @rankGoodOption.
  ///
  /// In es, this message translates to:
  /// **'Buena opción'**
  String get rankGoodOption;

  /// No description provided for @proposalSummaryLine.
  ///
  /// In es, this message translates to:
  /// **'{days} días/semana · {kcal} kcal/día'**
  String proposalSummaryLine(int days, int kcal);

  /// No description provided for @intensityLine.
  ///
  /// In es, this message translates to:
  /// **'Intensidad: {value}'**
  String intensityLine(String value);

  /// No description provided for @restLine.
  ///
  /// In es, this message translates to:
  /// **'Descanso: {value}'**
  String restLine(String value);

  /// No description provided for @progressionLine.
  ///
  /// In es, this message translates to:
  /// **'Progresión: {value}'**
  String progressionLine(String value);

  /// No description provided for @chooseThisProgram.
  ///
  /// In es, this message translates to:
  /// **'Elegir este programa'**
  String get chooseThisProgram;

  /// No description provided for @selectionNoLongerAvailable.
  ///
  /// In es, this message translates to:
  /// **'Esa opción ya no está disponible. Vuelve a cargar.'**
  String get selectionNoLongerAvailable;

  /// No description provided for @couldNotChooseProgram.
  ///
  /// In es, this message translates to:
  /// **'No se pudo elegir el programa. Inténtalo de nuevo.'**
  String get couldNotChooseProgram;

  /// No description provided for @profileMenu.
  ///
  /// In es, this message translates to:
  /// **'Tu perfil'**
  String get profileMenu;

  /// No description provided for @menuProgram.
  ///
  /// In es, this message translates to:
  /// **'Programa'**
  String get menuProgram;

  /// No description provided for @menuMeasurements.
  ///
  /// In es, this message translates to:
  /// **'Medidas'**
  String get menuMeasurements;

  /// No description provided for @menuProgress.
  ///
  /// In es, this message translates to:
  /// **'Progreso'**
  String get menuProgress;

  /// No description provided for @signOut.
  ///
  /// In es, this message translates to:
  /// **'Cerrar sesión'**
  String get signOut;

  /// No description provided for @dictate.
  ///
  /// In es, this message translates to:
  /// **'Dictar'**
  String get dictate;

  /// No description provided for @today.
  ///
  /// In es, this message translates to:
  /// **'Hoy'**
  String get today;

  /// No description provided for @dayDoneHeadline.
  ///
  /// In es, this message translates to:
  /// **'Entrenaste'**
  String get dayDoneHeadline;

  /// No description provided for @dayFreeHeadline.
  ///
  /// In es, this message translates to:
  /// **'Sesión libre'**
  String get dayFreeHeadline;

  /// No description provided for @startSession.
  ///
  /// In es, this message translates to:
  /// **'Empezar sesión'**
  String get startSession;

  /// No description provided for @logLate.
  ///
  /// In es, this message translates to:
  /// **'Registrar tarde'**
  String get logLate;

  /// No description provided for @viewLogged.
  ///
  /// In es, this message translates to:
  /// **'Ver lo registrado'**
  String get viewLogged;

  /// No description provided for @nothingLogged.
  ///
  /// In es, this message translates to:
  /// **'Nada registrado este día.'**
  String get nothingLogged;

  /// No description provided for @nutritionToday.
  ///
  /// In es, this message translates to:
  /// **'Alimentación · hoy'**
  String get nutritionToday;

  /// No description provided for @addMeal.
  ///
  /// In es, this message translates to:
  /// **'+ Comida'**
  String get addMeal;

  /// No description provided for @showMonth.
  ///
  /// In es, this message translates to:
  /// **'Ver el mes'**
  String get showMonth;

  /// No description provided for @showWeek.
  ///
  /// In es, this message translates to:
  /// **'Ver la semana'**
  String get showWeek;

  /// No description provided for @findYourProgram.
  ///
  /// In es, this message translates to:
  /// **'Encuentra tu programa'**
  String get findYourProgram;

  /// Fallback for a timeout or no-network failure on any API call.
  ///
  /// In es, this message translates to:
  /// **'No hay conexión con el servidor. Inténtalo de nuevo.'**
  String get apiErrorTransport;

  /// No description provided for @apiErrorBadRequest.
  ///
  /// In es, this message translates to:
  /// **'Revisa tus datos.'**
  String get apiErrorBadRequest;

  /// No description provided for @apiErrorUnauthorized.
  ///
  /// In es, this message translates to:
  /// **'Correo o contraseña incorrectos.'**
  String get apiErrorUnauthorized;

  /// No description provided for @apiErrorNotFound.
  ///
  /// In es, this message translates to:
  /// **'Eso ya no existe.'**
  String get apiErrorNotFound;

  /// No description provided for @apiErrorConflict.
  ///
  /// In es, this message translates to:
  /// **'Eso ya existe.'**
  String get apiErrorConflict;

  /// No description provided for @apiErrorUnknown.
  ///
  /// In es, this message translates to:
  /// **'Algo salió mal. Inténtalo de nuevo.'**
  String get apiErrorUnknown;

  /// No description provided for @regionNeck.
  ///
  /// In es, this message translates to:
  /// **'Cuello'**
  String get regionNeck;

  /// No description provided for @regionTraps.
  ///
  /// In es, this message translates to:
  /// **'Trapecios'**
  String get regionTraps;

  /// No description provided for @regionShoulders.
  ///
  /// In es, this message translates to:
  /// **'Hombros'**
  String get regionShoulders;

  /// No description provided for @regionChest.
  ///
  /// In es, this message translates to:
  /// **'Pecho'**
  String get regionChest;

  /// No description provided for @regionBiceps.
  ///
  /// In es, this message translates to:
  /// **'Bíceps'**
  String get regionBiceps;

  /// No description provided for @regionTriceps.
  ///
  /// In es, this message translates to:
  /// **'Tríceps'**
  String get regionTriceps;

  /// No description provided for @regionForearms.
  ///
  /// In es, this message translates to:
  /// **'Antebrazos'**
  String get regionForearms;

  /// No description provided for @regionCore.
  ///
  /// In es, this message translates to:
  /// **'Core'**
  String get regionCore;

  /// No description provided for @regionLats.
  ///
  /// In es, this message translates to:
  /// **'Dorsales'**
  String get regionLats;

  /// No description provided for @regionUpperBack.
  ///
  /// In es, this message translates to:
  /// **'Espalda alta'**
  String get regionUpperBack;

  /// No description provided for @regionLowerBack.
  ///
  /// In es, this message translates to:
  /// **'Lumbares'**
  String get regionLowerBack;

  /// No description provided for @regionQuads.
  ///
  /// In es, this message translates to:
  /// **'Cuádriceps'**
  String get regionQuads;

  /// No description provided for @regionHamstrings.
  ///
  /// In es, this message translates to:
  /// **'Isquios'**
  String get regionHamstrings;

  /// No description provided for @regionGlutes.
  ///
  /// In es, this message translates to:
  /// **'Glúteos'**
  String get regionGlutes;

  /// No description provided for @regionCalves.
  ///
  /// In es, this message translates to:
  /// **'Pantorrillas'**
  String get regionCalves;

  /// No description provided for @muscleChest.
  ///
  /// In es, this message translates to:
  /// **'Pecho'**
  String get muscleChest;

  /// No description provided for @muscleBack.
  ///
  /// In es, this message translates to:
  /// **'Espalda'**
  String get muscleBack;

  /// No description provided for @muscleShoulders.
  ///
  /// In es, this message translates to:
  /// **'Hombros'**
  String get muscleShoulders;

  /// No description provided for @muscleArms.
  ///
  /// In es, this message translates to:
  /// **'Brazos'**
  String get muscleArms;

  /// No description provided for @muscleLegs.
  ///
  /// In es, this message translates to:
  /// **'Piernas'**
  String get muscleLegs;

  /// No description provided for @muscleCore.
  ///
  /// In es, this message translates to:
  /// **'Core'**
  String get muscleCore;

  /// Joins the last item of a spoken list to the rest: 'quads and glutes'.
  ///
  /// In es, this message translates to:
  /// **'{head} y {last}'**
  String listAnd(String head, String last);

  /// No description provided for @rejectStartWorkoutFirst.
  ///
  /// In es, this message translates to:
  /// **'Primero empieza un entrenamiento.'**
  String get rejectStartWorkoutFirst;

  /// No description provided for @rejectAddExerciseFirst.
  ///
  /// In es, this message translates to:
  /// **'Primero agrega un ejercicio.'**
  String get rejectAddExerciseFirst;

  /// No description provided for @rejectEnterExerciseName.
  ///
  /// In es, this message translates to:
  /// **'Escribe el nombre del ejercicio.'**
  String get rejectEnterExerciseName;

  /// No description provided for @rejectNameTooLong.
  ///
  /// In es, this message translates to:
  /// **'El nombre puede tener hasta {max} caracteres.'**
  String rejectNameTooLong(int max);

  /// No description provided for @rejectEnterReps.
  ///
  /// In es, this message translates to:
  /// **'Pon las repeticiones.'**
  String get rejectEnterReps;

  /// No description provided for @rejectRepsOutOfRange.
  ///
  /// In es, this message translates to:
  /// **'Las repeticiones van de {min} a {max}.'**
  String rejectRepsOutOfRange(int min, int max);

  /// No description provided for @rejectWeightOutOfRange.
  ///
  /// In es, this message translates to:
  /// **'El peso debe ser mayor que 0 y máximo {max} kg.'**
  String rejectWeightOutOfRange(int max);

  /// No description provided for @rejectRpeOutOfRange.
  ///
  /// In es, this message translates to:
  /// **'El RPE va de {min} a {max}, en pasos de 0.5.'**
  String rejectRpeOutOfRange(double min, double max);

  /// No description provided for @fieldAreaSets.
  ///
  /// In es, this message translates to:
  /// **'tus series'**
  String get fieldAreaSets;

  /// No description provided for @fieldAreaExercise.
  ///
  /// In es, this message translates to:
  /// **'el ejercicio'**
  String get fieldAreaExercise;

  /// No description provided for @fieldAreaWorkout.
  ///
  /// In es, this message translates to:
  /// **'el entrenamiento'**
  String get fieldAreaWorkout;

  /// A finish failure that names the part of the screen to check.
  ///
  /// In es, this message translates to:
  /// **'{error} Revisa {area}.'**
  String errorCheckArea(String error, String area);

  /// No description provided for @discardThisWorkout.
  ///
  /// In es, this message translates to:
  /// **'¿Descartar este entrenamiento?'**
  String get discardThisWorkout;

  /// No description provided for @yourLoggedSetsWillBeLost.
  ///
  /// In es, this message translates to:
  /// **'Se perderán las series que registraste.'**
  String get yourLoggedSetsWillBeLost;

  /// No description provided for @cancel.
  ///
  /// In es, this message translates to:
  /// **'Cancelar'**
  String get cancel;

  /// No description provided for @discard.
  ///
  /// In es, this message translates to:
  /// **'Descartar'**
  String get discard;

  /// No description provided for @workout.
  ///
  /// In es, this message translates to:
  /// **'Entrenamiento'**
  String get workout;

  /// No description provided for @earbudCoachOn.
  ///
  /// In es, this message translates to:
  /// **'Activar coach en audífonos'**
  String get earbudCoachOn;

  /// No description provided for @earbudCoachOff.
  ///
  /// In es, this message translates to:
  /// **'Apagar coach en audífonos'**
  String get earbudCoachOff;

  /// No description provided for @voiceCoachOn.
  ///
  /// In es, this message translates to:
  /// **'Activar coach de voz'**
  String get voiceCoachOn;

  /// No description provided for @voiceCoachOff.
  ///
  /// In es, this message translates to:
  /// **'Apagar coach de voz'**
  String get voiceCoachOff;

  /// No description provided for @addExercise.
  ///
  /// In es, this message translates to:
  /// **'Agregar ejercicio'**
  String get addExercise;

  /// No description provided for @targetMusclesCaps.
  ///
  /// In es, this message translates to:
  /// **'MÚSCULOS OBJETIVO — {name}'**
  String targetMusclesCaps(String name);

  /// No description provided for @stopListening.
  ///
  /// In es, this message translates to:
  /// **'Dejar de escuchar'**
  String get stopListening;

  /// No description provided for @dictateASet.
  ///
  /// In es, this message translates to:
  /// **'Dictar una serie'**
  String get dictateASet;

  /// No description provided for @listening.
  ///
  /// In es, this message translates to:
  /// **'Escuchando…'**
  String get listening;

  /// No description provided for @coachIdleHint.
  ///
  /// In es, this message translates to:
  /// **'Toca el micrófono y di \"listo\": te pregunto repeticiones y kilos. Termina cada respuesta con \"cambio\".'**
  String get coachIdleHint;

  /// No description provided for @setLine.
  ///
  /// In es, this message translates to:
  /// **'Serie {n}: {label}'**
  String setLine(int n, String label);

  /// No description provided for @repsCount.
  ///
  /// In es, this message translates to:
  /// **'{n} reps'**
  String repsCount(int n);

  /// No description provided for @kg.
  ///
  /// In es, this message translates to:
  /// **'{value} kg'**
  String kg(String value);

  /// No description provided for @rpeValue.
  ///
  /// In es, this message translates to:
  /// **'RPE {value}'**
  String rpeValue(String value);

  /// No description provided for @repsLabel.
  ///
  /// In es, this message translates to:
  /// **'Reps'**
  String get repsLabel;

  /// No description provided for @rpeLabel.
  ///
  /// In es, this message translates to:
  /// **'RPE'**
  String get rpeLabel;

  /// No description provided for @repeatLastSet.
  ///
  /// In es, this message translates to:
  /// **'Repetir última serie'**
  String get repeatLastSet;

  /// No description provided for @logSet.
  ///
  /// In es, this message translates to:
  /// **'Registrar serie'**
  String get logSet;

  /// No description provided for @newExercise.
  ///
  /// In es, this message translates to:
  /// **'Nuevo ejercicio'**
  String get newExercise;

  /// No description provided for @add.
  ///
  /// In es, this message translates to:
  /// **'Agregar'**
  String get add;

  /// No description provided for @exerciseNameLabel.
  ///
  /// In es, this message translates to:
  /// **'Nombre del ejercicio'**
  String get exerciseNameLabel;

  /// No description provided for @commonLifts.
  ///
  /// In es, this message translates to:
  /// **'Ejercicios comunes'**
  String get commonLifts;

  /// No description provided for @muscleGroupOptional.
  ///
  /// In es, this message translates to:
  /// **'Grupo muscular (opcional)'**
  String get muscleGroupOptional;

  /// No description provided for @exerciseWithGroup.
  ///
  /// In es, this message translates to:
  /// **'{name} · {group}'**
  String exerciseWithGroup(String name, String group);

  /// No description provided for @noWorkoutsYet.
  ///
  /// In es, this message translates to:
  /// **'Aún no hay entrenamientos. Toca Empezar para registrar el primero.'**
  String get noWorkoutsYet;

  /// No description provided for @sessionSummary.
  ///
  /// In es, this message translates to:
  /// **'{exercises} ejercicios · {sets} series'**
  String sessionSummary(int exercises, int sets);

  /// No description provided for @delete.
  ///
  /// In es, this message translates to:
  /// **'Borrar'**
  String get delete;

  /// No description provided for @deleteThisWorkout.
  ///
  /// In es, this message translates to:
  /// **'¿Borrar este entrenamiento?'**
  String get deleteThisWorkout;

  /// No description provided for @yesterday.
  ///
  /// In es, this message translates to:
  /// **'Ayer'**
  String get yesterday;

  /// No description provided for @workoutNoLongerExists.
  ///
  /// In es, this message translates to:
  /// **'Ese entrenamiento ya no existe.'**
  String get workoutNoLongerExists;

  /// No description provided for @startWorkout.
  ///
  /// In es, this message translates to:
  /// **'Empezar entrenamiento'**
  String get startWorkout;

  /// No description provided for @coachSayDoneHandsFree.
  ///
  /// In es, this message translates to:
  /// **'Di listo cuando termines la serie.'**
  String get coachSayDoneHandsFree;

  /// No description provided for @coachSayDoneKeyMic.
  ///
  /// In es, this message translates to:
  /// **'Activa el micrófono y di listo cuando termines la serie.'**
  String get coachSayDoneKeyMic;

  /// Appended to an exercise name when its target muscles are known.
  ///
  /// In es, this message translates to:
  /// **' — objetivo {label}'**
  String coachTargetSuffix(String label);

  /// No description provided for @coachPlanLoaded.
  ///
  /// In es, this message translates to:
  /// **'Plan cargado: {count} ejercicios. Primero: {first}. {prompt}'**
  String coachPlanLoaded(int count, String first, String prompt);

  /// No description provided for @coachNoPlan.
  ///
  /// In es, this message translates to:
  /// **'Coach de voz activo. No hay plan: agrega un ejercicio y dicta tus series.'**
  String get coachNoPlan;

  /// No description provided for @coachOnCurrent.
  ///
  /// In es, this message translates to:
  /// **'Coach de voz activo. Ejercicio actual: {name}. {prompt}'**
  String coachOnCurrent(String name, String prompt);

  /// No description provided for @coachNoCurrentExercise.
  ///
  /// In es, this message translates to:
  /// **'ninguno'**
  String get coachNoCurrentExercise;

  /// No description provided for @voiceInputUnavailable.
  ///
  /// In es, this message translates to:
  /// **'Aquí no hay entrada de voz.'**
  String get voiceInputUnavailable;

  /// No description provided for @coachHowManyReps.
  ///
  /// In es, this message translates to:
  /// **'¿Cuántas repeticiones?'**
  String get coachHowManyReps;

  /// No description provided for @coachHowManyKilos.
  ///
  /// In es, this message translates to:
  /// **'¿Cuántos kilos? O di peso corporal.'**
  String get coachHowManyKilos;

  /// No description provided for @coachLoggedRestUp.
  ///
  /// In es, this message translates to:
  /// **'Registrado: {logged}. Descansa.'**
  String coachLoggedRestUp(String logged);

  /// No description provided for @coachAtKilos.
  ///
  /// In es, this message translates to:
  /// **'con {weight} kilos'**
  String coachAtKilos(String weight);

  /// No description provided for @coachNextUp.
  ///
  /// In es, this message translates to:
  /// **'Sigue: {next}.'**
  String coachNextUp(String next);

  /// No description provided for @coachLastExercise.
  ///
  /// In es, this message translates to:
  /// **'Ese era el último ejercicio. Di terminar entrenamiento para guardar.'**
  String get coachLastExercise;

  /// No description provided for @coachSaving.
  ///
  /// In es, this message translates to:
  /// **'Guardando tu entrenamiento.'**
  String get coachSaving;

  /// No description provided for @coachSaved.
  ///
  /// In es, this message translates to:
  /// **'Entrenamiento guardado. ¡Listo!'**
  String get coachSaved;

  /// No description provided for @coachGoodHowManyReps.
  ///
  /// In es, this message translates to:
  /// **'Bien. ¿Cuántas repeticiones?'**
  String get coachGoodHowManyReps;

  /// No description provided for @coachStandingBy.
  ///
  /// In es, this message translates to:
  /// **'En espera. Toca el micrófono cuando estés listo.'**
  String get coachStandingBy;

  /// No description provided for @coachDidNotCatch.
  ///
  /// In es, this message translates to:
  /// **'No te entendí. Di por ejemplo: diez repeticiones con sesenta kilos.'**
  String get coachDidNotCatch;

  /// No description provided for @coachJustReps.
  ///
  /// In es, this message translates to:
  /// **'Solo el número de repeticiones.'**
  String get coachJustReps;

  /// No description provided for @coachKilosAsNumber.
  ///
  /// In es, this message translates to:
  /// **'Los kilos en número, o di peso corporal.'**
  String get coachKilosAsNumber;

  /// No description provided for @coachLoggedNextSet.
  ///
  /// In es, this message translates to:
  /// **'Registrado: {logged}. Descansa y di listo cuando termines la siguiente serie.'**
  String coachLoggedNextSet(String logged);

  /// No description provided for @ttsNext.
  ///
  /// In es, this message translates to:
  /// **'Sigue: {name}.'**
  String ttsNext(String name);

  /// No description provided for @ttsNextSets.
  ///
  /// In es, this message translates to:
  /// **'Sigue: {name}. {sets} series de {reps} repeticiones.'**
  String ttsNextSets(String name, int sets, int reps);

  /// No description provided for @ttsNextSetsWeight.
  ///
  /// In es, this message translates to:
  /// **'Sigue: {name}. {sets} series de {reps} repeticiones con {weight} kg.'**
  String ttsNextSetsWeight(String name, int sets, int reps, String weight);

  /// No description provided for @ttsSetGo.
  ///
  /// In es, this message translates to:
  /// **'Serie {n}. Vamos.'**
  String ttsSetGo(int n);

  /// No description provided for @ttsSetOfGo.
  ///
  /// In es, this message translates to:
  /// **'Serie {n} de {total}. {reps} repeticiones. Vamos.'**
  String ttsSetOfGo(int n, int total, int reps);

  /// No description provided for @ttsSetOfWeightGo.
  ///
  /// In es, this message translates to:
  /// **'Serie {n} de {total}. {reps} repeticiones con {weight} kg. Vamos.'**
  String ttsSetOfWeightGo(int n, int total, int reps, String weight);

  /// No description provided for @ttsRest.
  ///
  /// In es, this message translates to:
  /// **'Descansa.'**
  String get ttsRest;

  /// No description provided for @ttsWorkoutDone.
  ///
  /// In es, this message translates to:
  /// **'Entrenamiento terminado.'**
  String get ttsWorkoutDone;

  /// No description provided for @ttsSeedFirstSet.
  ///
  /// In es, this message translates to:
  /// **'Registra tu primera serie en pantalla.'**
  String get ttsSeedFirstSet;

  /// No description provided for @sergeantVoiceUnavailable.
  ///
  /// In es, this message translates to:
  /// **'Aquí no hay entrada de voz. Toca una opción.'**
  String get sergeantVoiceUnavailable;

  /// No description provided for @sergeantGreeting.
  ///
  /// In es, this message translates to:
  /// **'Aquí el sargento. Di: empezar entrenamiento, ver plan, registrar comida. Termina cada orden con cambio.'**
  String get sergeantGreeting;

  /// No description provided for @sergeantRogerOut.
  ///
  /// In es, this message translates to:
  /// **'Recibido. Corto.'**
  String get sergeantRogerOut;

  /// No description provided for @sergeantLogged.
  ///
  /// In es, this message translates to:
  /// **'Registrado.'**
  String get sergeantLogged;

  /// No description provided for @sergeantTellMeMore.
  ///
  /// In es, this message translates to:
  /// **'Dime más.'**
  String get sergeantTellMeMore;

  /// No description provided for @sergeantRoger.
  ///
  /// In es, this message translates to:
  /// **'Recibido.'**
  String get sergeantRoger;

  /// No description provided for @sergeantDidNotCatch.
  ///
  /// In es, this message translates to:
  /// **'No te entendí.'**
  String get sergeantDidNotCatch;

  /// No description provided for @sergeantStandingBy.
  ///
  /// In es, this message translates to:
  /// **'En espera.'**
  String get sergeantStandingBy;

  /// No description provided for @sergeantAskMacros.
  ///
  /// In es, this message translates to:
  /// **'Dime los gramos de proteína, carbos y grasa, una porción como 200 gramos de pechuga de pollo, o un preset, como protein shake.'**
  String get sergeantAskMacros;

  /// No description provided for @sergeantStartingSession.
  ///
  /// In es, this message translates to:
  /// **'Empezando tu sesión.'**
  String get sergeantStartingSession;

  /// No description provided for @sergeantNoPlanYet.
  ///
  /// In es, this message translates to:
  /// **'Aún no hay plan. Vamos a buscar tu tipo de cuerpo.'**
  String get sergeantNoPlanYet;

  /// No description provided for @sergeantOpeningBodyMatch.
  ///
  /// In es, this message translates to:
  /// **'Abriendo el tipo de cuerpo.'**
  String get sergeantOpeningBodyMatch;

  /// No description provided for @sergeantRecentActivity.
  ///
  /// In es, this message translates to:
  /// **'Tu actividad reciente.'**
  String get sergeantRecentActivity;

  /// No description provided for @sergeantOpeningProfile.
  ///
  /// In es, this message translates to:
  /// **'Abriendo tu perfil.'**
  String get sergeantOpeningProfile;

  /// No description provided for @sergeantSayOptions.
  ///
  /// In es, this message translates to:
  /// **'Di: empezar entrenamiento, ver plan o registrar comida.'**
  String get sergeantSayOptions;

  /// No description provided for @sergeantDidNotGet.
  ///
  /// In es, this message translates to:
  /// **'No entendí \"{transcript}\". Di empezar entrenamiento, ver plan o registrar comida.'**
  String sergeantDidNotGet(String transcript);

  /// No description provided for @sergeantMacrosExample.
  ///
  /// In es, this message translates to:
  /// **'Gramos de proteína, carbos y grasa. Por ejemplo: 40 de proteína, 60 de carbos, 20 de grasa.'**
  String get sergeantMacrosExample;

  /// No description provided for @sergeantMealLogged.
  ///
  /// In es, this message translates to:
  /// **'Comida registrada'**
  String get sergeantMealLogged;

  /// No description provided for @sergeantLoggedLabel.
  ///
  /// In es, this message translates to:
  /// **'Registré {label}'**
  String sergeantLoggedLabel(String label);

  /// No description provided for @sergeantMealSummary.
  ///
  /// In es, this message translates to:
  /// **'{what}: {protein} de proteína, {carbs} de carbos, {fat} de grasa. {kcal} calorías. ¿Qué más?'**
  String sergeantMealSummary(
      String what, String protein, String carbs, String fat, int kcal);

  /// No description provided for @sergeantCouldNotSaveMeal.
  ///
  /// In es, this message translates to:
  /// **'No pude guardar la comida. Inténtalo en un momento.'**
  String get sergeantCouldNotSaveMeal;

  /// No description provided for @sergeantLookingUp.
  ///
  /// In es, this message translates to:
  /// **'Buscando {food}.'**
  String sergeantLookingUp(String food);

  /// No description provided for @sergeantCouldNotFind.
  ///
  /// In es, this message translates to:
  /// **'No encontré {food}. Dime los gramos de proteína, carbos y grasa.'**
  String sergeantCouldNotFind(String food);

  /// No description provided for @sergeantGramsOf.
  ///
  /// In es, this message translates to:
  /// **'{grams} gramos de {food}'**
  String sergeantGramsOf(String grams, String food);

  /// No description provided for @sergeantPlanSummary.
  ///
  /// In es, this message translates to:
  /// **'Tu plan: {split}. {days} días a la semana, {kcal} calorías.'**
  String sergeantPlanSummary(String split, int days, int kcal);

  /// No description provided for @hubWorkout.
  ///
  /// In es, this message translates to:
  /// **'Entrenar'**
  String get hubWorkout;

  /// No description provided for @hubMeal.
  ///
  /// In es, this message translates to:
  /// **'Comida'**
  String get hubMeal;

  /// No description provided for @hubProgram.
  ///
  /// In es, this message translates to:
  /// **'Programa'**
  String get hubProgram;

  /// No description provided for @hubBodyMatch.
  ///
  /// In es, this message translates to:
  /// **'Tipo de cuerpo'**
  String get hubBodyMatch;

  /// No description provided for @hubHistory.
  ///
  /// In es, this message translates to:
  /// **'Historial'**
  String get hubHistory;

  /// No description provided for @hubProfile.
  ///
  /// In es, this message translates to:
  /// **'Perfil'**
  String get hubProfile;

  /// No description provided for @hubIdleHint.
  ///
  /// In es, this message translates to:
  /// **'Toca el micrófono y habla. Termina cada orden con \"cambio\".'**
  String get hubIdleHint;

  /// No description provided for @progress.
  ///
  /// In es, this message translates to:
  /// **'Progreso'**
  String get progress;

  /// No description provided for @logout.
  ///
  /// In es, this message translates to:
  /// **'Salir'**
  String get logout;

  /// No description provided for @logMeasurement.
  ///
  /// In es, this message translates to:
  /// **'Registrar medida'**
  String get logMeasurement;

  /// No description provided for @bodyComposition.
  ///
  /// In es, this message translates to:
  /// **'Composición corporal'**
  String get bodyComposition;

  /// No description provided for @strength.
  ///
  /// In es, this message translates to:
  /// **'Fuerza'**
  String get strength;

  /// No description provided for @trainingBalance.
  ///
  /// In es, this message translates to:
  /// **'Balance de entrenamiento'**
  String get trainingBalance;

  /// No description provided for @couldNotLoadPullToRetry.
  ///
  /// In es, this message translates to:
  /// **'No se pudo cargar. Desliza para reintentar.'**
  String get couldNotLoadPullToRetry;

  /// No description provided for @logACoupleOfMeasurements.
  ///
  /// In es, this message translates to:
  /// **'Registra un par de medidas para ver tu tendencia.'**
  String get logACoupleOfMeasurements;

  /// No description provided for @bodyFatPercentLabel.
  ///
  /// In es, this message translates to:
  /// **'% de grasa'**
  String get bodyFatPercentLabel;

  /// No description provided for @leanMass.
  ///
  /// In es, this message translates to:
  /// **'Masa magra'**
  String get leanMass;

  /// No description provided for @weight.
  ///
  /// In es, this message translates to:
  /// **'Peso'**
  String get weight;

  /// No description provided for @bodyFat.
  ///
  /// In es, this message translates to:
  /// **'Grasa corporal'**
  String get bodyFat;

  /// No description provided for @overTime.
  ///
  /// In es, this message translates to:
  /// **'{label} en el tiempo'**
  String overTime(String label);

  /// No description provided for @logAFewWeightedSessions.
  ///
  /// In es, this message translates to:
  /// **'Registra unas sesiones con peso para ver subir tu fuerza.'**
  String get logAFewWeightedSessions;

  /// No description provided for @estimatedOneRmAllLifts.
  ///
  /// In es, this message translates to:
  /// **'1RM estimado · todos los ejercicios'**
  String get estimatedOneRmAllLifts;

  /// No description provided for @estimatedOneRm.
  ///
  /// In es, this message translates to:
  /// **'1RM estimado · {name}'**
  String estimatedOneRm(String name);

  /// No description provided for @allLifts.
  ///
  /// In es, this message translates to:
  /// **'Todos'**
  String get allLifts;

  /// No description provided for @gainSinceTracking.
  ///
  /// In es, this message translates to:
  /// **'{gain} kg desde que lo registras'**
  String gainSinceTracking(String gain);

  /// No description provided for @e1rmLabel.
  ///
  /// In es, this message translates to:
  /// **'1RM est.'**
  String get e1rmLabel;

  /// No description provided for @sessionVolume.
  ///
  /// In es, this message translates to:
  /// **'Volumen por sesión'**
  String get sessionVolume;

  /// No description provided for @volume.
  ///
  /// In es, this message translates to:
  /// **'Volumen'**
  String get volume;

  /// No description provided for @prBadge.
  ///
  /// In es, this message translates to:
  /// **'PR'**
  String get prBadge;

  /// No description provided for @coachSuggestions.
  ///
  /// In es, this message translates to:
  /// **'Sugerencias del coach'**
  String get coachSuggestions;

  /// No description provided for @lastWeeks.
  ///
  /// In es, this message translates to:
  /// **'últimas {n} sem'**
  String lastWeeks(int n);

  /// No description provided for @onTrack.
  ///
  /// In es, this message translates to:
  /// **'Vas bien. Sigue igual.'**
  String get onTrack;

  /// No description provided for @actionCaps.
  ///
  /// In es, this message translates to:
  /// **'ACCIÓN'**
  String get actionCaps;

  /// No description provided for @measurementLogged.
  ///
  /// In es, this message translates to:
  /// **'Medida registrada'**
  String get measurementLogged;

  /// No description provided for @weighInHint.
  ///
  /// In es, this message translates to:
  /// **'Pésate. El % de grasa es opcional, pero alimenta la tendencia.'**
  String get weighInHint;

  /// No description provided for @bodyFatPercentInputLabel.
  ///
  /// In es, this message translates to:
  /// **'Grasa (%)'**
  String get bodyFatPercentInputLabel;

  /// No description provided for @saveMeasurement.
  ///
  /// In es, this message translates to:
  /// **'Guardar medida'**
  String get saveMeasurement;

  /// No description provided for @mealLogged.
  ///
  /// In es, this message translates to:
  /// **'Comida registrada'**
  String get mealLogged;

  /// No description provided for @logAMeal.
  ///
  /// In es, this message translates to:
  /// **'Registrar una comida'**
  String get logAMeal;

  /// No description provided for @proteinGramsLabel.
  ///
  /// In es, this message translates to:
  /// **'Proteína (g)'**
  String get proteinGramsLabel;

  /// No description provided for @carbsGramsLabel.
  ///
  /// In es, this message translates to:
  /// **'Carbos (g)'**
  String get carbsGramsLabel;

  /// No description provided for @fatGramsLabel.
  ///
  /// In es, this message translates to:
  /// **'Grasa (g)'**
  String get fatGramsLabel;

  /// No description provided for @kcalLoggedForToday.
  ///
  /// In es, this message translates to:
  /// **'≈ {kcal} kcal · se registra hoy'**
  String kcalLoggedForToday(String kcal);

  /// No description provided for @saveMeal.
  ///
  /// In es, this message translates to:
  /// **'Guardar comida'**
  String get saveMeal;

  /// A metric tile's number and its unit, e.g. 82kg or 18%.
  ///
  /// In es, this message translates to:
  /// **'{value}{unit}'**
  String valueWithUnit(String value, String unit);
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'es'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'es':
      return AppLocalizationsEs();
  }

  throw FlutterError(
      'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
      'an issue with the localizations generation tool. Please file an issue '
      'on GitHub with a reproducible sample app and the gen-l10n configuration '
      'that was used.');
}
