// Catalogue names for the body-type picker's wire enums (R-0061 AC11). The
// enums stay plain wire tokens; the words the user reads live here.

import '../../../l10n/app_localizations.dart';
import '../models/synthetic_match.dart';

String bodyShapeName(BodyShape shape, AppLocalizations l10n) => switch (shape) {
      BodyShape.ectomorph => l10n.bodyShapeEctomorph,
      BodyShape.mesomorph => l10n.bodyShapeMesomorph,
      BodyShape.endomorph => l10n.bodyShapeEndomorph,
    };

String bodyShapeDescription(BodyShape shape, AppLocalizations l10n) =>
    switch (shape) {
      BodyShape.ectomorph => l10n.bodyShapeEctomorphDescription,
      BodyShape.mesomorph => l10n.bodyShapeMesomorphDescription,
      BodyShape.endomorph => l10n.bodyShapeEndomorphDescription,
    };

String fatBandName(FatBand band, AppLocalizations l10n) => switch (band) {
      FatBand.lean => l10n.fatBandLean,
      FatBand.moderate => l10n.fatBandModerate,
      FatBand.bulky => l10n.fatBandBulky,
    };

String fatBandDescription(FatBand band, AppLocalizations l10n) =>
    switch (band) {
      FatBand.lean => l10n.fatBandLeanDescription,
      FatBand.moderate => l10n.fatBandModerateDescription,
      FatBand.bulky => l10n.fatBandBulkyDescription,
    };
