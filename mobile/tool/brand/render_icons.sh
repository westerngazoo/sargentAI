#!/usr/bin/env bash
# Render the app's web and launcher icons from the two SVGs in this folder.
# macOS only: uses headless Google Chrome to rasterise and `sips` to resize.
# Run from anywhere; it writes into mobile/web and the native icon folders.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
mobile="$(cd "$here/../.." && pwd)"
chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# Rasterise an SVG to a 1024×1024 PNG with an explicit background.
master() {
  local svg="$1" bg="$2" out="$3"
  printf '<html><body style="margin:0;background:%s"><img src="file://%s" width="1024" height="1024" style="display:block"></body></html>' \
    "$bg" "$here/$svg" > "$work/page.html"
  "$chrome" --headless=new --disable-gpu --hide-scrollbars --window-size=1024,1024 \
    --default-background-color=00000000 --user-data-dir="$work/profile" \
    --screenshot="$out" "file://$work/page.html" >/dev/null 2>&1
}

# Resize the master to the pixel width an existing icon file already has.
fit() {
  local src="$1" dst="$2" px
  px="$(sips -g pixelWidth "$dst" | awk '/pixelWidth/ {print $2}')"
  sips -z "$px" "$px" "$src" --out "$dst" >/dev/null
}

master goose-badge.svg transparent "$work/badge.png"
master goose-fullbleed.svg '#39511F' "$work/fullbleed.png"

fit "$work/badge.png" "$mobile/web/favicon.png"
fit "$work/badge.png" "$mobile/web/icons/Icon-192.png"
fit "$work/badge.png" "$mobile/web/icons/Icon-512.png"
fit "$work/fullbleed.png" "$mobile/web/icons/Icon-maskable-192.png"
fit "$work/fullbleed.png" "$mobile/web/icons/Icon-maskable-512.png"
for f in "$mobile"/android/app/src/main/res/mipmap-*/ic_launcher.png; do
  fit "$work/badge.png" "$f"
done
# iOS icons must be opaque; the full-bleed master is.
for f in "$mobile"/ios/Runner/Assets.xcassets/AppIcon.appiconset/*.png; do
  fit "$work/fullbleed.png" "$f"
done
echo "icons rendered"
