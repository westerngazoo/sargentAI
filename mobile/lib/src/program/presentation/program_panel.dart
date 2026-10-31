import 'package:flutter/material.dart';

import '../../core/panels/panel_flow.dart';
import 'program_detail_page.dart';

/// The Programa panel's body (R-0061 AC8, SPEC-0061 §2.6): the current
/// program, and its choose flow — the body-type picker, then the proposals —
/// as pages of one [PanelFlow], so choosing never leaves the panel.
class ProgramPanel extends StatelessWidget {
  const ProgramPanel({super.key});

  @override
  Widget build(BuildContext context) =>
      const PanelFlow(initialPage: ProgramDetailPage());
}
