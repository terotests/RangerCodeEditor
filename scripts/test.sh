#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Runs the editor's suite (tests/ScriptEditorTest.rgr) on es6 and Go and
# checks that every target passes every check and prints the same lines, then
# type-checks the crate with cargo.
#
#   scripts/test.sh [--ranger DIR] [--only es6|go|cargo|rust] [--rust]
#
# --rust also runs the suite through Ranger's Rust writer (.rs -> Ranger ->
# Rust -> rustc, the path MFilesAPI's Rust WASM window takes); it needs a
# Ranger with PR #1303 (the struct wrapped into `dyn EditorLanguage`'s enum).
#
# The sources import each other as they would sit in a Ranger checkout at
# gallery/codeeditor; scripts/stage.sh links this repository there.
set -u
HERE="$(cd "$(dirname "$0")/.." && pwd)"
ONLY=""
RUST=0
while [ $# -gt 0 ]; do
  case "$1" in
    --ranger) export RANGER_DIR="$2"; shift 2 ;;
    --only) ONLY="$2"; shift 2 ;;
    --rust) RUST=1; shift ;;
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
want() {
  if [ "$1" = rust ] && [ -z "$ONLY" ]; then [ $RUST = 1 ]; return; fi
  [ -z "$ONLY" ] || [ "$ONLY" = "$1" ]
}

if want es6 || want rust || want go; then
  if compile es6 "$OUT/es6" test.js; then
    es6=$(node "$OUT/es6/test.js" 2>&1)
    check es6 "$es6"
  else
    fail=1
  fi
fi

if want rust && command -v rustc >/dev/null; then
  # EVGHostTextMeasurer's boxed closure: Ranger's Rust writer cannot write
  # it yet, so it is patched out as build.rs does for the crate.
  if compile rust "$OUT/rust" test.rs &&
     sed -i -e 's/^    self\.metric_fn = Some(f);$/    let _ = f;/' \
       -e 's/self\.metric_fn\.clone()\.unwrap();/\&mut |_: i64, _: String, _: String, _: f64, _: bool, _: bool| -> f64 { 0.0 };/' "$OUT/rust/test.rs" &&
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
  # cargo resolves the manifest's ../../runtime from the real path, so the
  # crate is checked from a copy that really sits in the gallery when
  # gallery/codeeditor is a link to this repository.
  crate="$RANGER/gallery/codeeditor"
  if [ -L "$crate" ]; then
    crate="$RANGER/gallery/.codeeditor-cargo"
    mkdir -p "$crate"
    cp "$HERE/Cargo.toml" "$HERE/build.rs" "$crate/"
    rm -rf "$crate/src"; cp -r "$HERE/src" "$crate/src"
  fi
  if (cd "$crate" && RANGER_HOME="$RANGER" cargo check --quiet 2>&1 | grep -v "^warning" | tail -40; exit "${PIPESTATUS[0]}"); then
    echo "ok: cargo check"
  else
    echo "FAIL: cargo check" >&2; fail=1
  fi
fi

[ $fail = 0 ] && echo "ALL PASS" || { echo "FAILED" >&2; exit 1; }
