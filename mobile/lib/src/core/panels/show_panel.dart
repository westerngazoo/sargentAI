import 'package:flutter/material.dart';

import '../theme/tokens.dart';

/// Width from which a panel slides in from the right instead of rising as a
/// bottom sheet (SPEC-0061 §2.6).
const panelBreakpoint = 720.0;

/// Opens [body] over the main screen (R-0061 AC1, AC8).
///
/// Below [panelBreakpoint] it is a bottom sheet; from it, a 480 px side
/// panel. Both close on the ✕ in the title row, on a tap outside, on Esc
/// (the route's own `DismissIntent`) and on Android back. The route under it
/// is never replaced, so the selected day survives.
///
/// The frame scrolls the body. With [scrolls] false the body fills the frame
/// and scrolls itself, which a `PanelFlow` needs, since its pages scroll on
/// their own; on a phone that sheet has a fixed height instead of a draggable
/// one. The sheet also rises above the keyboard, so a form's fields and
/// button stay reachable.
///
/// A tap outside pops the panel route itself, never `maybePop` (SPEC-0061
/// §2.6, N11): from a nested flow's second page `maybePop` would only step
/// back. Esc and back do go through `maybePop`, so they step back first.
Future<T?> showPanel<T>(
  BuildContext context, {
  required String title,
  required WidgetBuilder body,
  bool scrolls = true,
}) {
  final navigator = Navigator.of(context, rootNavigator: true);
  final wide = MediaQuery.sizeOf(context).width >= panelBreakpoint;
  if (!wide) {
    final localizations = MaterialLocalizations.of(context);
    return navigator.push(_SheetPanelRoute<T>(
      barrierLabel: localizations.scrimLabel,
      barrierOnTapHint:
          localizations.scrimOnTapHint(localizations.bottomSheetLabel),
      builder: (context) => Padding(
        padding:
            EdgeInsets.only(bottom: MediaQuery.viewInsetsOf(context).bottom),
        child: scrolls
            ? DraggableScrollableSheet(
                expand: false,
                initialChildSize: 0.7,
                minChildSize: 0.4,
                maxChildSize: 1,
                builder: (context, controller) => _PanelFrame(
                  title: title,
                  scrolls: true,
                  scrollController: controller,
                  child: body(context),
                ),
              )
            : FractionallySizedBox(
                heightFactor: 0.9,
                child: _PanelFrame(
                  title: title,
                  scrolls: false,
                  child: body(context),
                ),
              ),
      ),
    ));
  }
  return navigator.push(_SidePanelRoute<T>(
    barrierLabel: title,
    pageBuilder: (context, _, __) => Align(
      alignment: Alignment.centerRight,
      child: SizedBox(
        width: 480,
        height: double.infinity,
        child: Material(
          color: Theme.of(context).extension<FbfTokens>()?.panel,
          child: SafeArea(
            child: _PanelFrame(
              title: title,
              scrolls: scrolls,
              child: body(context),
            ),
          ),
        ),
      ),
    ),
    transitionBuilder: (context, animation, _, child) => SlideTransition(
      position: Tween(begin: const Offset(1, 0), end: Offset.zero)
          .animate(CurvedAnimation(parent: animation, curve: Curves.easeOut)),
      child: child,
    ),
  ));
}

/// The barrier both panel routes share: a tap on it pops the panel route.
/// The default barrier calls `maybePop`, which a nested flow's
/// `NavigatorPopHandler` turns into a step back (SPEC-0061 §2.6, N11).
Widget _wholePanelBarrier(ModalRoute<Object?> route, Color color) =>
    AnimatedModalBarrier(
      color: route.animation!.drive(
        ColorTween(begin: color.withValues(alpha: 0), end: color)
            .chain(CurveTween(curve: route.barrierCurve)),
      ),
      dismissible: true,
      semanticsLabel: route.barrierLabel,
      barrierSemanticsDismissible: route.semanticsDismissible,
      onDismiss: () {
        if (route.isCurrent) route.navigator?.pop();
      },
    );

/// The side panel from [panelBreakpoint] up: a dialog route whose barrier
/// closes the whole panel.
class _SidePanelRoute<T> extends RawDialogRoute<T> {
  _SidePanelRoute({
    required super.pageBuilder,
    required String super.barrierLabel,
    required super.transitionBuilder,
  }) : super(
          barrierColor: Colors.black54,
          transitionDuration: const Duration(milliseconds: 220),
        );

  @override
  Widget buildModalBarrier() => _wholePanelBarrier(this, barrierColor!);
}

/// The bottom sheet below [panelBreakpoint]: `showModalBottomSheet`'s route
/// with a barrier that closes the whole panel.
class _SheetPanelRoute<T> extends ModalBottomSheetRoute<T> {
  _SheetPanelRoute({
    required super.builder,
    required super.barrierLabel,
    required super.barrierOnTapHint,
  }) : super(isScrollControlled: true, useSafeArea: true);

  @override
  Widget buildModalBarrier() => _wholePanelBarrier(this, barrierColor);
}

/// Title row with the ✕, then the body: in its own scroll view, or filling
/// the rest of the frame when it scrolls itself.
class _PanelFrame extends StatelessWidget {
  const _PanelFrame({
    required this.title,
    required this.child,
    required this.scrolls,
    this.scrollController,
  });

  final String title;
  final Widget child;
  final bool scrolls;
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
          child: scrolls
              ? SingleChildScrollView(
                  controller: scrollController,
                  padding: const EdgeInsets.fromLTRB(20, 4, 20, 24),
                  child: child,
                )
              : child,
        ),
      ],
    );
  }
}
