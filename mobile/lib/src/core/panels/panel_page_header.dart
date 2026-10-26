import 'package:flutter/material.dart';

/// The header of a page past the first in a `PanelFlow`: a back arrow, which
/// steps back within the flow, and the page's title.
class PanelPageHeader extends StatelessWidget {
  const PanelPageHeader({super.key, required this.title});

  /// The page's title, from the catalogue.
  final String title;

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.fromLTRB(4, 0, 16, 4),
        child: Row(
          children: [
            const BackButton(),
            Expanded(
              child:
                  Text(title, style: Theme.of(context).textTheme.titleMedium),
            ),
          ],
        ),
      );
}
