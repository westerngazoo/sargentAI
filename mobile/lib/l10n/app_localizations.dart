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
  /// **'pon tu fecha de nacimiento'**
  String get dobMissing;

  /// No description provided for @dobInFuture.
  ///
  /// In es, this message translates to:
  /// **'la fecha de nacimiento no puede ser futura'**
  String get dobInFuture;

  /// No description provided for @ageOutOfRange.
  ///
  /// In es, this message translates to:
  /// **'la edad debe estar entre {min} y {max}'**
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
  String bodyFatOutOfRange(num min, num max);

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
  /// **'Delgado y estrecho'**
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
  /// **'Intermedio'**
  String get fatBandModerate;

  /// No description provided for @fatBandBulky.
  ///
  /// In es, this message translates to:
  /// **'Voluminoso'**
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

  /// Eyebrow label on the home program card, set in capitals.
  ///
  /// In es, this message translates to:
  /// **'TU PROGRAMA'**
  String get yourProgramCaps;

  /// No description provided for @kcalShort.
  ///
  /// In es, this message translates to:
  /// **'{n} kcal'**
  String kcalShort(int n);

  /// Centre of the progress ring: training days done over the weekly target.
  ///
  /// In es, this message translates to:
  /// **'{done}/{target}'**
  String doneOfTarget(int done, int target);

  /// No description provided for @getYourProgram.
  ///
  /// In es, this message translates to:
  /// **'Consigue tu programa'**
  String get getYourProgram;

  /// No description provided for @takeAPhotoForPlan.
  ///
  /// In es, this message translates to:
  /// **'Tómate una foto y recibe un plan a tu medida'**
  String get takeAPhotoForPlan;

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
  /// **'Opción cercana'**
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
