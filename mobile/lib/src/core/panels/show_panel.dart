import 'package:flutter/material.dart';

import '../theme/tokens.dart';

/// Width from which a panel slides in from the right instead of rising as a
/// bottom sheet (SPEC-0061 §2.6).
const panelBreakpoint = 720.0;

/// Opens [body] over the main screen (R-0061 AC1, AC8).
///
/// Below [panelBreakpoint] it is a draggable bottom sheet; from it, a 480 px
/// side panel. Both close on the ✕ in the title row, on a tap outside, on
/// Esc (the route's own `DismissIntent`) and on Android back. The route under
/// it is never replaced, so the selected day survives.
Future<T?> showPanel<T>(
  BuildContext context, {
  required String title,
  required WidgetBuilder body,
}) {
  final wide = MediaQuery.sizeOf(context).width >= panelBreakpoint;
  if (!wide) {
    return showModalBottomSheet<T>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      builder: (context) => DraggableScrollableSheet(
        expand: false,
        initialChildSize: 0.7,
        minChildSize: 0.4,
        maxChildSize: 1,
        builder: (context, controller) => _PanelFrame(
          title: title,
          scrollController: controller,
          child: body(context),
        ),
      ),
    );
  }
  return showGeneralDialog<T>(
    context: context,
    barrierDismissible: true,
    barrierLabel: title,
    barrierColor: Colors.black54,
    transitionDuration: const Duration(milliseconds: 220),
    pageBuilder: (context, _, __) => Align(
      alignment: Alignment.centerRight,
      child: SizedBox(
        width: 480,
        height: double.infinity,
        child: Material(
          color: Theme.of(context).extension<FbfTokens>()?.panel,
          child:
              SafeArea(child: _PanelFrame(title: title, child: body(context))),
        ),
      ),
    ),
    transitionBuilder: (context, animation, _, child) => SlideTransition(
      position: Tween(begin: const Offset(1, 0), end: Offset.zero)
          .animate(CurvedAnimation(parent: animation, curve: Curves.easeOut)),
      child: child,
    ),
  );
}

/// Title row with the ✕, then the body in its own scroll view.
class _PanelFrame extends StatelessWidget {
  const _PanelFrame({
    required this.title,
    required this.child,
    this.scrollController,
  });

  final String title;
  final Widget child;
  final ScrollController? scrollController;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(20, 12, 8, 4),
          child: Row(
            children: [
              Expanded(child: Text(title, style: text.titleLarge)),
              IconButton(
                key: const Key('panel-close'),
                icon: const Icon(Icons.close),
                tooltip: MaterialLocalizations.of(context).closeButtonTooltip,
                onPressed: () => Navigator.of(context).pop(),
              ),
            ],
          ),
        ),
        Expanded(
          child: SingleChildScrollView(
            controller: scrollController,
            padding: const EdgeInsets.fromLTRB(20, 4, 20, 24),
            child: child,
          ),
        ),
      ],
    );
  }
}
