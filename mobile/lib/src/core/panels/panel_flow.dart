import 'package:flutter/material.dart';

/// A flow of pages inside one panel (SPEC-0061 §2.6): a nested [Navigator]
/// opened on [initialPage]. A page pushes the next as a [PanelFlowRoute] on
/// `Navigator.of` its own context, the nested one.
///
/// Back and Esc step back within the flow and reach the panel only from its
/// first page: [NavigatorPopHandler] answers the panel route's pop with the
/// nested navigator's while that one has a page to pop. A tap outside is the
/// panel's own business — its barrier pops the panel route (N11).
class PanelFlow extends StatefulWidget {
  const PanelFlow({super.key, required this.initialPage});

  /// The flow's first page.
  final Widget initialPage;

  @override
  State<PanelFlow> createState() => _PanelFlowState();
}

class _PanelFlowState extends State<PanelFlow> {
  final _navigator = GlobalKey<NavigatorState>();

  @override
  Widget build(BuildContext context) => NavigatorPopHandler(
        onPopWithResult: (_) => _navigator.currentState?.maybePop(),
        child: Navigator(
          key: _navigator,
          onGenerateRoute: (_) =>
              PanelFlowRoute<void>(builder: (_) => widget.initialPage),
        ),
      );
}

/// The route of every page in a [PanelFlow].
///
/// `Actions.maybeInvoke` stops at the first `Actions` scope mapping the
/// intent's type and drops a disabled action; a page route's own
/// `DismissIntent` action is disabled, so Esc would end there. Each page
/// therefore maps `DismissIntent` itself, below the focused subtree, so its
/// scope is found first: `maybePop` on the root navigator, which steps back
/// within the flow or closes the panel from its first page. The page keeps
/// focus inside with its own `FocusScope`.
class PanelFlowRoute<T> extends MaterialPageRoute<T> {
  PanelFlowRoute({required WidgetBuilder builder})
      : super(
          builder: (context) => Actions(
            actions: {
              DismissIntent: CallbackAction<DismissIntent>(
                onInvoke: (_) =>
                    Navigator.of(context, rootNavigator: true).maybePop(),
              ),
            },
            child: FocusScope(autofocus: true, child: builder(context)),
          ),
        );
}
