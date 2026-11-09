#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Builds the editor's page: static files any server can serve, no host
# process. The editor IS the page.
#
#   scripts/build-web.sh [--ranger DIR] [--out DIR]     (default out: dist/)
#
# The page is web/code_editor_web.rgr (the browser's side of the input loop)
# over the crate in src/, compiled to one script by Ranger's committed
# compiler (dist/rgrc.js), with EVG's WebGL renderer and the font faces.
set -e
HERE="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$HERE/dist"
while [ $# -gt 0 ]; do
  case "$1" in
    --ranger) export RANGER_DIR="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
RANGER="$("$HERE/scripts/stage.sh")"
cd "$RANGER"
export RANGER_LIB=./compiler/Lang.rgr:./lib/stdops.rgr

# A previous build's bundle must not survive this one: the check below asks
# whether a bundle is loadable, and a stale file answers yes.
rm -f "$OUT/code_editor_web.js"
log=$(node --max-old-space-size=8192 dist/rgrc.js -es6 gallery/codeeditor/web/code_editor_web.rgr -d="$OUT" -o=code_editor_web.js 2>&1)
if echo "$log" | grep -q "\[FAIL\]" || [ ! -f "$OUT/code_editor_web.js" ]; then
  echo "$log" | grep -A3 "\[FAIL\]" | head -40
  echo "FAILED to compile web/code_editor_web.rgr" >&2
  exit 1
fi

# Loadable by a browser? The bundle keeps the EVG stack's file-reading
# functions (never the path this page takes), so the question is whether
# anything CALLS one while the script loads.
node --input-type=module -e "
  import fs from 'fs';
  globalThis.require = undefined;
  const src = fs.readFileSync('$OUT/code_editor_web.js', 'utf8');
  const found = (0, eval)(src + '; typeof CodeEditorWeb');
  if (found !== 'function') {
    console.error('code_editor_web.js does not define CodeEditorWeb without require()');
    process.exit(1);
  }
"
# The page loads this beside other scripts, so it publishes one name.
node --input-type=module -e "
  import fs from 'fs';
  const p = '$OUT/code_editor_web.js';
  const src = fs.readFileSync(p, 'utf8');
  fs.writeFileSync(p, '// scoped: publishes CodeEditorWeb only.\n(function () {\n' + src + '\n;globalThis.CodeEditorWeb = CodeEditorWeb;\n})();\n');
"

cp "$HERE/web/index.html" "$HERE/web/editor.mjs" "$HERE/web/keyboard.mjs" "$OUT/"
mkdir -p "$OUT/gl" "$OUT/fonts"
cp lib/evg/gl/evg-webgl.js "$OUT/gl/evg-webgl.js"
FONTS=gallery/pdf_writer/assets/fonts
cp "$FONTS/Open_Sans/OpenSans-Regular.ttf" "$FONTS/Open_Sans/OpenSans-Bold.ttf" \
   "$FONTS/Noto_Emoji/NotoEmoji-Regular.ttf" "$FONTS/Noto_Sans/NotoSans-Regular.ttf" \
   "$FONTS/El_Messiri/ElMessiri-Regular.ttf" "$FONTS/El_Messiri/ElMessiri-Bold.ttf" "$OUT/fonts/"
for f in "$FONTS/Open_Sans/OFL.txt" "$FONTS/Open_Sans/LICENSE.txt"; do
  [ -f "$f" ] && cp "$f" "$OUT/fonts/OpenSans-LICENSE.txt" && break
done
touch "$OUT/.nojekyll"
echo "built $OUT (serve it with: python3 -m http.server -d $OUT 8001)"
