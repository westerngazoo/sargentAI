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
  /// **'Volver a entrar'**
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
  /// **'Si ese correo tiene cuenta, el link ya va en camino. Vale 30 minutos.'**
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
  /// **'Este link ya no sirve. Los links vencen a los 30 minutos y se usan una vez.'**
  String get linkNoLongerValid;

  /// No description provided for @requestANewLink.
  ///
  /// In es, this message translates to:
  /// **'Pedir otro link'**
  String get requestANewLink;
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
