import '../../../l10n/app_localizations.dart';
import '../domain/save_notice.dart';

/// The catalogue line for a [SaveNotice] (R-0061 AC11), shown once the
/// session is saved.
String saveNoticeText(SaveNotice notice, AppLocalizations l10n) =>
    switch (notice) {
      SaveNotice.programNotCurrent => l10n.noticeProgramNotCurrent,
      SaveNotice.planUnavailable => l10n.noticePlanUnavailable,
    };
