# RangerCodeEditor

A code editor that paints into an **EVG display list**, written in
[Ranger](https://github.com/terotests/Ranger)'s Rust syntax. The same `.rs`
files compile with Ranger to JavaScript and Go (and the other Ranger
targets), and with cargo as a Rust crate.

Live: <https://terotests.github.io/RangerCodeEditor/>

It came from EVGSheets (`datagrid/src/script`, commit
[`ab5dd03`](https://github.com/terotests/EVGSheets/tree/ab5dd0340a8290732f27af03833e13d5b71876f5/datagrid/src/script)),
where it was `.rgr`. The first commit here is that code as it was; the
commits after it convert it file by file. EVGSheets and MFilesAPI still use
their own copy; moving them to this repository is a separate step.

```text
                 CodeEditorPage::buildDisplayList()
                               │
                        EVGDisplayList
                               │
                  gl/evg-webgl.js (WebGL 2)   ← the page in web/
```

## What is in it

| Piece | File | What it does |
| --- | --- | --- |
| Tokens | [`src/code_tokens.rs`](src/code_tokens.rs) | the coloured-run record every lexer produces |
| Lexers | [`src/js_tokens.rs`](src/js_tokens.rs), [`src/ranger_tokens.rs`](src/ranger_tokens.rs) | JavaScript/TSX and Ranger, one line at a time, with state carried across lines |
| Language plugins | [`src/editor_language.rs`](src/editor_language.rs) | the `EditorLanguage` trait, `LanguageRegistry`, `PlainText`, shared `LanguageText` helpers |
| | [`src/js_language.rs`](src/js_language.rs) | JavaScript/TSX: structure checks per keystroke, a real parse (Ranger's `TSParserSimple`) for the full pass, completions |
| | [`src/ranger_language.rs`](src/ranger_language.rs) | Ranger: bracket and string checks, the common mistakes, completions |
| Diagnostics, completions | [`src/editor_diagnostic.rs`](src/editor_diagnostic.rs), [`src/completion.rs`](src/completion.rs) | LSP-shaped records |
| Editor panel | [`src/script_editor.rs`](src/script_editor.rs) | gutter, current line, selection, caret, typing, undo, find, completions popup, squiggles, minimap |
| Page | [`src/code_editor_page.rs`](src/code_editor_page.rs) | header, panel, status bar, input loop, sample documents |
| Browser host | [`web/code_editor_web.rgr`](web/code_editor_web.rgr), [`web/editor.mjs`](web/editor.mjs) | browser events → `UIInput`, scene → `evg-webgl.js`, the hidden textarea for screen readers |

The text model comes from Ranger's `gallery/text_editor` (`EditorBuffer`,
`EditorSelection`, `EditorLayout`): a buffer with undo, a codepoint-aware
selection, and a layout that measures with a real TTF. Those and the other
`.rgr` classes the crate uses are listed once in [`src/rgr.rs`](src/rgr.rs)
and [`src/CeRgr.rgr`](src/CeRgr.rgr). `src/CeNative.rgr` gives the `.rs`
code `strlen` / `charAt` / `substring` in each target's own string units.

A third language is one more `impl EditorLanguage for X` and a
`registry.add(Rc::new(RefCell::new(X::new())))`; nothing in the editor
changes.

## Building

The sources import each other as they sit in a Ranger checkout at
`gallery/codeeditor`. Clone this repository there, or next to a Ranger
checkout (`../Ranger`), or point `RANGER_DIR` at one;
[`scripts/stage.sh`](scripts/stage.sh) links it into place and fetches
`lib/evg` with Ranger's `scripts/deps.mjs` when it is missing. Node 22 is
needed; Go and cargo for their parts of the test.

```bash
scripts/test.sh                  # suite on es6 and Go (same output), cargo check
scripts/test.sh --rust           # also through Ranger's Rust writer (.rs -> Ranger -> Rust)
scripts/build-web.sh             # the page, into dist/
python3 -m http.server -d dist 8001
npm install && node web/smoke.mjs --dist dist       # type into it in headless Chrome
node web/keyboard.mjs --dist dist                   # keyboard, focus and ARIA, 83 checks
```

As a crate, from a Rust program in the same Ranger checkout:

```toml
[dependencies]
rangercodeeditor = { path = "../codeeditor" }
```

`build.rs` compiles the `.rgr` classes in `src/CeRgr.rgr` into a Rust module
with Ranger's compiler (node, `RANGER_HOME` or the enclosing checkout).

From `.rgr` code, `Import "../codeeditor/src/lib.rs"` gives every type. On
the `.rgr` side a trait object is `DynEditorLanguage`, and its methods are
called as `DynEditorLanguage_ops.id(lang)` (see the registry checks in the
test).

## Keys

| | |
| --- | --- |
| click / drag | place the caret, select |
| click / drag the minimap | scroll in proportion |
| arrows, Home/End, PgUp/PgDn | move (Shift extends) |
| Enter | newline, keeping the indent, one level more after `{`, `(`, `[` |
| Tab / Shift+Tab | indent / outdent the line or the selected lines |
| Ctrl+A / Ctrl+Z / Ctrl+Y | select all, undo, redo |
| Ctrl+C / Ctrl+X / Ctrl+V | copy, cut, paste |
| Ctrl+Space | suggest; arrows choose, Enter accepts, Escape closes |
| Ctrl+F, Ctrl+G | find, next match |
| Ctrl+E | jump to the next problem |
| double / triple click | select the word / the line |
| Ctrl+K / Ctrl+L | next sample document / reload it |

## Tests

- `tests/ScriptEditorTest.rgr`: 188 checks on the lexers, the language
  plugins, the registry, completions and the editing model. It is `.rgr`
  because it is the program's entry; everything it tests is `.rs`.
- `web/smoke.mjs`: the page in headless Chrome types, undoes and switches
  documents, and checks that the scene reaching WebGL has text in more than
  one colour (12 checks).
- `web/keyboard.mjs`: a real keyboard through Playwright: focus order,
  ARIA, the screen reader textarea, completions, find (83 checks).

GitHub Actions ([`.github/workflows/pages.yml`](.github/workflows/pages.yml))
runs all of them on every push and publishes `main` to GitHub Pages.

## Known gaps

- `EVGHostTextMeasurer`'s optional boxed closure is also beyond that writer;
  `build.rs` patches it out of the generated module, as MFilesAPI's builds do.
- Ranger's es6 writer copies a doc comment into `/** … */` unescaped, so a
  doc comment here must not contain `*/`.
- `web/keyboard.mjs`'s idle-frame check (at least 5 animation frames in 2 s)
  failed in about one run of three on this build in a container, where the
  `.rgr` original passed 3 of 3. It is not blocking in CI until that is
  understood.
- The SDL host and the CPU (SoftPainter) render of the EVGSheets copy are
  not here: both depend on the datagrid.

## License

AGPL-3.0-or-later, as EVGSheets. The fonts the page serves are under their
own licenses (Open Sans, Noto: SIL OFL 1.1; El Messiri: OFL).
