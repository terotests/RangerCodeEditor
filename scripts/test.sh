#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Runs the editor's suite (tests/ScriptEditorTest.rgr) on es6, Rust and Go
# and checks that every target passes every check and prints the same lines,
# then type-checks the crate with cargo.
#
#   scripts/test.sh [--ranger DIR] [--only es6|rust|go|cargo]
#
# The sources import each other as they would sit in a Ranger checkout at
# gallery/codeeditor; scripts/stage.sh links this repository there.
set -u
HERE="$(cd "$(dirname "$0")/.." && pwd)"
ONLY=""
while [ $# -gt 0 ]; do
  case "$1" in
    --ranger) export RANGER_DIR="$2"; shift 2 ;;
    --only) ONLY="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
RANGER="$("$HERE/scripts/stage.sh")" || exit 1
cd "$RANGER"
export RANGER_LIB=./compiler/Lang.rgr:./lib/stdops.rgr
OUT="$RANGER/tmp/codeeditor-test"
SRC=gallery/codeeditor/tests/ScriptEditorTest.rgr
fail=0

compile() { # lang outdir file
  rm -rf "$2"; mkdir -p "$2"
  local log
  log=$(node --max-old-space-size=8192 dist/rgrc.js "-l=$1" "$SRC" -nodecli "-d=$2" "-o=$3" 2>&1)
  if echo "$log" | grep -q "\[FAIL\]" || [ ! -f "$2/$3" ]; then
    echo "$log" | grep -A3 "\[FAIL\]" | head -40
    echo "FAIL: $1 does not compile" >&2
    return 1
  fi
}

check() { # name output
  local summary
  summary=$(echo "$2" | grep -E "^passed [0-9]+, failed [0-9]+" | tail -1)
  if [ -z "$summary" ] || ! echo "$summary" | grep -q "failed 0"; then
    echo "$2" | grep "FAIL" | head -20
    echo "FAIL: $1 (${summary:-no summary})" >&2
    fail=1
    return 1
  fi
  echo "ok: $1 ($summary)"
}

es6=""
want() { [ -z "$ONLY" ] || [ "$ONLY" = "$1" ]; }

if want es6 || want rust || want go; then
  if compile es6 "$OUT/es6" test.js; then
    es6=$(node "$OUT/es6/test.js" 2>&1)
    check es6 "$es6"
  else
    fail=1
  fi
fi

if want rust && command -v rustc >/dev/null; then
  if compile rust "$OUT/rust" test.rs &&
     rustc --edition 2021 -O -o "$OUT/rust/test" "$OUT/rust/test.rs" 2>"$OUT/rust/rustc.log"; then
    out=$("$OUT/rust/test" 2>&1)
    check rust "$out" && { [ "$out" = "$es6" ] || { echo "FAIL: rust output differs from es6" >&2; diff <(echo "$es6") <(echo "$out") | head; fail=1; }; }
  else
    tail -30 "$OUT/rust/rustc.log" 2>/dev/null; echo "FAIL: rust build" >&2; fail=1
  fi
fi

if want go && command -v go >/dev/null; then
  if compile go "$OUT/go" main.go; then
    printf 'module codeeditortest\n\ngo 1.21\n' > "$OUT/go/go.mod"
    if (cd "$OUT/go" && go build -o test . 2>build.log); then
      out=$("$OUT/go/test" 2>&1)
      check go "$out" && { [ "$out" = "$es6" ] || { echo "FAIL: go output differs from es6" >&2; diff <(echo "$es6") <(echo "$out") | head; fail=1; }; }
    else
      tail -30 "$OUT/go/build.log"; echo "FAIL: go build" >&2; fail=1
    fi
  else
    fail=1
  fi
fi

if want cargo && command -v cargo >/dev/null; then
  if (cd "$RANGER/gallery/codeeditor" && RANGER_HOME="$RANGER" cargo check --quiet 2>&1 | tail -40; exit "${PIPESTATUS[0]}"); then
    echo "ok: cargo check"
  else
    echo "FAIL: cargo check" >&2; fail=1
  fi
fi

[ $fail = 0 ] && echo "ALL PASS" || { echo "FAILED" >&2; exit 1; }
