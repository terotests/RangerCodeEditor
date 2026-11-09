#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Puts this repository where its imports expect it, at gallery/codeeditor of
# a Ranger checkout, and prints that checkout's path.
#
#   Ranger   $RANGER_DIR, the enclosing checkout when this repository already
#            sits at gallery/codeeditor, or ../Ranger.
#
# lib/evg is fetched by Ranger's own scripts/deps.mjs when it is missing.
set -e
HERE="$(cd "$(dirname "$0")/.." && pwd)"
if [ -n "${RANGER_DIR:-}" ]; then
  RANGER="$(cd "$RANGER_DIR" && pwd)"
elif [ -f "$HERE/../../dist/rgrc.js" ]; then
  RANGER="$(cd "$HERE/../.." && pwd)"
else
  RANGER="$(cd "$HERE/../Ranger" 2>/dev/null && pwd)" || { echo "no Ranger checkout: set RANGER_DIR" >&2; exit 1; }
fi
[ -f "$RANGER/dist/rgrc.js" ] || { echo "not a Ranger checkout: $RANGER" >&2; exit 1; }
link="$RANGER/gallery/codeeditor"
if [ "$(cd "$link" 2>/dev/null && pwd -P)" != "$(cd "$HERE" && pwd -P)" ]; then
  if [ -L "$link" ] || [ ! -e "$link" ]; then
    ln -sfn "$HERE" "$link"
  else
    echo "$link exists and is not this repository" >&2; exit 1
  fi
fi
if [ ! -f "$RANGER/lib/evg/EVGDisplayList.rgr" ]; then
  (cd "$RANGER" && node scripts/deps.mjs >&2)
fi
echo "$RANGER"
