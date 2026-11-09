// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::code_tokens::CodeToken;
use crate::code_tokens::CodeTokenKind;
use crate::code_tokens::CodeTokenLine;
use crate::completion::Completion;
use crate::completion::CompletionKind;
use crate::editor_diagnostic::DiagnosticSeverity;
use crate::editor_diagnostic::EditorDiagnostic;
use crate::editor_language::EditorLanguage;
use crate::editor_language::PlainText;
use crate::editor_language::LanguageText;
use crate::rgr::CeNative;
use crate::rgr::{EVGColor, EVGDisplayList, EVGDrawCmd, EditorBuffer, EditorLayout, EditorPos, EditorSelection, EditorWord, UIKey, UITextRenderer};

/// ==============================================================================
/// ScriptEditor.rgr — a code editor panel, drawn into the grid's display list
/// ==============================================================================
///
/// The text model is not written here. `gallery/text_editor` already has a
/// buffer with undo, a selection with codepoint-aware movement, and a layout
/// that measures with a real TTF and hit-tests a click back to a column — so
/// this uses those three and adds what a CODE editor needs on top: tokens with
/// colours, a gutter with line numbers, a current-line band, and typing that
/// keeps its indent.
///
/// It paints into the display list it is given, at the rectangle it is given,
/// with the TEXT RENDERER IT IS GIVEN — the grid's own. That is what makes it a
/// panel rather than an application: no canvas of its own, no font of its own,
/// no frame of its own. The same list goes to SoftCanvas on a desktop host and
/// to WebGL in a browser, so the editor is on the GPU for free. The right edge
/// is a minimap: a scaled picture of the file, like VS Code's, that you scroll
/// with a click.
///
/// Every token is positioned by `layout.caretPixelX`, the same call the caret
/// and the selection use, rather than by adding up token widths as it goes: two
/// ways of measuring the same prefix is how a caret ends up half a character
/// away from the letter it is supposed to be in front of.
/// ==============================================================================
pub struct ScriptEditorTheme {
    pub(crate) bg: Rc<RefCell<EVGColor>>,
    pub(crate) gutterBg: Rc<RefCell<EVGColor>>,
    pub(crate) gutterText: Rc<RefCell<EVGColor>>,
    pub(crate) gutterActive: Rc<RefCell<EVGColor>>,
    pub(crate) currentLine: Rc<RefCell<EVGColor>>,
    pub(crate) selection: Rc<RefCell<EVGColor>>,
    pub(crate) caret: Rc<RefCell<EVGColor>>,
    pub(crate) border: Rc<RefCell<EVGColor>>,
    /// Token colours.
    pub(crate) plain: Rc<RefCell<EVGColor>>,
    pub(crate) keyword: Rc<RefCell<EVGColor>>,
    pub(crate) builtin: Rc<RefCell<EVGColor>>,
    pub(crate) number: Rc<RefCell<EVGColor>>,
    pub(crate) text: Rc<RefCell<EVGColor>>,
    pub(crate) comment: Rc<RefCell<EVGColor>>,
    pub(crate) punct: Rc<RefCell<EVGColor>>,
    pub(crate) tag: Rc<RefCell<EVGColor>>,
    pub(crate) attr: Rc<RefCell<EVGColor>>,
    /// The completion popup.
    pub(crate) popupBg: Rc<RefCell<EVGColor>>,
    pub(crate) popupEdge: Rc<RefCell<EVGColor>>,
    pub(crate) popupPick: Rc<RefCell<EVGColor>>,
    pub(crate) popupDetail: Rc<RefCell<EVGColor>>,
    /// The find bar.
    pub(crate) findBg: Rc<RefCell<EVGColor>>,
    pub(crate) findEdge: Rc<RefCell<EVGColor>>,
    pub(crate) findInk: Rc<RefCell<EVGColor>>,
    pub(crate) findDim: Rc<RefCell<EVGColor>>,
    /// Diagnostics.
    pub(crate) errorInk: Rc<RefCell<EVGColor>>,
    pub(crate) warningInk: Rc<RefCell<EVGColor>>,
    /// Diff marks: a band across an added or a removed line and a strip in
    /// the gutter, so a change below the fold still shows in the margin.
    pub(crate) addedLine: Rc<RefCell<EVGColor>>,
    pub(crate) removedLine: Rc<RefCell<EVGColor>>,
    pub(crate) addedMark: Rc<RefCell<EVGColor>>,
    pub(crate) removedMark: Rc<RefCell<EVGColor>>,
    /// The minimap on the right: a darker strip, a slider over the visible
    /// range, and a hairline where the text ends.
    pub(crate) minimapBg: Rc<RefCell<EVGColor>>,
    pub(crate) minimapSlider: Rc<RefCell<EVGColor>>,
    pub(crate) minimapEdge: Rc<RefCell<EVGColor>>,
    /// Horizontal scrollbar, the same idea as the minimap's vertical slider:
    /// a track along the foot of the text and a thumb for how far along the
    /// longest line you have scrolled.
    pub(crate) hBarTrack: Rc<RefCell<EVGColor>>,
    pub(crate) hBarThumb: Rc<RefCell<EVGColor>>,
    pub(crate) hBarEdge: Rc<RefCell<EVGColor>>,
}

impl ScriptEditorTheme {
    pub fn new() -> Self {
        Self {
            bg: EVGColor::rgb(26, 27, 38),
            gutterBg: EVGColor::rgb(22, 22, 30),
            gutterText: EVGColor::rgb(59, 66, 97),
            gutterActive: EVGColor::rgb(115, 122, 162),
            currentLine: EVGColor::rgb(31, 34, 51),
            selection: EVGColor::rgba(47, 75, 124, 0.75),
            caret: EVGColor::rgb(122, 162, 247),
            border: EVGColor::rgb(41, 46, 66),
            plain: EVGColor::rgb(192, 202, 245),
            keyword: EVGColor::rgb(187, 154, 247),
            builtin: EVGColor::rgb(125, 207, 255),
            number: EVGColor::rgb(255, 158, 100),
            text: EVGColor::rgb(158, 206, 106),
            comment: EVGColor::rgb(86, 95, 137),
            punct: EVGColor::rgb(137, 221, 255),
            tag: EVGColor::rgb(247, 118, 142),
            attr: EVGColor::rgb(224, 175, 104),
            popupBg: EVGColor::rgb(36, 40, 59),
            popupEdge: EVGColor::rgb(65, 72, 104),
            popupPick: EVGColor::rgb(52, 63, 102),
            popupDetail: EVGColor::rgb(118, 128, 154),
            findBg: EVGColor::rgb(36, 40, 59),
            findEdge: EVGColor::rgb(65, 72, 104),
            findInk: EVGColor::rgb(192, 202, 245),
            findDim: EVGColor::rgb(118, 128, 154),
            errorInk: EVGColor::rgb(247, 118, 142),
            warningInk: EVGColor::rgb(224, 175, 104),
            addedLine: EVGColor::rgba(34, 197, 94, 0.18),
            removedLine: EVGColor::rgba(239, 68, 68, 0.18),
            addedMark: EVGColor::rgb(34, 197, 94),
            removedMark: EVGColor::rgb(239, 68, 68),
            minimapBg: EVGColor::rgb(22, 23, 32),
            minimapSlider: EVGColor::rgba(155, 168, 210, 0.1),
            minimapEdge: EVGColor::rgb(41, 46, 66),
            hBarTrack: EVGColor::rgb(22, 23, 32),
            hBarThumb: EVGColor::rgba(155, 168, 210, 0.45),
            hBarEdge: EVGColor::rgb(41, 46, 66),
        }
    }
    pub(crate) fn forSeverity(&self, severity: i64) -> Rc<RefCell<EVGColor>> {
        if severity == DiagnosticSeverity::error() {
            return self.errorInk.clone();
        }
        self.warningInk.clone()
    }
    pub(crate) fn forKind(&self, kind: i64) -> Rc<RefCell<EVGColor>> {
        if kind == CodeTokenKind::keyword() {
            return self.keyword.clone();
        }
        if kind == CodeTokenKind::builtin() {
            return self.builtin.clone();
        }
        if kind == CodeTokenKind::number() {
            return self.number.clone();
        }
        if kind == CodeTokenKind::text() {
            return self.text.clone();
        }
        if kind == CodeTokenKind::comment() {
            return self.comment.clone();
        }
        if kind == CodeTokenKind::punct() {
            return self.punct.clone();
        }
        if kind == CodeTokenKind::tag() {
            return self.tag.clone();
        }
        if kind == CodeTokenKind::attr() {
            return self.attr.clone();
        }
        self.plain.clone()
    }
}

pub struct ScriptEditor {
    pub(crate) buf: Rc<RefCell<EditorBuffer>>,
    pub(crate) sel: Rc<RefCell<EditorSelection>>,
    pub(crate) layout: Rc<RefCell<EditorLayout>>,
    pub theme: Rc<RefCell<ScriptEditorTheme>>,
    pub lines: Vec<Rc<RefCell<CodeTokenLine>>>,
    /// The buffer version the tokens were made from. Re-lexing 200 lines is
    /// cheap, doing it sixty times a second because nothing changed is not.
    pub tokenVersion: i64,
    pub focused: bool,
    pub fontFamily: String,
    /// Bumped by whoever edits; the owner reads it to know the script is dirty.
    pub dirty: bool,
    /// The language plugin that says what is wrong with this document, and what
    /// it last said. Nothing here knows what a syntax error IS — see
    /// lang/EditorLanguage.rgr.
    pub language: Rc<RefCell<dyn EditorLanguage>>,
    pub diagnostics: Vec<Rc<RefCell<EditorDiagnostic>>>,
    /// Per line: 0 plain, 1 added, 2 removed. Shorter than the buffer means
    /// the rest is plain. Set by a host showing a merged diff view; cleared
    /// by `init`, because the marks belong to one text.
    pub lineMarks: Vec<i64>,
    /// Two clocks, because there are two passes. The cheap one runs on every
    /// change; the expensive one waits for a pause once it has proved expensive.
    pub quickVersion: i64,
    pub diagVersion: i64,
    pub diagnoseUs: i64,
    /// Over this, the full pass stops running on the keystroke path and waits
    /// for the typing to stop. 8 ms is about half a frame.
    pub diagnoseBudgetUs: i64,
    pub idleMs: f64,
    pub lastEditMs: f64,
    /// --- completion -----------------------------------------------------------
    /// The list, whether it is up, which row is picked, and the column the word
    /// being completed starts at — replacing from there is what makes accepting
    /// a suggestion an edit rather than an append.
    pub completions: Vec<Rc<RefCell<Completion>>>,
    pub completionOpen: bool,
    pub completionIndex: i64,
    pub completionStart: i64,
    pub completionLine: i64,
    /// How many rows the popup shows at once.
    pub completionRows: i64,
    /// Whether typing opens the list by itself. A host that wants Ctrl+Space
    /// only turns this off; nothing else changes.
    pub autoComplete: bool,
    /// True while what is shown came from the cheap pass alone — the status bar
    /// can say so rather than implying the document has been fully checked.
    pub quickOnly: bool,
    /// --- find -----------------------------------------------------------------
    /// Ctrl+F. A one-line bar over the top of the text area: the query, and how
    /// many matches there are. Enter goes to the next one, Shift+Enter to the
    /// one before, Escape puts the keys back. The match is the selection, so it
    /// is highlighted and scrolled to by the code that already does both.
    pub findOpen: bool,
    pub findQuery: String,
    pub findOrdinal: i64,
    pub findTotal: i64,
    /// --- minimap --------------------------------------------------------------
    /// A scaled picture of the whole file on the right, the same idea as
    /// VS Code's and Monaco's: each token is a coloured speck, the slider is
    /// the viewport, and a click there is a jump. Hidden when the panel is too
    /// narrow to spare the strip.
    pub showMinimap: bool,
    pub minimapW: i64,
    pub minimapCharW: f64,
    pub minimapDrag: bool,
    /// Horizontal scrollbar along the foot of the text. Hidden while every
    /// line fits; a drag on the thumb, or a sideways wheel, moves `scrollX`.
    pub hBarSize: i64,
    pub hBarDrag: bool,
    pub contentWPx: i64,
    pub contentWVersion: i64,
    /// Last renderer that painted or hit-tested, so a key that must keep the
    /// caret on screen can measure without the host threading one through.
    pub(crate) measureTr: Option<Rc<RefCell<UITextRenderer>>>,
}

impl ScriptEditor {
    pub fn new() -> Self {
        Self {
            buf: Rc::new(RefCell::new(EditorBuffer::new())),
            sel: Rc::new(RefCell::new(EditorSelection::new())),
            layout: Rc::new(RefCell::new(EditorLayout::new())),
            theme: Rc::new(RefCell::new(ScriptEditorTheme::new())),
            lines: Vec::new(),
            tokenVersion: -1,
            focused: true,
            fontFamily: "Open Sans".to_string(),
            dirty: false,
            language: Rc::new(RefCell::new(PlainText::new())),
            diagnostics: Vec::new(),
            lineMarks: Vec::new(),
            quickVersion: -1,
            diagVersion: -1,
            diagnoseUs: 0,
            diagnoseBudgetUs: 8000,
            idleMs: 350.0,
            lastEditMs: 0.0,
            completions: Vec::new(),
            completionOpen: false,
            completionIndex: 0,
            completionStart: 0,
            completionLine: 0,
            completionRows: 8,
            autoComplete: true,
            quickOnly: false,
            findOpen: false,
            findQuery: String::new(),
            findOrdinal: 0,
            findTotal: 0,
            showMinimap: true,
            minimapW: 96,
            minimapCharW: 2.0,
            minimapDrag: false,
            hBarSize: 10,
            hBarDrag: false,
            contentWPx: 0,
            contentWVersion: -1,
            measureTr: None,
        }
    }
    /// A file from disk may have CRLF endings, and a stray \r is a character
    /// the buffer would carry to the end of every line: an extra column the
    /// caret can sit in and a glyph the atlas has no picture for.
    pub fn withoutCarriageReturns(src: &str) -> String {
        let n: i64 = CeNative::str_len(&src);
        let mut has_cr: bool = false;
        let mut i: i64 = 0;
        while i < n {
            if CeNative::char_at(&src, i) == 13 {
                has_cr = true;
                break;
            }
            i += 1;
        }
        if !has_cr {
            return src.to_string();
        }
        let mut out: String = String::new();
        let mut j: i64 = 0;
        while j < n {
            let ch: i64 = CeNative::char_at(&src, j);
            if ch != 13 {
                out.push_str(&char::from_u32(ch as u32).unwrap_or('\0').to_string());
            }
            j += 1;
        }
        out
    }
    /// Tabs, likewise: the atlas has no picture for one, so a file indented
    /// with tabs — most C and C++ — drew a row of empty boxes down the left
    /// margin. Expand to the next four-column stop, which is what a tab means
    /// on screen anyway. Tab the key still inserts spaces (indentSelection).
    pub fn expandTabs(src: &str) -> String {
        let n: i64 = CeNative::str_len(&src);
        let mut has_tab: bool = false;
        let mut i: i64 = 0;
        while i < n {
            if CeNative::char_at(&src, i) == 9 {
                has_tab = true;
                break;
            }
            i += 1;
        }
        if !has_tab {
            return src.to_string();
        }
        let mut out: String = String::new();
        let mut col: i64 = 0;
        let mut j: i64 = 0;
        while j < n {
            let ch: i64 = CeNative::char_at(&src, j);
            if ch == 9 {
                let pad: i64 = 4 - col % 4;
                let mut k: i64 = 0;
                while k < pad {
                    out.push_str(&" ".to_string());
                    k += 1;
                }
                col += pad;
            } else {
                out.push_str(&char::from_u32(ch as u32).unwrap_or('\0').to_string());
                if ch == 10 {
                    col = 0;
                } else {
                    col += 1;
                }
            }
            j += 1;
        }
        out
    }
    pub fn indexOfIn(hay: &str, needle: &str, from: i64) -> i64 {
        let n: i64 = CeNative::str_len(&hay);
        let mut start: i64 = from;
        if start < 0 {
            start = 0;
        }
        if start >= n {
            return -1;
        }
        let at: i64 = CeNative::index_of(&CeNative::substr(&hay, start, n), needle);
        if at < 0 {
            return -1;
        }
        start + at
    }
    /// The last match that starts before `limit`.
    pub fn lastIndexOfIn(hay: &str, needle: &str, limit: i64) -> i64 {
        let mut best: i64 = -1;
        let mut at: i64 = ScriptEditor::indexOfIn(hay, needle, 0);
        while at >= 0 {
            if at >= limit {
                return best;
            }
            best = at;
            at = ScriptEditor::indexOfIn(hay, needle, at + 1);
        }
        best
    }
    /// How much of a glyph cell a character fills. A minimap letter is a 1×1
    /// sprite: a `.` is almost empty (~3 %), an `I` is a thin stem (~10 %),
    /// an `R` is denser (~20 %). That fraction is the speck's opacity, so the
    /// picture reads as type rather than as solid bars.
    pub fn minimapInk(ch: i64) -> f64 {
        if ch == 32 || ch == 9 {
            return 0.0;
        }
        if ch == 46 || ch == 44 {
            return 0.03;
        }
        if (ch == 39 || ch == 34) || ch == 96 {
            return 0.04;
        }
        if ch == 58 || ch == 59 {
            return 0.05;
        }
        if (ch == 45 || ch == 95) || ch == 126 {
            return 0.07;
        }
        if ((ch == 124 || ch == 33) || ch == 47) || ch == 92 {
            return 0.08;
        }
        if ((ch == 73 || ch == 105) || ch == 108) || ch == 49 {
            return 0.1;
        }
        if (((ch == 106 || ch == 116) || ch == 102) || ch == 114) || ch == 74 {
            return 0.1;
        }
        if ((ch == 40 || ch == 41) || ch == 91) || ch == 93 {
            return 0.1;
        }
        if ((ch == 123 || ch == 125) || ch == 60) || ch == 62 {
            return 0.1;
        }
        if (((ch == 35 || ch == 64) || ch == 37) || ch == 38) || ch == 42 {
            return 0.22;
        }
        if (((ch == 77 || ch == 87) || ch == 109) || ch == 119) || ch == 56 {
            return 0.22;
        }
        if (ch >= 65 && ch <= 90) {
            return 0.2;
        }
        if (ch >= 97 && ch <= 122) {
            return 0.15;
        }
        if (ch >= 48 && ch <= 57) {
            return 0.16;
        }
        if ch > 32 && ch < 127 {
            return 0.12;
        }
        0.15
    }
    /// A token that is only spaces would paint a bar where the file is empty,
    /// which is the opposite of an approximation.
    pub fn blankSpan(s: &str, from: i64, __len: i64) -> bool {
        let n: i64 = CeNative::str_len(&s);
        let mut i: i64 = from;
        let mut stop: i64 = from + __len;
        if stop > n {
            stop = n;
        }
        if i >= stop {
            return true;
        }
        while i < stop {
            let ch: i64 = CeNative::char_at(&s, i);
            if ch != 32 && ch != 9 {
                return false;
            }
            i += 1;
        }
        true
    }
    pub fn firstInk(s: &str) -> i64 {
        let n: i64 = CeNative::str_len(&s);
        let mut i: i64 = 0;
        while i < n {
            let ch: i64 = CeNative::char_at(&s, i);
            if ch != 32 && ch != 9 {
                return i;
            }
            i += 1;
        }
        n
    }
    pub fn lastInk(s: &str) -> i64 {
        let n: i64 = CeNative::str_len(&s);
        let mut i: i64 = n;
        while i > 0 {
            let ch: i64 = CeNative::char_at(&s, i - 1);
            if ch != 32 && ch != 9 {
                return i;
            }
            i -= 1;
        }
        0
    }
    /// How many leading characters one outdent step removes: a tab, or up to
    /// two spaces — the same unit Tab inserts.
    pub fn leadingOutdent(line: &str) -> i64 {
        let n: i64 = CeNative::str_len(&line);
        if n == 0 {
            return 0;
        }
        if CeNative::char_at(&line, 0) == 9 {
            return 1;
        }
        let mut i: i64 = 0;
        while i < 2 && i < n {
            if CeNative::char_at(&line, i) == 32 {
                i += 1;
            } else {
                break;
            }
        }
        i
    }
    pub fn adjustIndentCol(col: i64, delta: i64) -> i64 {
        let next: i64 = col + delta;
        if next < 0 {
            return 0;
        }
        next
    }
    pub fn setLineMarks(&mut self, marks: Vec<i64>) {
        self.lineMarks = marks.clone();
    }
    pub fn lineMark(&self, line: i64) -> i64 {
        if line < 0 {
            return 0;
        }
        if line >= (self.lineMarks.len() as i64) {
            return 0;
        }
        self.lineMarks[line as usize]
    }
    pub fn markCount(&self, kind: i64) -> i64 {
        let mut n: i64 = 0;
        let mut i: i64 = 0;
        while i < (self.lineMarks.len() as i64) {
            if self.lineMarks[i as usize] == kind {
                n += 1;
            }
            i += 1;
        }
        n
    }
    pub fn init(&mut self, source: &str) {
        let mut none: Vec<i64> = Vec::new();
        self.lineMarks = none.clone();
        let arg_0 = ScriptEditor::expandTabs(&ScriptEditor::withoutCarriageReturns(source)).clone();
        EditorBuffer::set_text(&self.buf.clone(), &arg_0);
        EditorSelection::set_caret(&self.sel.clone(), 0, 0);
        EditorSelection::collapse_to_caret(&self.sel.clone());
        self.layout.borrow_mut().scroll_line = 0;
        self.layout.borrow_mut().scroll_x = 0;
        self.contentWPx = 0;
        self.contentWVersion = -1;
        self.layout.borrow_mut().font_size = 13.0;
        self.layout.borrow_mut().show_line_numbers = true;
        self.tokenVersion = -1;
        self.dirty = false;
    }
    pub fn text(&self) -> String {
        return EditorBuffer::get_text(&self.buf);
    }
    pub fn setBounds(&self, bx: i64, by: i64, bw: i64, bh: i64) {
        EditorLayout::set_bounds(&self.layout.clone(), bx, by, bw, bh);
        self.layout.borrow_mut().right_inset = self.minimapWidth();
    }
    pub fn insideRect(&self, px: i64, py: i64) -> bool {
        if px < self.layout.borrow().x {
            return false;
        }
        if py < self.layout.borrow().y {
            return false;
        }
        let right: i64 = self.layout.borrow().x + self.layout.borrow().w;
        if px >= right {
            return false;
        }
        let bottom: i64 = self.layout.borrow().y + self.layout.borrow().h;
        py < bottom
    }
    /// --- diagnostics -----------------------------------------------------------
    pub fn setLanguage(&mut self, lang: Rc<RefCell<dyn EditorLanguage>>) {
        self.language = lang.clone();
        self.tokenVersion = -1;
        self.diagVersion = -1;
        self.quickVersion = -1;
        self.diagnoseUs = 0;
        self.diagnostics.clear();
    }
    /// Re-run the plugin when, and only when, the document changed. The same
    /// rule the tokens follow, for the same reason: a plugin that parses is not
    /// free, and nothing has changed sixty times a second.
    pub fn ensureDiagnostics(&mut self) {
        // The cheap pass, on every change. A bracket that has just lost its
        // partner should be underlined while the finger is still on the key.
        if self.quickVersion != self.buf.borrow().version {
            let _tmp_3550 = self.language.borrow().quickDiagnose(&self.buf.borrow().lines.clone());
            self.diagnostics = _tmp_3550;
            self.quickVersion = self.buf.borrow().version;
            self.quickOnly = true;
        }
        if self.diagVersion == self.buf.borrow().version {
            return;
        }
        // The full pass, if it can be afforded. "Afforded" is measured, not
        // declared: a plugin that answered quickly last time is asked again
        // immediately, and one that did not waits for a gap in the typing.
        let now: f64 = wall_clock_ms();
        let idle_for: f64 = now - self.lastEditMs;
        let cheap: bool = self.diagnoseUs <= self.diagnoseBudgetUs;
        if !cheap {
            if idle_for < self.idleMs {
                return;
            }
        }
        let t0: f64 = wall_clock_ms();
        self.diagnostics = self.language.borrow_mut().diagnose(&EditorBuffer::get_text(&self.buf));
        let t1: f64 = wall_clock_ms();
        let took: f64 = (t1 - t0) * 1000.0;
        self.diagnoseUs = took.floor() as i64;
        self.diagVersion = self.buf.borrow().version;
        self.quickOnly = false;
    }
    pub fn problemCount(&mut self) -> i64 {
        self.ensureDiagnostics();
        self.diagnostics.len() as i64
    }
    pub fn problemAt(&self, i: i64) -> Rc<RefCell<EditorDiagnostic>> {
        self.diagnostics[i as usize].clone()
    }
    /// The first complaint about this line, or a blank one. The status bar shows
    /// it for the line the caret is on, which is how a message stays out of the
    /// way until it is wanted.
    pub fn problemOnLine(&mut self, line: i64) -> Rc<RefCell<EditorDiagnostic>> {
        self.ensureDiagnostics();
        let mut i: i64 = 0;
        let n: i64 = self.diagnostics.len() as i64;
        while i < n {
            let mut d: Rc<RefCell<EditorDiagnostic>> = self.diagnostics[i as usize].clone();
            if d.borrow().coversLine(line) {
                return d;
            }
            i += 1;
        }
        let mut blank: Rc<RefCell<EditorDiagnostic>> = Rc::new(RefCell::new(EditorDiagnostic::new()));
        blank.borrow_mut().line = -1;
        blank
    }
    pub fn hasProblemOnLine(&mut self, line: i64) -> bool {
        let mut d: Rc<RefCell<EditorDiagnostic>> = self.problemOnLine(line);
        return d.borrow().line >= 0;
    }
    /// Caret to the next complaint, wrapping. F8 in every other editor.
    pub fn goToNextProblem(&mut self) -> bool {
        self.ensureDiagnostics();
        let n: i64 = self.diagnostics.len() as i64;
        if n == 0 {
            return false;
        }
        let here: i64 = self.sel.borrow().caret.borrow().line;
        let mut best: i64 = -1;
        let mut i: i64 = 0;
        while i < n {
            let mut d: Rc<RefCell<EditorDiagnostic>> = self.diagnostics[i as usize].clone();
            if d.borrow().line > here {
                if best < 0 {
                    best = i;
                }
            }
            i += 1;
        }
        if best < 0 {
            best = 0;
        }
        let mut target: Rc<RefCell<EditorDiagnostic>> = self.diagnostics[best as usize].clone();
        let mut _tmp_3556 = target.borrow().line.clone();
        self.moveCaret(_tmp_3556, target.borrow().col.clone(), false);
        self.layout.borrow_mut().prefer_col = self.sel.borrow().caret.borrow().col;
        let mut _tmp_3560 = self.buf.clone();
        let mut _tmp_3561 = self.sel.clone();
        EditorLayout::ensure_caret_visible(&self.layout.clone(), &_tmp_3560, &_tmp_3561);
        true
    }
    /// --- completion -------------------------------------------------------------
    /// Ask the plugin. Nothing here knows what a candidate is or why one sorts
    /// before another — see lang/EditorLanguage.rgr.
    pub fn openCompletions(&mut self) -> bool {
        let line: i64 = self.sel.borrow().caret.borrow().line;
        let col: i64 = self.sel.borrow().caret.borrow().col;
        let _tmp_3565 = self.language.borrow().complete(&self.buf.borrow().lines.clone(), line, col);
        self.completions = _tmp_3565;
        let n: i64 = self.completions.len() as i64;
        if n == 0 {
            self.completionOpen = false;
            return false;
        }
        let text: String = EditorBuffer::line_at(&self.buf, line);
        self.completionStart = LanguageText::prefixStart(&text, col);
        self.completionLine = line;
        self.completionIndex = 0;
        self.completionOpen = true;
        true
    }
    pub fn closeCompletions(&mut self) {
        self.completionOpen = false;
        self.completions.clear();
    }
    /// After an edit: keep the list up while the word is still being typed, and
    /// take it down the moment the caret leaves the word it belongs to.
    pub fn refreshCompletions(&mut self) {
        if !self.completionOpen {
            return;
        }
        if self.sel.borrow().caret.borrow().line != self.completionLine {
            self.closeCompletions();
            return;
        }
        if self.sel.borrow().caret.borrow().col < self.completionStart {
            self.closeCompletions();
            return;
        }
        self.openCompletions();
    }
    pub fn completionCount(&self) -> i64 {
        self.completions.len() as i64
    }
    pub fn completionAt(&self, i: i64) -> Rc<RefCell<Completion>> {
        self.completions[i as usize].clone()
    }
    pub fn moveCompletion(&mut self, delta: i64) {
        let n: i64 = self.completions.len() as i64;
        if n == 0 {
            return;
        }
        let mut next: i64 = self.completionIndex + delta;
        while next < 0 {
            next += n;
        }
        while next >= n {
            next -= n;
        }
        self.completionIndex = next;
    }
    /// Replace the word being typed with the picked one. Not an insertion: the
    /// three letters that opened the list are part of what is being replaced.
    pub fn acceptCompletion(&mut self) -> bool {
        if !self.completionOpen {
            return false;
        }
        let n: i64 = self.completions.len() as i64;
        if n == 0 {
            self.closeCompletions();
            return false;
        }
        let mut pick: Rc<RefCell<Completion>> = self.completions[(self.completionIndex) as usize].clone();
        let line: i64 = self.completionLine;
        let from: i64 = self.completionStart;
        let to: i64 = self.sel.borrow().caret.borrow().col;
        self.closeCompletions();
        let mut _tmp_3569 = pick.borrow().insert.clone();
        let mut p: Rc<RefCell<EditorPos>> = EditorBuffer::replace_range(
            &self.buf,
            line,
            from,
            line,
            to,
            _tmp_3569,
            true
        );
        self.markEdited(&p);
        true
    }
    /// --- tokens ---------------------------------------------------------------
    pub fn ensureTokens(&mut self) {
        if self.tokenVersion == self.buf.borrow().version {
            return;
        }
        // Through the plugin: the editor does not know which lexer that is.
        let _tmp_3572 = self.language.borrow().tokenize(&self.buf.borrow().lines.clone());
        self.lines = _tmp_3572;
        self.tokenVersion = self.buf.borrow().version;
    }
    pub fn tokensFor(&self, line: i64) -> Rc<RefCell<CodeTokenLine>> {
        if line < 0 {
            let mut none: Rc<RefCell<CodeTokenLine>> = Rc::new(RefCell::new(CodeTokenLine::new()));
            return none;
        }
        if line >= (self.lines.len() as i64) {
            let mut none2: Rc<RefCell<CodeTokenLine>> = Rc::new(RefCell::new(CodeTokenLine::new()));
            return none2;
        }
        self.lines[line as usize].clone()
    }
    /// --- minimap --------------------------------------------------------------
    /// How wide the strip is this frame: the configured width, or nothing when
    /// the panel cannot spare it. Every other measurement reads this, so a
    /// hidden map takes no space and eats no clicks.
    /// --- find ------------------------------------------------------------------
    /// Ctrl+F. Opening with a selection on one line takes that text as the
    /// query, which saves retyping the word you just pointed at.
    pub fn openFind(&mut self) -> bool {
        self.findOpen = true;
        self.closeCompletions();
        if !EditorSelection::is_collapsed(&self.sel) {
            let mut a: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
            let mut b: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
            EditorSelection::normalized(&self.sel.clone(), &a, &b);
            if a.borrow().line == b.borrow().line {
                let _tmp_3579 = CeNative::substr(&EditorBuffer::line_at(&self.buf, a.borrow().line.clone()), a.borrow().col, b.borrow().col);
                self.findQuery = _tmp_3579;
            }
        }
        self.refreshFindCount();
        true
    }
    pub fn closeFind(&mut self) {
        self.findOpen = false;
    }
    pub fn findActive(&self) -> bool {
        self.findOpen
    }
    /// The bar's own text entry. While it is up every keystroke arrives here
    /// first, so a search never edits the document.
    pub fn findType(&mut self, text: &str) {
        self.findQuery.push_str(&text);
        let line: i64 = self.sel.borrow().caret.borrow().line;
        let col: i64 = self.sel.borrow().caret.borrow().col;
        self.findFrom(line, col, false, true);
        self.refreshFindCount();
    }
    pub fn findBackspace(&mut self) {
        let n: i64 = CeNative::str_len(&self.findQuery);
        if n > 0 {
            self.findQuery = CeNative::substr(&self.findQuery, 0, n - 1);
        }
        let line: i64 = self.sel.borrow().caret.borrow().line;
        let col: i64 = self.sel.borrow().caret.borrow().col;
        self.findFrom(line, col, false, true);
        self.refreshFindCount();
    }
    pub fn findNext(&mut self, back: bool) -> bool {
        if self.findQuery.is_empty() {
            return false;
        }
        let mut a: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        let mut b: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        EditorSelection::normalized(&self.sel.clone(), &a, &b);
        let mut col: i64 = a.borrow().col + 1;
        if back {
            col = a.borrow().col;
        }
        let hit: bool = self.findFrom(a.borrow().line.clone(), col, back, false);
        self.refreshFindCount();
        hit
    }
    /// Search from (line, col), wrapping once around the buffer. `atCaret`
    /// restarts at the current match, so typing one more letter keeps the
    /// match under the caret instead of skipping to the next one.
    pub fn findFrom(&mut self, line: i64, col: i64, back: bool, at_caret: bool) -> bool {
        let q: String = self.findQuery.to_lowercase();
        if q.is_empty() {
            return false;
        }
        let n: i64 = EditorBuffer::line_count(&self.buf);
        if n == 0 {
            return false;
        }
        let mut start_line: i64 = line;
        let mut start_col: i64 = col;
        if at_caret {
            let mut a: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
            let mut b: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
            EditorSelection::normalized(&self.sel.clone(), &a, &b);
            start_line = a.borrow().line;
            start_col = a.borrow().col;
        }
        let mut i: i64 = 0;
        while i <= n {
            let mut li: i64 = start_line;
            if back {
                li = start_line - i;
                while li < 0 {
                    li += n;
                }
            } else {
                li = (start_line + i) % n;
            }
            let text: String = EditorBuffer::line_at(&self.buf, li).to_lowercase();
            let mut at: i64 = -1;
            if back {
                let mut limit: i64 = CeNative::str_len(&text);
                if i == 0 {
                    limit = start_col;
                }
                at = ScriptEditor::lastIndexOfIn(&text, &q, limit);
            } else {
                let mut from: i64 = 0;
                if i == 0 {
                    from = start_col;
                }
                at = ScriptEditor::indexOfIn(&text, &q, from);
            }
            if at >= 0 {
                EditorSelection::set_anchor(&self.sel.clone(), li, at);
                let arg_0 = at + (CeNative::str_len(&self.findQuery));
                EditorSelection::set_caret(&self.sel.clone(), li, arg_0);
                self.layout.borrow_mut().prefer_col = self.sel.borrow().caret.borrow().col;
                let mut _tmp_3598 = self.buf.clone();
                let mut _tmp_3599 = self.sel.clone();
                EditorLayout::ensure_caret_visible(&self.layout.clone(), &_tmp_3598, &_tmp_3599);
                self.ensureCaretX();
                return true;
            }
            i += 1;
        }
        false
    }
    /// "3 of 17" needs both numbers, and the ordinal is which of the matches
    /// the selection is sitting on.
    pub fn refreshFindCount(&mut self) {
        self.findTotal = 0;
        self.findOrdinal = 0;
        let q: String = self.findQuery.to_lowercase();
        if q.is_empty() {
            return;
        }
        let mut a: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        let mut b: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        EditorSelection::normalized(&self.sel.clone(), &a, &b);
        let n: i64 = EditorBuffer::line_count(&self.buf);
        let mut li: i64 = 0;
        while li < n {
            let text: String = EditorBuffer::line_at(&self.buf, li).to_lowercase();
            let mut from: i64 = 0;
            let mut at: i64 = ScriptEditor::indexOfIn(&text, &q, from);
            while at >= 0 {
                self.findTotal += 1;
                if li == a.borrow().line {
                    if at == a.borrow().col {
                        self.findOrdinal = self.findTotal;
                    }
                }
                from = at + 1;
                at = ScriptEditor::indexOfIn(&text, &q, from);
            }
            li += 1;
        }
    }
    pub fn findStatus(&self) -> String {
        if self.findQuery.is_empty() {
            return "find:  (type to search, Esc to close)".to_string();
        }
        if self.findTotal == 0 {
            return format!("find: {}   no matches", self.findQuery);
        }
        format!("find: {}   {} of {}", self.findQuery, self.findOrdinal, self.findTotal)
    }
    pub(crate) fn paintFind(&self, dl: &Rc<RefCell<EVGDisplayList>>, tr: &Rc<RefCell<UITextRenderer>>, blink: i64) {
        if !self.findOpen {
            return;
        }
        let bar_h: f64 = (self.layout.borrow().line_height as f64) + 10.0;
        let right: f64 = ((self.layout.borrow().x + self.layout.borrow().w) as f64) - ((self.minimapWidth()) as f64);
        let mut w: f64 = 320.0;
        let max_w: f64 = (right - ((self.layout.borrow().x + self.layout.borrow().gutter_w) as f64)) - 8.0;
        if w > max_w {
            w = max_w;
        }
        if w < 80.0 {
            return;
        }
        let x: f64 = (right - w) - 8.0;
        let y: f64 = (self.layout.borrow().y as f64) + 6.0;
        self.pushRect(dl, x, y, w, bar_h, &self.theme.borrow().findBg.clone());
        let mut edge: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        edge.borrow_mut().kind = 1;
        edge.borrow_mut().x = x;
        edge.borrow_mut().y = y;
        edge.borrow_mut().w = w;
        edge.borrow_mut().h = bar_h;
        edge.borrow_mut().thickness = 1.0;
        edge.borrow_mut().r = EVGColor::red(&self.theme.borrow().findEdge);
        edge.borrow_mut().g = EVGColor::green(&self.theme.borrow().findEdge);
        edge.borrow_mut().b = EVGColor::blue(&self.theme.borrow().findEdge);
        edge.borrow_mut().a = EVGColor::alpha(&self.theme.borrow().findEdge);
        dl.borrow_mut().cmds.push(edge.clone());
        let mut ink: Rc<RefCell<EVGColor>> = self.theme.borrow().findInk.clone().clone();
        if self.findQuery.is_empty() {
            ink = self.theme.borrow().findDim.clone();
        }
        if !self.findQuery.is_empty() {
            if self.findTotal == 0 {
                ink = self.theme.borrow().errorInk.clone();
            }
        }
        let size: f64 = self.layout.borrow().font_size * 0.9;
        let label: String = self.findStatus();
        self.pushText(dl, tr, label.clone(), x + 8.0, y + 5.0, size, &ink);
        // A caret after the query, blinking on the editor's own clock. Without
        // it the bar gives no sign that the keys are going into it — which is
        // the one thing it has to say while it is up.
        let on: i64 = blink - (blink / 2) * 2;
        if on == 0 {
            let head: String = format!("find: {}", self.findQuery);
            let cx: f64 = (x + 8.0) + UITextRenderer::measure_width(&tr, &head, size);
            let mut _tmp_3613 = self.theme.borrow().caret.clone();
            self.pushRect(dl, cx, y + 4.0, 1.5, (self.layout.borrow().line_height as f64) - 2.0, &_tmp_3613);
        }
    }
    pub fn minimapWidth(&self) -> i64 {
        if !self.showMinimap {
            return 0;
        }
        let need: i64 = (self.layout.borrow().gutter_w + 200) + self.minimapW;
        if self.layout.borrow().w < need {
            return 0;
        }
        self.minimapW
    }
    pub fn minimapX(&self) -> i64 {
        let mw: i64 = self.minimapWidth();
        (self.layout.borrow().x + self.layout.borrow().w) - mw
    }
    pub fn insideMinimap(&self, px: i64, py: i64) -> bool {
        let mw: i64 = self.minimapWidth();
        if mw <= 0 {
            return false;
        }
        let left: i64 = self.minimapX();
        if px < left {
            return false;
        }
        let right: i64 = self.layout.borrow().x + self.layout.borrow().w;
        if px >= right {
            return false;
        }
        if py < self.layout.borrow().y {
            return false;
        }
        let bottom: i64 = self.layout.borrow().y + self.layout.borrow().h;
        py < bottom
    }
    /// Pixels of minimap height per source line. Two is the usual size; a
    /// file that would not fit is scaled so the picture still sits in the
    /// strip. Short files stay a compact picture at the top — they are not
    /// stretched to fill the editor.
    pub fn minimapScale(&self) -> f64 {
        let n: i64 = EditorBuffer::line_count(&self.buf);
        if n < 1 {
            return 2.0;
        }
        let h: f64 = self.layout.borrow().h as f64;
        if h < 1.0 {
            return 2.0;
        }
        let mut px: f64 = 2.0;
        let need: f64 = (n as f64) * px;
        if need > h {
            px = h / (n as f64);
        }
        px
    }
    /// How tall the specks actually are, which is what a pointer on the
    /// strip is measured against. Empty space below a short file is not
    /// part of the picture.
    pub fn minimapPictureH(&self) -> f64 {
        let n: i64 = EditorBuffer::line_count(&self.buf);
        let px: f64 = self.minimapScale();
        let h: f64 = (n as f64) * px;
        let max_h: f64 = self.layout.borrow().h as f64;
        if h > max_h {
            return max_h;
        }
        if h < 1.0 {
            return 1.0;
        }
        h
    }
    pub fn minimapLineAt(&self, py: i64) -> i64 {
        let n: i64 = EditorBuffer::line_count(&self.buf);
        if n <= 1 {
            return 0;
        }
        let px: f64 = self.minimapScale();
        if px < 0.0001 {
            return 0;
        }
        let mut local: f64 = (py as f64) - (self.layout.borrow().y as f64);
        if local < 0.0 {
            local = 0.0;
        }
        let mut line: i64 = (local / px).floor() as i64;
        let last: i64 = n - 1;
        if line > last {
            line = last;
        }
        if line < 0 {
            line = 0;
        }
        line
    }
    /// The scrollbar square sits on the picture. The pointer's Y as a
    /// fraction of that picture's height is the editor's scroll as a
    /// fraction of how far it can go — empty space below the specks does
    /// not stretch the mapping.
    pub fn jumpMinimap(&self, py: i64) {
        let n: i64 = EditorBuffer::line_count(&self.buf);
        let vis: i64 = EditorLayout::visible_line_count(&self.layout);
        let mut max_scroll: i64 = n - vis;
        if max_scroll < 0 {
            max_scroll = 0;
        }
        let pic_h: f64 = self.minimapPictureH();
        let mut local: f64 = (py as f64) - (self.layout.borrow().y as f64);
        if local < 0.0 {
            local = 0.0;
        }
        if local > pic_h {
            local = pic_h;
        }
        let mut t: f64 = 0.0;
        if pic_h > 0.0001 {
            t = local / pic_h;
        }
        if t > 1.0 {
            t = 1.0;
        }
        let max_d: f64 = max_scroll as f64;
        let scrolled: i64 = (t * max_d + 0.5).floor() as i64;
        self.layout.borrow_mut().scroll_line = scrolled;
        let mut _tmp_3620 = self.buf.clone();
        EditorLayout::clamp_scroll(&self.layout.clone(), &_tmp_3620);
    }
    /// True when the pointer is on the map and the view was moved. Pointer
    /// down and drag both come through here so a test can drive it without a
    /// text renderer.
    pub fn tryMinimapPointer(&mut self, px: i64, py: i64) -> bool {
        if !self.insideMinimap(px, py) {
            self.minimapDrag = false;
            return false;
        }
        self.minimapDrag = true;
        self.jumpMinimap(py);
        true
    }
    /// --- horizontal scroll ----------------------------------------------------
    /// The text area's width, excluding gutter, minimap and the padding the
    /// caret sits in. This is what a long line is compared against.
    pub fn textViewW(&self) -> i64 {
        let mut vw: i64 = ((self.layout.borrow().w - self.layout.borrow().gutter_w) - self.minimapWidth()) - self.layout.borrow()
            .pad_x;
        vw -= self.layout.borrow().pad_x;
        if vw < 1 {
            return 1;
        }
        vw
    }
    pub(crate) fn linePixelW(&self, tr: &Rc<RefCell<UITextRenderer>>, s: &str) -> i64 {
        let w: i64 = (UITextRenderer::measure_width(&tr, s, self.layout.borrow().font_size.clone())).floor() as i64;
        if w < 1 {
            let n: i64 = CeNative::str_len(&s);
            if n <= 0 {
                return 0;
            }
            return n * 8;
        }
        w
    }
    pub(crate) fn refreshContentWidth(&mut self, tr: &Rc<RefCell<UITextRenderer>>) {
        if self.contentWVersion == self.buf.borrow().version {
            if self.contentWPx > 0 {
                return;
            }
        }
        let mut max_w: i64 = 0;
        let mut i: i64 = 0;
        let n: i64 = EditorBuffer::line_count(&self.buf);
        while i < n {
            let s: String = EditorBuffer::line_at(&self.buf, i);
            let lw: i64 = self.linePixelW(tr, &s);
            if lw > max_w {
                max_w = lw;
            }
            i += 1;
        }
        self.contentWPx = max_w;
        self.contentWVersion = self.buf.borrow().version;
    }
    pub fn maxScrollX(&self) -> i64 {
        let extra: i64 = (self.contentWPx - self.textViewW()) + 8;
        if extra < 0 {
            return 0;
        }
        extra
    }
    pub fn hScrollH(&self) -> i64 {
        if self.maxScrollX() > 0 {
            return self.hBarSize;
        }
        0
    }
    pub fn hBarX(&self) -> i64 {
        self.layout.borrow().x + self.layout.borrow().gutter_w
    }
    pub fn hBarY(&self) -> i64 {
        (self.layout.borrow().y + self.layout.borrow().h) - self.hBarSize
    }
    pub fn hBarW(&self) -> i64 {
        let bw: i64 = (self.layout.borrow().w - self.layout.borrow().gutter_w) - self.minimapWidth();
        if bw < 1 {
            return 1;
        }
        bw
    }
    pub fn insideHBar(&self, px: i64, py: i64) -> bool {
        if self.hScrollH() <= 0 {
            return false;
        }
        if px < self.hBarX() {
            return false;
        }
        if px >= self.hBarX() + self.hBarW() {
            return false;
        }
        if py < self.hBarY() {
            return false;
        }
        let bottom: i64 = self.layout.borrow().y + self.layout.borrow().h;
        py < bottom
    }
    pub fn jumpHBar(&self, px: i64) {
        let max_x: i64 = self.maxScrollX();
        if max_x <= 0 {
            self.layout.borrow_mut().scroll_x = 0;
            return;
        }
        let x0: i64 = self.hBarX() + 4;
        let span: i64 = self.hBarW() - 8;
        if span < 1 {
            return;
        }
        let mut t: f64 = ((px as f64) - (x0 as f64)) / (span as f64);
        if t < 0.0 {
            t = 0.0;
        }
        if t > 1.0 {
            t = 1.0;
        }
        self.layout.borrow_mut()
            .scroll_x = (t * (max_x as f64) + 0.5)
            .floor() as i64;
        self.clampHScroll();
    }
    pub fn tryHBarPointer(&mut self, px: i64, py: i64) -> bool {
        if !self.insideHBar(px, py) {
            self.hBarDrag = false;
            return false;
        }
        self.hBarDrag = true;
        self.jumpHBar(px);
        true
    }
    pub fn clampHScroll(&self) {
        let mut _tmp_3627 = self.maxScrollX();
        EditorLayout::clamp_scroll_x(&self.layout.clone(), _tmp_3627);
    }
    pub fn scrollByX(&self, pixels: i64) {
        self.layout.borrow_mut().scroll_x = self.layout.borrow().scroll_x + pixels;
        self.clampHScroll();
    }
    pub(crate) fn rememberTr(&mut self, tr: Rc<RefCell<UITextRenderer>>) {
        self.measureTr = Some(tr.clone());
    }
    pub(crate) fn caretViewX(&self, tr: &Rc<RefCell<UITextRenderer>>, line: i64, col: i64) -> i64 {
        return EditorLayout::caret_view_x(&self.layout, tr, &self.buf.clone(), line, col);
    }
    pub fn ensureCaretX(&mut self) {
        if self.measureTr.is_none() {
            return;
        }
        let mut tr: Rc<RefCell<UITextRenderer>> = self.measureTr.clone().unwrap();
        self.refreshContentWidth(&tr);
        let mut _tmp_3633 = self.buf.clone();
        let mut _tmp_3634 = self.sel.borrow().caret.borrow().line.clone();
        let mut _tmp_3635 = self.sel.borrow().caret.borrow().col.clone();
        let cx: i64 = EditorLayout::caret_pixel_x(
            &self.layout,
            &tr,
            &_tmp_3633,
            _tmp_3634,
            _tmp_3635
        );
        let left: i64 = EditorLayout::content_x(&self.layout);
        let view: i64 = self.textViewW();
        let rel: i64 = cx - left;
        if rel < self.layout.borrow().scroll_x {
            self.layout.borrow_mut().scroll_x = rel;
        }
        let right: i64 = (self.layout.borrow().scroll_x + view) - 2;
        if rel > right {
            self.layout.borrow_mut().scroll_x = (rel - view) + 2;
        }
        self.clampHScroll();
    }
    pub(crate) fn paintHScroll(&self, dl: &Rc<RefCell<EVGDisplayList>>) {
        if self.hScrollH() <= 0 {
            return;
        }
        let x0: f64 = (self.hBarX()) as f64;
        let y0: f64 = (self.hBarY()) as f64;
        let w0: f64 = (self.hBarW()) as f64;
        let h0: f64 = self.hBarSize as f64;
        self.pushRect(dl, x0, y0, w0, h0, &self.theme.borrow().hBarTrack.clone());
        let mut edge: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        edge.borrow_mut().kind = 1;
        edge.borrow_mut().x = x0;
        edge.borrow_mut().y = y0;
        edge.borrow_mut().w = w0;
        edge.borrow_mut().h = 1.0;
        edge.borrow_mut().thickness = 1.0;
        edge.borrow_mut().r = EVGColor::red(&self.theme.borrow().hBarEdge);
        edge.borrow_mut().g = EVGColor::green(&self.theme.borrow().hBarEdge);
        edge.borrow_mut().b = EVGColor::blue(&self.theme.borrow().hBarEdge);
        edge.borrow_mut().a = EVGColor::alpha(&self.theme.borrow().hBarEdge);
        dl.borrow_mut().cmds.push(edge.clone());
        let max_x: i64 = self.maxScrollX();
        let track_w: f64 = w0 - 8.0;
        if track_w < 8.0 {
            return;
        }
        let view: f64 = (self.textViewW()) as f64;
        let content: f64 = view + (max_x as f64);
        if content < 1.0 {
            return;
        }
        let mut thumb_w: f64 = (track_w * view) / content;
        if thumb_w < 24.0 {
            thumb_w = 24.0;
        }
        if thumb_w > track_w {
            thumb_w = track_w;
        }
        let travel: f64 = track_w - thumb_w;
        let mut thumb_x: f64 = (x0 + 4.0) + (travel * (self.layout.borrow().scroll_x as f64)) / (max_x as f64);
        if max_x <= 0 {
            thumb_x = x0 + 4.0;
        }
        let thumb_y: f64 = y0 + 2.0;
        let mut thumb_h: f64 = h0 - 4.0;
        if thumb_h < 4.0 {
            thumb_h = 4.0;
        }
        self.pushRect(dl, thumb_x, thumb_y, thumb_w, thumb_h, &self.theme.borrow().hBarThumb.clone());
    }
    /// --- painting -------------------------------------------------------------
    pub(crate) fn pushRect(&self, dl: &Rc<RefCell<EVGDisplayList>>, x: f64, y: f64, w: f64, h: f64, col: &Rc<RefCell<EVGColor>>) {
        let mut c: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        c.borrow_mut().kind = 0;
        c.borrow_mut().x = x;
        c.borrow_mut().y = y;
        c.borrow_mut().w = w;
        c.borrow_mut().h = h;
        c.borrow_mut().r = EVGColor::red(&col);
        c.borrow_mut().g = EVGColor::green(&col);
        c.borrow_mut().b = EVGColor::blue(&col);
        c.borrow_mut().a = EVGColor::alpha(&col);
        dl.borrow_mut().cmds.push(c.clone());
    }
    pub(crate) fn pushClip(&self, dl: &Rc<RefCell<EVGDisplayList>>, x: f64, y: f64, w: f64, h: f64) {
        let mut c: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        c.borrow_mut().kind = 4;
        c.borrow_mut().x = x;
        c.borrow_mut().y = y;
        c.borrow_mut().w = w;
        c.borrow_mut().h = h;
        dl.borrow_mut().cmds.push(c.clone());
    }
    pub(crate) fn popClip(&self, dl: &Rc<RefCell<EVGDisplayList>>) {
        let mut c: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        c.borrow_mut().kind = 5;
        dl.borrow_mut().cmds.push(c.clone());
    }
    pub(crate) fn pushText(&self, dl: &Rc<RefCell<EVGDisplayList>>, tr: &Rc<RefCell<UITextRenderer>>, s: String, x: f64, y: f64, size: f64, col: &Rc<RefCell<EVGColor>>) {
        if s.is_empty() {
            return;
        }
        let mut c: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        c.borrow_mut().kind = 3;
        c.borrow_mut().x = x;
        c.borrow_mut().y = y;
        c.borrow_mut().w = UITextRenderer::measure_width(&tr, &s, size);
        c.borrow_mut().h = UITextRenderer::line_height(&tr, size);
        c.borrow_mut().text = s.clone();
        c.borrow_mut().font_family = self.fontFamily.clone();
        c.borrow_mut().font_size = size;
        c.borrow_mut().r = EVGColor::red(&col);
        c.borrow_mut().g = EVGColor::green(&col);
        c.borrow_mut().b = EVGColor::blue(&col);
        c.borrow_mut().a = EVGColor::alpha(&col);
        dl.borrow_mut().cmds.push(c.clone());
    }
    /// `blink` is a phase, not a frame count: the page derives it from the
    /// clock so a page that only repaints when something changed still blinks.
    pub(crate) fn paint(&mut self, dl: &Rc<RefCell<EVGDisplayList>>, tr: Rc<RefCell<UITextRenderer>>, blink: i64) {
        self.rememberTr(tr.clone());
        EditorLayout::sync_metrics(&self.layout.clone(), &tr);
        self.layout.borrow_mut().right_inset = self.minimapWidth();
        self.refreshContentWidth(&tr);
        if self.maxScrollX() > 0 {
            self.layout.borrow_mut().bottom_inset = self.hBarSize;
        } else {
            self.layout.borrow_mut().bottom_inset = 0;
        }
        let mut _tmp_3658 = self.buf.clone();
        EditorLayout::clamp_scroll(&self.layout.clone(), &_tmp_3658);
        self.clampHScroll();
        self.ensureTokens();
        self.ensureDiagnostics();
        let x0: f64 = self.layout.borrow().x as f64;
        let y0: f64 = self.layout.borrow().y as f64;
        let w0: f64 = self.layout.borrow().w as f64;
        let h0: f64 = self.layout.borrow().h as f64;
        self.pushRect(dl, x0, y0, w0, h0, &self.theme.borrow().bg.clone());
        let mut _tmp_3661 = self.theme.borrow().gutterBg.clone();
        self.pushRect(dl, x0, y0, self.layout.borrow().gutter_w as f64, h0, &_tmp_3661);
        let map_w: i64 = self.minimapWidth();
        let clip_w: f64 = w0 - (map_w as f64);
        self.pushClip(dl, x0, y0, clip_w, h0);
        let first: i64 = EditorLayout::first_visible(&self.layout);
        let last: i64 = EditorLayout::last_visible(&self.layout, &self.buf.clone());
        let caret_line: i64 = self.sel.borrow().caret.borrow().line;
        let bar_h: i64 = self.hScrollH();
        let text_h: f64 = h0 - (bar_h as f64);
        let text_x: f64 = (self.layout.borrow().x + self.layout.borrow().gutter_w) as f64;
        let text_w: f64 = clip_w - (self.layout.borrow().gutter_w as f64);
        // The line the caret is on, banded across the text area — the one piece
        // of chrome that makes a wall of code navigable.
        if EditorSelection::is_collapsed(&self.sel) {
            if ((caret_line >= first) && (caret_line <= last)) {
                let band_y: f64 = (EditorLayout::caret_pixel_y(&self.layout, caret_line)) as f64;
                let band_x: f64 = text_x;
                let band_w: f64 = text_w;
                let mut _tmp_3667 = self.theme.borrow().currentLine.clone();
                self.pushRect(dl, band_x, band_y, band_w, self.layout.borrow().line_height as f64, &_tmp_3667);
            }
        } else {
            self.pushClip(dl, text_x, y0, text_w, text_h);
            self.paintSelection(dl, &tr, first, last);
            self.popClip(dl);
        }
        // Diff bands under the text, over the current-line band: the change
        // is what this view is for.
        if !self.lineMarks.is_empty() {
            let band_x2: f64 = text_x;
            let band_w2: f64 = text_w;
            let mut mi: i64 = first;
            while mi <= last {
                let mark: i64 = self.lineMark(mi);
                if mark > 0 {
                    let my: f64 = (EditorLayout::caret_pixel_y(&self.layout, mi)) as f64;
                    let mut band: Rc<RefCell<EVGColor>> = self.theme.borrow()
                        .addedLine.clone()
                        .clone();
                    let mut strip: Rc<RefCell<EVGColor>> = self.theme.borrow()
                        .addedMark.clone()
                        .clone();
                    if mark == 2 {
                        band = self.theme.borrow().removedLine.clone();
                        strip = self.theme.borrow().removedMark.clone();
                    }
                    self.pushRect(dl, band_x2, my, band_w2, self.layout.borrow().line_height as f64, &band);
                    self.pushRect(dl, x0, my, 3.0, self.layout.borrow().line_height as f64, &strip);
                }
                mi += 1;
            }
        }
        let mut li: i64 = first;
        while li <= last {
            let ty: i64 = EditorLayout::caret_pixel_y(&self.layout, li);
            let num: String = (li + 1).to_string();
            let num_size: f64 = self.layout.borrow().font_size * 0.85;
            let nw: i64 = (UITextRenderer::measure_width(&tr, &num, num_size)).floor() as i64;
            let nx: i64 = ((self.layout.borrow().x + self.layout.borrow().gutter_w) - nw) - 8;
            let mut num_col: Rc<RefCell<EVGColor>> = self.theme.borrow()
                .gutterText.clone()
                .clone();
            if li == caret_line {
                num_col = self.theme.borrow().gutterActive.clone();
            }
            self.pushText(dl,
                &tr,
                num.clone(),
                nx as f64,
                (ty + 2) as f64,
                num_size,
                &num_col
            );
            li += 1;
        }
        self.pushClip(dl, text_x, y0, text_w, text_h);
        let mut lj: i64 = first;
        while lj <= last {
            self.paintLine(dl, &tr, lj, EditorLayout::caret_pixel_y(&self.layout, lj));
            lj += 1;
        }
        // Diagnostics over the text: a squiggle under the span. Gutter marks
        // are painted after this clip so a problem still shows in the margin
        // when the span itself has scrolled off to the left.
        self.paintDiagnosticSquiggles(dl, &tr, first, last);
        // Caret: on for half a second, off for half a second, at 60 fps.
        if self.focused {
            if ((caret_line >= first) && (caret_line <= last)) {
                let on: i64 = blink - (blink / 2) * 2;
                if on == 0 {
                    let cx: i64 = self.caretViewX(&tr, caret_line, self.sel.borrow().caret.borrow().col.clone());
                    let cy: i64 = EditorLayout::caret_pixel_y(&self.layout, caret_line);
                    let mut _tmp_3675 = self.theme.borrow().caret.clone();
                    self.pushRect(dl, cx as f64, cy as f64, 2.0, self.layout.borrow().line_height as f64, &_tmp_3675);
                }
            }
        }
        // The completion list, over the text and inside the panel's clip: a
        // popup that could be drawn outside the editor would be drawn over the
        // status bar the first time someone typed on the last line.
        self.paintCompletions(dl, &tr);
        self.popClip(dl);
        self.paintDiagnosticMarks(dl, &tr, first, last);
        self.paintFind(dl, &tr, blink);
        self.popClip(dl);
        self.paintHScroll(dl);
        self.paintMinimap(dl);
        let mut edge: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        edge.borrow_mut().kind = 1;
        edge.borrow_mut().x = x0;
        edge.borrow_mut().y = y0;
        edge.borrow_mut().w = w0;
        edge.borrow_mut().h = h0;
        edge.borrow_mut().thickness = 1.0;
        edge.borrow_mut().r = EVGColor::red(&self.theme.borrow().border);
        edge.borrow_mut().g = EVGColor::green(&self.theme.borrow().border);
        edge.borrow_mut().b = EVGColor::blue(&self.theme.borrow().border);
        edge.borrow_mut().a = EVGColor::alpha(&self.theme.borrow().border);
        dl.borrow_mut().cmds.push(edge.clone());
    }
    pub(crate) fn paintCompletions(&self, dl: &Rc<RefCell<EVGDisplayList>>, tr: &Rc<RefCell<UITextRenderer>>) {
        if !self.completionOpen {
            return;
        }
        let n: i64 = self.completions.len() as i64;
        if n == 0 {
            return;
        }
        let mut shown: i64 = n;
        if shown > self.completionRows {
            shown = self.completionRows;
        }
        // Scroll the window of rows so the picked one is always in it.
        let mut first: i64 = 0;
        if self.completionIndex >= shown {
            first = (self.completionIndex - shown) + 1;
        }
        let row_h: i64 = self.layout.borrow().line_height + 2;
        let mut widest: f64 = 120.0;
        let mut i: i64 = 0;
        while i < shown {
            let mut c: Rc<RefCell<Completion>> = self.completions[(first + i) as usize].clone();
            let mut _tmp_3680 = self.layout.borrow().font_size.clone();
            let w: f64 = UITextRenderer::measure_width(
                &tr,
                &format!("{}   {}", c.borrow().label, c.borrow().detail),
                _tmp_3680
            );
            if w > widest {
                widest = w;
            }
            i += 1;
        }
        let box_w: f64 = widest + 28.0;
        let box_h: f64 = (row_h * shown) as f64;
        let mut _tmp_3681 = self.completionLine.clone();
        let mut x: f64 = (self.caretViewX(tr, _tmp_3681, self.completionStart.clone())) as f64;
        let mut y: f64 = ((EditorLayout::caret_pixel_y(&self.layout, self.completionLine.clone()) + self.layout.borrow().line_height) + 2) as f64;
        // Flip above the caret when there is no room below it.
        let bottom: f64 = y + box_h;
        let limit: f64 = (self.layout.borrow().y + self.layout.borrow().h) as f64;
        if bottom > limit {
            y = (((EditorLayout::caret_pixel_y(&self.layout, self.completionLine)) as f64) - box_h) - 2.0;
        }
        let right: f64 = x + box_w;
        let map_w: i64 = self.minimapWidth();
        let edge_x: f64 = (((self.layout.borrow().x + self.layout.borrow().w) - map_w) - 8) as f64;
        if right > edge_x {
            x = edge_x - box_w;
        }
        if x < (self.layout.borrow().x as f64) {
            x = self.layout.borrow().x as f64;
        }
        self.pushRect(dl, x + 2.0, y + 2.0, box_w, box_h, &EVGColor::rgba(10, 12, 20, 0.45).clone());
        self.pushRect(dl, x, y, box_w, box_h, &self.theme.borrow().popupBg.clone());
        let mut border: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        border.borrow_mut().kind = 1;
        border.borrow_mut().x = x;
        border.borrow_mut().y = y;
        border.borrow_mut().w = box_w;
        border.borrow_mut().h = box_h;
        border.borrow_mut().thickness = 1.0;
        border.borrow_mut().r = EVGColor::red(&self.theme.borrow().popupEdge);
        border.borrow_mut().g = EVGColor::green(&self.theme.borrow().popupEdge);
        border.borrow_mut().b = EVGColor::blue(&self.theme.borrow().popupEdge);
        border.borrow_mut().a = EVGColor::alpha(&self.theme.borrow().popupEdge);
        dl.borrow_mut().cmds.push(border.clone());
        let mut r: i64 = 0;
        while r < shown {
            let idx: i64 = first + r;
            let mut c2: Rc<RefCell<Completion>> = self.completions[idx as usize].clone();
            let ry: f64 = y + ((row_h * r) as f64);
            let mut ink: Rc<RefCell<EVGColor>> = self.theme.borrow().plain.clone().clone();
            if idx == self.completionIndex {
                let mut _tmp_3690 = self.theme.borrow().popupPick.clone();
                self.pushRect(dl,
                    x + 1.0,
                    ry,
                    box_w - 2.0,
                    row_h as f64,
                    &_tmp_3690
                );
            }
            if c2.borrow().kind == CompletionKind::builtin() {
                ink = self.theme.borrow().builtin.clone();
            }
            if c2.borrow().kind == CompletionKind::keyword() {
                ink = self.theme.borrow().keyword.clone();
            }
            let mut _tmp_3693 = c2.borrow().label.clone();
            self.pushText(dl, tr, _tmp_3693, x + 8.0, ry + 1.0, self.layout.borrow().font_size.clone(), &ink);
            let mut _tmp_3695 = c2.borrow().label.clone();
            let label_w: f64 = UITextRenderer::measure_width(&tr, &_tmp_3695, self.layout.borrow().font_size.clone());
            let mut _tmp_3697 = c2.borrow().detail.clone();
            let mut _tmp_3698 = self.theme.borrow().popupDetail.clone();
            let arg_0 = self.layout.borrow().font_size * 0.85;
            self.pushText(dl,
                tr,
                _tmp_3697,
                (x + 20.0) + label_w,
                ry + 1.0,
                arg_0,
                &_tmp_3698
            );
            r += 1;
        }
        // How many are not shown, said rather than implied by a scrollbar.
        if n > shown {
            let more: String = format!("+{} more", n - shown);
            let mut _tmp_3699 = self.theme.borrow().popupDetail.clone();
            let arg_0 = self.layout.borrow().font_size * 0.8;
            self.pushText(dl,
                tr,
                more.clone(),
                x + 8.0,
                (y + box_h) + 1.0,
                arg_0,
                &_tmp_3699
            );
        }
    }
    /// A wave, drawn as rectangles rather than a font glyph or a path: every
    /// backend this editor runs on can draw a rectangle, and the SDL binary has
    /// no shape library at all.
    pub(crate) fn paintSquiggle(&self, dl: &Rc<RefCell<EVGDisplayList>>, x0: f64, y: f64, x1: f64, col: &Rc<RefCell<EVGColor>>) {
        let mut x: f64 = x0;
        let step: f64 = 3.0;
        let mut up: bool = true;
        while x < x1 {
            let mut dy: f64 = 0.0;
            if !up {
                dy = 2.0;
            }
            let mut w: f64 = step;
            let over: f64 = (x + w) - x1;
            if over > 0.0 {
                w -= over;
            }
            if w > 0.5 {
                self.pushRect(dl, x, y + dy, w, 1.0, col);
            }
            up = !up;
            x += step;
        }
    }
    pub(crate) fn paintDiagnosticSquiggles(&self, dl: &Rc<RefCell<EVGDisplayList>>, tr: &Rc<RefCell<UITextRenderer>>, first: i64, last: i64) {
        self.paintDiagnosticsAt(dl, tr, first, last, false);
    }
    pub(crate) fn paintDiagnosticMarks(&self, dl: &Rc<RefCell<EVGDisplayList>>, tr: &Rc<RefCell<UITextRenderer>>, first: i64, last: i64) {
        self.paintDiagnosticsAt(dl, tr, first, last, true);
    }
    pub(crate) fn paintDiagnostics(&self, dl: &Rc<RefCell<EVGDisplayList>>, tr: &Rc<RefCell<UITextRenderer>>, first: i64, last: i64) {
        self.paintDiagnosticSquiggles(dl, tr, first, last);
        self.paintDiagnosticMarks(dl, tr, first, last);
    }
    pub(crate) fn paintDiagnosticsAt(&self, dl: &Rc<RefCell<EVGDisplayList>>, tr: &Rc<RefCell<UITextRenderer>>, first: i64, last: i64, marks: bool) {
        let mut i: i64 = 0;
        let n: i64 = self.diagnostics.len() as i64;
        while i < n {
            let mut d: Rc<RefCell<EditorDiagnostic>> = self.diagnostics[i as usize].clone();
            let mut col: Rc<RefCell<EVGColor>> = self.theme.borrow().forSeverity(d.borrow().severity.clone());
            let dl0: i64 = d.borrow().line;
            let dl1: i64 = d.borrow().endLine;
            let mut line: i64 = dl0;
            while line <= dl1 {
                if ((line >= first) && (line <= last)) {
                    if marks {
                        let gx: f64 = (self.layout.borrow().x + 4) as f64;
                        let gy: f64 = (EditorLayout::caret_pixel_y(&self.layout, line) + 5) as f64;
                        self.pushRect(dl, gx, gy, 3.0, (self.layout.borrow().line_height - 10) as f64, &col);
                    } else {
                        let text: String = EditorBuffer::line_at(&self.buf, line);
                        let __len: i64 = CeNative::str_len(&text);
                        let mut c0: i64 = 0;
                        let mut c1: i64 = __len;
                        if line == dl0 {
                            c0 = d.borrow().col;
                        }
                        if line == dl1 {
                            c1 = d.borrow().endCol;
                        }
                        if c1 > __len {
                            c1 = __len;
                        }
                        if c1 <= c0 {
                            // A zero-width complaint still has to be visible, so it
                            // underlines the character it points at.
                            c1 = c0 + 1;
                            if c1 > __len {
                                c0 = __len;
                                c1 = __len + 1;
                            }
                        }
                        let x0: i64 = self.caretViewX(tr, line, c0);
                        let mut x1: i64 = self.caretViewX(tr, line, c1);
                        let wide: f64 = (x1 as f64) - (x0 as f64);
                        if wide < 6.0 {
                            x1 = x0 + 6;
                        }
                        let y_base: f64 = ((EditorLayout::caret_pixel_y(&self.layout, line) + self.layout.borrow().line_height) - 3) as f64;
                        self.paintSquiggle(dl, x0 as f64, y_base, x1 as f64, &col);
                    }
                }
                line += 1;
            }
            i += 1;
        }
    }
    pub(crate) fn paintSelection(&self, dl: &Rc<RefCell<EVGDisplayList>>, tr: &Rc<RefCell<UITextRenderer>>, first: i64, last: i64) {
        let mut a: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        let mut b: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        EditorSelection::normalized(&self.sel.clone(), &a, &b);
        let mut line: i64 = a.borrow().line;
        while line <= b.borrow().line {
            if ((line >= first) && (line <= last)) {
                let s: String = EditorBuffer::line_at(&self.buf, line);
                let __len: i64 = CeNative::str_len(&s);
                let mut c0: i64 = 0;
                let mut c1: i64 = __len;
                if line == a.borrow().line {
                    c0 = a.borrow().col;
                }
                if line == b.borrow().line {
                    c1 = b.borrow().col;
                }
                if c0 < 0 {
                    c0 = 0;
                }
                if c1 > __len {
                    c1 = __len;
                }
                if c1 < c0 {
                    c1 = c0;
                }
                let x0: i64 = self.caretViewX(tr, line, c0);
                let mut x1: i64 = self.caretViewX(tr, line, c1);
                if line < b.borrow().line {
                    // A selection that continues onto the next line covers the
                    // newline too, or a block selection looks like ragged bars.
                    x1 += 6;
                }
                let mut rw: i64 = x1 - x0;
                if rw < 2 {
                    rw = 2;
                }
                let sy: i64 = EditorLayout::caret_pixel_y(&self.layout, line);
                let mut _tmp_3709 = self.theme.borrow().selection.clone();
                let arg_0 = self.layout.borrow().line_height as f64;
                self.pushRect(dl,
                    x0 as f64,
                    sy as f64,
                    rw as f64,
                    arg_0,
                    &_tmp_3709
                );
            }
            line += 1;
        }
    }
    pub(crate) fn paintLine(&self, dl: &Rc<RefCell<EVGDisplayList>>, tr: &Rc<RefCell<UITextRenderer>>, line: i64, ty: i64) {
        let s: String = EditorBuffer::line_at(&self.buf, line);
        if s.is_empty() {
            return;
        }
        let mut toks: Rc<RefCell<CodeTokenLine>> = self.tokensFor(line);
        let n: i64 = toks.borrow().count();
        if n == 0 {
            let plain_x: i64 = self.caretViewX(tr, line, 0);
            let mut _tmp_3711 = self.layout.borrow().font_size.clone();
            let mut _tmp_3712 = self.theme.borrow().plain.clone();
            self.pushText(dl,
                tr,
                s.clone(),
                plain_x as f64,
                ty as f64,
                _tmp_3711,
                &_tmp_3712
            );
            return;
        }
        let mut i: i64 = 0;
        while i < n {
            let mut t: Rc<RefCell<CodeToken>> = toks.borrow().at(i);
            let stop: i64 = t.borrow().start + t.borrow().length;
            let part: String = CeNative::substr(&s, t.borrow().start, stop);
            let px: i64 = self.caretViewX(tr, line, t.borrow().start.clone());
            let mut col: Rc<RefCell<EVGColor>> = self.theme.borrow().forKind(t.borrow().kind.clone());
            let mut _tmp_3716 = self.layout.borrow().font_size.clone();
            self.pushText(dl,
                tr,
                part.clone(),
                px as f64,
                ty as f64,
                _tmp_3716,
                &col
            );
            i += 1;
        }
    }
    pub(crate) fn paintMinimap(&self, dl: &Rc<RefCell<EVGDisplayList>>) {
        let mw: i64 = self.minimapWidth();
        if mw <= 0 {
            return;
        }
        let x0: f64 = (self.minimapX()) as f64;
        let y0: f64 = self.layout.borrow().y as f64;
        let w0: f64 = mw as f64;
        let h0: f64 = self.layout.borrow().h as f64;
        self.pushRect(dl, x0, y0, w0, h0, &self.theme.borrow().minimapBg.clone());
        self.pushRect(dl, x0, y0, 1.0, h0, &self.theme.borrow().minimapEdge.clone());
        self.pushClip(dl, x0, y0, w0, h0);
        let n: i64 = EditorBuffer::line_count(&self.buf);
        if n < 1 {
            self.popClip(dl);
            return;
        }
        let scale: f64 = self.minimapScale();
        let caret_line: i64 = self.sel.borrow().caret.borrow().line;
        let inner_x: f64 = x0 + 3.0;
        let mut inner_w: f64 = w0 - 8.0;
        if inner_w < 8.0 {
            inner_w = 8.0;
        }
        // Selection first, so the tokens sit on top of it the way they do in
        // the text — a selected region is a wash, not a replacement.
        if !EditorSelection::is_collapsed(&self.sel) {
            let mut a: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
            let mut b: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
            EditorSelection::normalized(&self.sel.clone(), &a, &b);
            let sy: f64 = y0 + (a.borrow().line as f64) * scale;
            let mut sh: f64 = (((b.borrow().line - a.borrow().line) + 1) as f64) * scale;
            if sh < 2.0 {
                sh = 2.0;
            }
            self.pushRect(dl, inner_x, sy, inner_w, sh, &self.theme.borrow().selection.clone());
        } else {
            let cy: f64 = y0 + (caret_line as f64) * scale;
            let mut ch: f64 = scale;
            if ch < 2.0 {
                ch = 2.0;
            }
            self.pushRect(dl, inner_x, cy, inner_w, ch, &self.theme.borrow().currentLine.clone());
        }
        if scale >= 1.0 {
            let mut li: i64 = 0;
            while li < n {
                let ly: f64 = y0 + (li as f64) * scale;
                let mut lh: f64 = scale;
                if lh > 1.6 {
                    lh -= 0.35;
                }
                self.paintMinimapLine(dl, li, inner_x, ly, inner_w, lh);
                li += 1;
            }
        } else {
            // Longer than the strip: one source line per pixel row, so the
            // command count stays a function of the panel's height.
            let mut row: i64 = 0;
            let rows: i64 = self.layout.borrow().h;
            while row < rows {
                let mut li2: i64 = (((row as f64) * (n as f64)) / h0).floor() as i64;
                if li2 >= n {
                    li2 = n - 1;
                }
                if li2 < 0 {
                    li2 = 0;
                }
                self.paintMinimapLine(dl,
                    li2,
                    inner_x,
                    y0 + (row as f64),
                    inner_w,
                    1.0
                );
                row += 1;
            }
        }
        // A tick on the right for each diagnostic, so a problem below the
        // fold is a colour even when the slider is elsewhere.
        let mut di: i64 = 0;
        let dn: i64 = self.diagnostics.len() as i64;
        let tick_w: f64 = 3.0;
        let tick_x: f64 = (x0 + w0) - (tick_w + 1.0);
        while di < dn {
            let mut d: Rc<RefCell<EditorDiagnostic>> = self.diagnostics[di as usize].clone();
            let tick_y: f64 = y0 + (d.borrow().line as f64) * scale;
            let mut tick_h: f64 = scale;
            if tick_h < 2.0 {
                tick_h = 2.0;
            }
            self.pushRect(dl, tick_x, tick_y, tick_w, tick_h, &self.theme.borrow().forSeverity(d.borrow().severity).clone());
            di += 1;
        }
        // The slider: the visible window, drawn last so it tints the specks
        // rather than hiding them.
        let vis: i64 = EditorLayout::visible_line_count(&self.layout);
        let pic_h: f64 = self.minimapPictureH();
        let mut slider_y: f64 = y0 + (self.layout.borrow().scroll_line as f64) * scale;
        let mut slider_h: f64 = (vis as f64) * scale;
        let pic_bottom: f64 = y0 + pic_h;
        if slider_h > pic_h {
            slider_h = pic_h;
        }
        if slider_h < 8.0 {
            slider_h = 8.0;
            if slider_h > pic_h {
                slider_h = pic_h;
            }
        }
        let slider_bottom: f64 = slider_y + slider_h;
        if slider_bottom > pic_bottom {
            slider_h = pic_bottom - slider_y;
        }
        if slider_y < y0 {
            slider_h -= y0 - slider_y;
            slider_y = y0;
        }
        if slider_h > 0.5 {
            self.pushRect(dl, x0, slider_y, w0, slider_h, &self.theme.borrow().minimapSlider.clone());
            let mut slider_edge: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
            slider_edge.borrow_mut().kind = 1;
            slider_edge.borrow_mut().x = x0;
            slider_edge.borrow_mut().y = slider_y;
            slider_edge.borrow_mut().w = w0;
            slider_edge.borrow_mut().h = slider_h;
            slider_edge.borrow_mut().thickness = 1.0;
            slider_edge.borrow_mut().r = EVGColor::red(&self.theme.borrow().minimapEdge);
            slider_edge.borrow_mut().g = EVGColor::green(&self.theme.borrow().minimapEdge);
            slider_edge.borrow_mut().b = EVGColor::blue(&self.theme.borrow().minimapEdge);
            slider_edge.borrow_mut().a = 0.55;
            dl.borrow_mut().cmds.push(slider_edge.clone());
        }
        self.popClip(dl);
    }
    pub(crate) fn paintMinimapLine(&self, dl: &Rc<RefCell<EVGDisplayList>>, line: i64, x0: f64, y: f64, max_w: f64, h: f64) {
        if h < 0.5 {
            return;
        }
        let s: String = EditorBuffer::line_at(&self.buf, line);
        if s.is_empty() {
            return;
        }
        let mut char_w: f64 = self.minimapCharW;
        if char_w < 0.75 {
            char_w = 0.75;
        }
        let limit: f64 = x0 + max_w;
        let mut toks: Rc<RefCell<CodeTokenLine>> = self.tokensFor(line);
        let n: i64 = toks.borrow().count();
        if n == 0 {
            let mut _tmp_3731 = self.theme.borrow().plain.clone();
            self.paintMinimapSpan(dl,
                &s,
                0,
                (CeNative::str_len(&s)),
                x0,
                y,
                h,
                char_w,
                limit,
                &_tmp_3731
            );
            return;
        }
        let mut i: i64 = 0;
        while i < n {
            let mut t: Rc<RefCell<CodeToken>> = toks.borrow().at(i);
            let mut col: Rc<RefCell<EVGColor>> = self.theme.borrow().forKind(t.borrow().kind.clone());
            let mut _tmp_3734 = t.borrow().start.clone();
            let mut _tmp_3735 = t.borrow().length.clone();
            self.paintMinimapSpan(dl,
                &s,
                _tmp_3734,
                _tmp_3735,
                x0,
                y,
                h,
                char_w,
                limit,
                &col
            );
            i += 1;
        }
    }
    /// One faint rectangle per run of letters that share a colour and a
    /// fill. Neighbouring `l`s stay one command; a `.` next to an `R` does
    /// not, because they do not occupy the cell the same way.
    pub(crate) fn paintMinimapSpan(&self, dl: &Rc<RefCell<EVGDisplayList>>, s: &str, from: i64, __len: i64, x0: f64, y: f64, h: f64, char_w: f64, limit: f64, ink: &Rc<RefCell<EVGColor>>) {
        let n: i64 = CeNative::str_len(&s);
        let mut i: i64 = from;
        let mut stop: i64 = from + __len;
        if stop > n {
            stop = n;
        }
        let mut run_at: i64 = -1;
        let mut run_n: i64 = 0;
        let mut run_fill: f64 = 0.0;
        while i < stop {
            let ch: i64 = CeNative::char_at(&s, i);
            let fill: f64 = ScriptEditor::minimapInk(ch);
            if fill < 0.001 {
                if run_at >= 0 {
                    self.paintMinimapDots(dl,
                        x0,
                        y,
                        h,
                        char_w,
                        limit,
                        run_at,
                        run_n,
                        ink,
                        run_fill
                    );
                    run_at = -1;
                    run_n = 0;
                }
            } else {
                if run_at < 0 {
                    run_at = i;
                    run_n = 1;
                    run_fill = fill;
                } else {
                    if fill == run_fill {
                        run_n += 1;
                    } else {
                        self.paintMinimapDots(dl,
                            x0,
                            y,
                            h,
                            char_w,
                            limit,
                            run_at,
                            run_n,
                            ink,
                            run_fill
                        );
                        run_at = i;
                        run_n = 1;
                        run_fill = fill;
                    }
                }
            }
            i += 1;
        }
        if run_at >= 0 {
            self.paintMinimapDots(dl,
                x0,
                y,
                h,
                char_w,
                limit,
                run_at,
                run_n,
                ink,
                run_fill
            );
        }
    }
    pub(crate) fn paintMinimapDots(&self, dl: &Rc<RefCell<EVGDisplayList>>, x0: f64, y: f64, h: f64, char_w: f64, limit: f64, col: i64, __len: i64, ink: &Rc<RefCell<EVGColor>>, fill: f64) {
        if __len <= 0 {
            return;
        }
        if fill < 0.001 {
            return;
        }
        let rx: f64 = x0 + (col as f64) * char_w;
        if rx >= limit {
            return;
        }
        let mut rw: f64 = (__len as f64) * char_w;
        if rx + rw > limit {
            rw = limit - rx;
        }
        if rw < 0.5 {
            return;
        }
        let r: i64 = EVGColor::red(&ink);
        let g: i64 = EVGColor::green(&ink);
        let b: i64 = EVGColor::blue(&ink);
        let base_a: f64 = EVGColor::alpha(&ink);
        let a: f64 = base_a * fill;
        let mut speck: Rc<RefCell<EVGColor>> = EVGColor::rgba(r, g, b, a);
        self.pushRect(dl, rx, y, rw, h, &speck);
    }
    /// --- editing --------------------------------------------------------------
    pub(crate) fn markEdited(&mut self, p: &Rc<RefCell<EditorPos>>) {
        self.lastEditMs = wall_clock_ms();
        let mut _tmp_3737 = p.borrow().line.clone();
        let mut _tmp_3738 = p.borrow().col.clone();
        EditorSelection::set_caret(&self.sel.clone(), _tmp_3737, _tmp_3738);
        EditorSelection::collapse_to_caret(&self.sel.clone());
        self.layout.borrow_mut().prefer_col = self.sel.borrow().caret.borrow().col;
        let mut _tmp_3744 = self.buf.clone();
        let mut _tmp_3745 = self.sel.clone();
        EditorLayout::ensure_caret_visible(&self.layout.clone(), &_tmp_3744, &_tmp_3745);
        self.ensureCaretX();
        self.dirty = true;
    }
    pub fn moveCaret(&mut self, line: i64, col: i64, extend: bool) {
        let mut p: Rc<RefCell<EditorPos>> = EditorPos::create(line, col);
        EditorBuffer::clamp_pos(&self.buf.clone(), &p);
        let mut _tmp_3750 = p.borrow().line.clone();
        let mut _tmp_3751 = p.borrow().col.clone();
        EditorSelection::set_caret(&self.sel.clone(), _tmp_3750, _tmp_3751);
        if !extend {
            EditorSelection::collapse_to_caret(&self.sel.clone());
        }
        let mut _tmp_3756 = self.buf.clone();
        let mut _tmp_3757 = self.sel.clone();
        EditorLayout::ensure_caret_visible(&self.layout.clone(), &_tmp_3756, &_tmp_3757);
        self.ensureCaretX();
    }
    pub fn deleteSelection(&mut self) -> bool {
        if EditorSelection::is_collapsed(&self.sel) {
            return false;
        }
        let mut a: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        let mut b: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        EditorSelection::normalized(&self.sel.clone(), &a, &b);
        let mut _tmp_3763 = a.borrow().line.clone();
        let mut _tmp_3764 = a.borrow().col.clone();
        let mut _tmp_3765 = b.borrow().line.clone();
        let mut _tmp_3766 = b.borrow().col.clone();
        let mut p: Rc<RefCell<EditorPos>> = EditorBuffer::delete_range(
            &self.buf,
            _tmp_3763,
            _tmp_3764,
            _tmp_3765,
            _tmp_3766
        );
        self.markEdited(&p);
        true
    }
    pub fn insert(&mut self, s: &str) {
        self.deleteSelection();
        let mut _tmp_3768 = self.sel.borrow().caret.borrow().line.clone();
        let mut _tmp_3769 = self.sel.borrow().caret.borrow().col.clone();
        let mut p: Rc<RefCell<EditorPos>> = EditorBuffer::insert_text(
            &self.buf,
            _tmp_3768,
            _tmp_3769,
            s
        );
        self.markEdited(&p);
    }
    /// Enter keeps the indent it found, and opens one more level after a line
    /// that ends in `{`, `(` or `[` — the smallest amount of help that stops a
    /// script from drifting left as it is written.
    pub fn insertNewline(&mut self) {
        self.deleteSelection();
        let line: String = EditorBuffer::line_at(&self.buf, self.sel.borrow().caret.borrow().line.clone());
        let mut indent: String = String::new();
        let mut i: i64 = 0;
        let n: i64 = CeNative::str_len(&line);
        while i < n {
            let ch: i64 = CeNative::char_at(&line, i);
            if ch == 32 {
                indent.push_str(" ");
            } else {
                if ch == 9 {
                    indent.push_str("  ");
                } else {
                    break;
                }
            }
            i += 1;
        }
        let upto: String = CeNative::substr(&line, 0, self.sel.borrow().caret.borrow().col);
        let trimmed: String = upto.trim().to_string();
        let tn: i64 = CeNative::str_len(&trimmed);
        if tn > 0 {
            let last_ch: i64 = CeNative::char_at(&trimmed, tn - 1);
            if last_ch == 123 || last_ch == 40 {
                indent.push_str("  ");
            }
            if last_ch == 91 {
                indent.push_str("  ");
            }
        }
        let opened: String = format!("\n{}", indent);
        self.insert(&opened);
    }
    /// Shift every selected line one indent unit. A collapsed caret + Tab still
    /// inserts two spaces at the caret (so Tab in the middle of a word does
    /// what people type); a range is always a block indent, never a replace.
    pub fn indentSelection(&mut self, outdent: bool) {
        self.closeCompletions();
        if EditorSelection::is_collapsed(&self.sel) && !outdent {
            self.insert("  ");
            return;
        }
        let mut a: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        let mut b: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        EditorSelection::normalized(&self.sel.clone(), &a, &b);
        let mut last: i64 = b.borrow().line;
        if b.borrow().col == 0 && last > a.borrow().line {
            last -= 1;
        }
        self.shiftLines(a.borrow().line.clone(), last, outdent);
    }
    pub fn shiftLines(&mut self, first: i64, last: i64, outdent: bool) {
        if last < first {
            return;
        }
        let mut a: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        let mut b: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        EditorSelection::normalized(&self.sel.clone(), &a, &b);
        let caret_at_end: bool = self.sel.borrow()
            .caret.borrow()
            .line == b.borrow()
            .line && self.sel.borrow()
            .caret.borrow()
            .col == b.borrow()
            .col;
        let mut a_delta: i64 = 0;
        let mut b_delta: i64 = 0;
        let mut built: String = String::new();
        let mut i: i64 = first;
        while i <= last {
            let line: String = EditorBuffer::line_at(&self.buf, i);
            let mut delta: i64 = 0;
            let mut shifted: String = line.clone();
            if outdent {
                let n: i64 = ScriptEditor::leadingOutdent(&line);
                shifted = CeNative::substr(&line, n, (CeNative::str_len(&line)));
                delta = 0 - n;
            } else {
                shifted = format!("  {}", line);
                delta = 2;
            }
            if i > first {
                built.push_str("\n");
            }
            built.push_str(&shifted);
            if i == a.borrow().line {
                a_delta = delta;
            }
            if i == b.borrow().line {
                b_delta = delta;
            }
            i += 1;
        }
        let last_len: i64 = CeNative::str_len(&EditorBuffer::line_at(&self.buf, last));
        EditorBuffer::replace_range(&self.buf.clone(), first, 0, last, last_len, built.clone(), true);
        let a_col: i64 = ScriptEditor::adjustIndentCol(a.borrow().col.clone(), a_delta);
        let b_col: i64 = ScriptEditor::adjustIndentCol(b.borrow().col.clone(), b_delta);
        if caret_at_end {
            let mut _tmp_3785 = a.borrow().line.clone();
            EditorSelection::set_anchor(&self.sel.clone(), _tmp_3785, a_col);
            let mut _tmp_3788 = b.borrow().line.clone();
            EditorSelection::set_caret(&self.sel.clone(), _tmp_3788, b_col);
        } else {
            let mut _tmp_3791 = b.borrow().line.clone();
            EditorSelection::set_anchor(&self.sel.clone(), _tmp_3791, b_col);
            let mut _tmp_3794 = a.borrow().line.clone();
            EditorSelection::set_caret(&self.sel.clone(), _tmp_3794, a_col);
        }
        self.layout.borrow_mut().prefer_col = self.sel.borrow().caret.borrow().col;
        let mut _tmp_3798 = self.buf.clone();
        let mut _tmp_3799 = self.sel.clone();
        EditorLayout::ensure_caret_visible(&self.layout.clone(), &_tmp_3798, &_tmp_3799);
        self.ensureCaretX();
        self.lastEditMs = wall_clock_ms();
        self.dirty = true;
    }
    pub fn backspace(&mut self) {
        if self.deleteSelection() {
            self.refreshCompletions();
            return;
        }
        let line: i64 = self.sel.borrow().caret.borrow().line;
        let col: i64 = self.sel.borrow().caret.borrow().col;
        if col == 0 {
            if line == 0 {
                return;
            }
            let prev_len: i64 = CeNative::str_len(&EditorBuffer::line_at(&self.buf, line - 1));
            let mut p: Rc<RefCell<EditorPos>> = EditorBuffer::delete_range(
                &self.buf,
                line - 1,
                prev_len,
                line,
                0
            );
            self.markEdited(&p);
            return;
        }
        let mut before: Rc<RefCell<EditorPos>> = EditorBuffer::move_codepoint(
            &self.buf,
            line,
            col,
            -1
        );
        let mut _tmp_3805 = before.borrow().line.clone();
        let mut _tmp_3806 = before.borrow().col.clone();
        let mut p2: Rc<RefCell<EditorPos>> = EditorBuffer::delete_range(
            &self.buf,
            _tmp_3805,
            _tmp_3806,
            line,
            col
        );
        self.markEdited(&p2);
        self.refreshCompletions();
    }
    pub fn deleteForward(&mut self) {
        if self.deleteSelection() {
            return;
        }
        let line: i64 = self.sel.borrow().caret.borrow().line;
        let col: i64 = self.sel.borrow().caret.borrow().col;
        let __len: i64 = CeNative::str_len(&EditorBuffer::line_at(&self.buf, line));
        if col >= __len {
            if line >= EditorBuffer::line_count(&self.buf) - 1 {
                return;
            }
            let mut p: Rc<RefCell<EditorPos>> = EditorBuffer::delete_range(
                &self.buf,
                line,
                col,
                line + 1,
                0
            );
            self.markEdited(&p);
            return;
        }
        let mut after: Rc<RefCell<EditorPos>> = EditorBuffer::move_codepoint(
            &self.buf,
            line,
            col,
            1
        );
        let mut _tmp_3812 = after.borrow().line.clone();
        let mut _tmp_3813 = after.borrow().col.clone();
        let mut p2: Rc<RefCell<EditorPos>> = EditorBuffer::delete_range(
            &self.buf,
            line,
            col,
            _tmp_3812,
            _tmp_3813
        );
        self.markEdited(&p2);
    }
    pub fn moveVertical(&mut self, delta: i64, extend: bool) {
        let mut want: i64 = self.layout.borrow().prefer_col;
        if want < self.sel.borrow().caret.borrow().col {
            want = self.sel.borrow().caret.borrow().col;
        }
        let mut line: i64 = self.sel.borrow().caret.borrow().line + delta;
        if line < 0 {
            line = 0;
        }
        let max_line: i64 = EditorBuffer::line_count(&self.buf) - 1;
        if line > max_line {
            line = max_line;
        }
        let __len: i64 = CeNative::str_len(&EditorBuffer::line_at(&self.buf, line));
        let mut col: i64 = want;
        if col > __len {
            col = __len;
        }
        let keep: i64 = want;
        self.moveCaret(line, col, extend);
        self.layout.borrow_mut().prefer_col = keep;
    }
    /// --- the clipboard's half of the model --------------------------------------
    /// The host owns the system clipboard; what belongs here is the text a
    /// selection stands for, across as many lines as it covers. The browser
    /// page used to let the hidden textarea answer Ctrl+C by itself, which is
    /// right for a selection inside one line and silently WRONG for any other:
    /// the mirror only ever holds the caret's line, so a three-line selection
    /// copied one line and lost the rest without saying so.
    pub fn selectionText(&self) -> String {
        if EditorSelection::is_collapsed(&self.sel) {
            return String::new();
        }
        let mut a: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        let mut b: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        EditorSelection::normalized(&self.sel.clone(), &a, &b);
        let mut _tmp_3820 = a.borrow().line.clone();
        let mut _tmp_3821 = a.borrow().col.clone();
        let mut _tmp_3822 = b.borrow().line.clone();
        let mut _tmp_3823 = b.borrow().col.clone();
        return EditorBuffer::text_in_range(
            &self.buf,
            _tmp_3820,
            _tmp_3821,
            _tmp_3822,
            _tmp_3823
        ).clone();
    }
    /// Cut is copy plus this, and the host does them in that order.
    pub fn cutSelection(&mut self) -> String {
        let text: String = self.selectionText();
        if text.is_empty() {
            return String::new();
        }
        self.deleteSelection();
        text
    }
    pub fn selectAll(&mut self) {
        let mut _tmp_3825 = self.buf.clone();
        EditorSelection::select_all(&self.sel.clone(), &_tmp_3825);
        let mut _tmp_3828 = self.buf.clone();
        let mut _tmp_3829 = self.sel.clone();
        EditorLayout::ensure_caret_visible(&self.layout.clone(), &_tmp_3828, &_tmp_3829);
        self.ensureCaretX();
    }
    pub fn undo(&mut self) {
        let mut p: Rc<RefCell<EditorPos>> = EditorBuffer::undo(&self.buf);
        self.markEdited(&p);
    }
    pub fn redo(&mut self) {
        let mut p: Rc<RefCell<EditorPos>> = EditorBuffer::redo(&self.buf);
        self.markEdited(&p);
    }
    /// --- word and document movement -------------------------------------------
    /// Ctrl+arrow, Ctrl+Home and Ctrl+End are not decoration: they are how a
    /// keyboard-only user crosses a file. Word boundaries come from EditorWord,
    /// the same ones double-click selection uses, so a word means one thing in
    /// this editor rather than two.
    pub fn wordLeftCol(&self, line: i64, col: i64) -> i64 {
        let s: String = EditorBuffer::line_at(&self.buf, line);
        let mut i: i64 = col;
        while i > 0 {
            let ch: i64 = CeNative::char_at(&s, i - 1);
            if EditorWord::is_word_char(ch) {
                break;
            }
            i -= 1;
        }
        if i == 0 {
            return 0;
        }
        EditorWord::word_start(&s, i - 1)
    }
    pub fn wordRightCol(&self, line: i64, col: i64) -> i64 {
        let s: String = EditorBuffer::line_at(&self.buf, line);
        let n: i64 = CeNative::str_len(&s);
        let mut i: i64 = col;
        while i < n {
            let ch: i64 = CeNative::char_at(&s, i);
            if EditorWord::is_word_char(ch) {
                break;
            }
            i += 1;
        }
        if i >= n {
            return n;
        }
        EditorWord::word_end(&s, i)
    }
    pub fn moveWord(&mut self, dir: i64, extend: bool) {
        let line: i64 = self.sel.borrow().caret.borrow().line;
        let col: i64 = self.sel.borrow().caret.borrow().col;
        if dir < 0 {
            if col == 0 {
                if line == 0 {
                    return;
                }
                let prev_len: i64 = CeNative::str_len(&EditorBuffer::line_at(&self.buf, line - 1));
                self.moveCaret(line - 1, prev_len, extend);
                self.layout.borrow_mut().prefer_col = self.sel.borrow().caret.borrow().col;
                return;
            }
            let mut target: i64 = self.wordLeftCol(line, col);
            if target >= col {
                target = col - 1;
            }
            self.moveCaret(line, target, extend);
            self.layout.borrow_mut().prefer_col = self.sel.borrow().caret.borrow().col;
            return;
        }
        let __len: i64 = CeNative::str_len(&EditorBuffer::line_at(&self.buf, line));
        if col >= __len {
            if line >= EditorBuffer::line_count(&self.buf) - 1 {
                return;
            }
            self.moveCaret(line + 1, 0, extend);
            self.layout.borrow_mut().prefer_col = 0;
            return;
        }
        let mut right: i64 = self.wordRightCol(line, col);
        if right <= col {
            right = col + 1;
        }
        self.moveCaret(line, right, extend);
        self.layout.borrow_mut().prefer_col = self.sel.borrow().caret.borrow().col;
    }
    pub fn moveDocumentStart(&mut self, extend: bool) {
        self.moveCaret(0, 0, extend);
        self.layout.borrow_mut().prefer_col = 0;
    }
    pub fn moveDocumentEnd(&mut self, extend: bool) {
        let last: i64 = EditorBuffer::line_count(&self.buf) - 1;
        let __len: i64 = CeNative::str_len(&EditorBuffer::line_at(&self.buf, last));
        self.moveCaret(last, __len, extend);
        self.layout.borrow_mut().prefer_col = __len;
    }
    /// True when it did the collapsing: an open selection and no Shift.
    pub fn collapseToEdge(&mut self, to_end: bool, shift: bool) -> bool {
        if shift {
            return false;
        }
        if EditorSelection::is_collapsed(&self.sel) {
            return false;
        }
        let mut a: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        let mut b: Rc<RefCell<EditorPos>> = Rc::new(RefCell::new(EditorPos::new()));
        EditorSelection::normalized(&self.sel.clone(), &a, &b);
        if to_end {
            let mut _tmp_3846 = b.borrow().line.clone();
            self.moveCaret(_tmp_3846, b.borrow().col.clone(), false);
        } else {
            let mut _tmp_3848 = a.borrow().line.clone();
            self.moveCaret(_tmp_3848, a.borrow().col.clone(), false);
        }
        self.layout.borrow_mut().prefer_col = self.sel.borrow().caret.borrow().col;
        true
    }
    /// --- input ----------------------------------------------------------------
    pub fn handleKey(&mut self, key: i64, shift: bool, ctrl: bool) -> bool {
        // The find bar owns Escape, Enter and Backspace while it is up; the
        // arrows and the wheel keep working, so you can look around without
        // losing the query.
        if self.findOpen {
            if key == UIKey::escape() {
                self.closeFind();
                return true;
            }
            if key == UIKey::enter() {
                self.findNext(shift);
                return true;
            }
            if key == UIKey::backspace() {
                self.findBackspace();
                return true;
            }
        }
        // While the list is up it owns the keys that mean "which one" — and
        // only those. Everything else keeps its meaning, which is what makes a
        // completion list something you can ignore.
        if self.completionOpen {
            if key == UIKey::up() {
                self.moveCompletion(-1);
                return true;
            }
            if key == UIKey::down() {
                self.moveCompletion(1);
                return true;
            }
            if key == UIKey::enter() {
                return self.acceptCompletion();
            }
            if key == UIKey::tab() {
                return self.acceptCompletion();
            }
            if key == UIKey::escape() {
                self.closeCompletions();
                return true;
            }
            if key == UIKey::left() {
                self.closeCompletions();
            }
            if key == UIKey::right() {
                self.closeCompletions();
            }
        }
        if key == UIKey::left() {
            if ctrl {
                self.moveWord(-1, shift);
                return true;
            }
            // A plain arrow with a selection open COLLAPSES it to the edge it
            // points at — it does not also move a character. That is what every
            // text field does, and what CodeMirror does; the bench in
            // web/editor/compare is where the difference showed up.
            if self.collapseToEdge(false, shift) {
                return true;
            }
            let mut _tmp_3852 = self.sel.borrow().caret.borrow().line.clone();
            let mut _tmp_3853 = self.sel.borrow().caret.borrow().col.clone();
            let mut p: Rc<RefCell<EditorPos>> = EditorBuffer::move_codepoint(
                &self.buf,
                _tmp_3852,
                _tmp_3853,
                -1
            );
            let mut _tmp_3854 = p.borrow().line.clone();
            self.moveCaret(_tmp_3854, p.borrow().col.clone(), shift);
            self.layout.borrow_mut().prefer_col = self.sel.borrow().caret.borrow().col;
            return true;
        }
        if key == UIKey::right() {
            if ctrl {
                self.moveWord(1, shift);
                return true;
            }
            if self.collapseToEdge(true, shift) {
                return true;
            }
            let mut _tmp_3858 = self.sel.borrow().caret.borrow().line.clone();
            let mut _tmp_3859 = self.sel.borrow().caret.borrow().col.clone();
            let mut p2: Rc<RefCell<EditorPos>> = EditorBuffer::move_codepoint(
                &self.buf,
                _tmp_3858,
                _tmp_3859,
                1
            );
            let mut _tmp_3860 = p2.borrow().line.clone();
            self.moveCaret(_tmp_3860, p2.borrow().col.clone(), shift);
            self.layout.borrow_mut().prefer_col = self.sel.borrow().caret.borrow().col;
            return true;
        }
        if key == UIKey::up() {
            self.moveVertical(-1, shift);
            return true;
        }
        if key == UIKey::down() {
            self.moveVertical(1, shift);
            return true;
        }
        if key == UIKey::page_up() {
            let step: i64 = EditorLayout::visible_line_count(&self.layout);
            self.moveVertical(0 - step, shift);
            return true;
        }
        if key == UIKey::page_down() {
            let step2: i64 = EditorLayout::visible_line_count(&self.layout);
            self.moveVertical(step2, shift);
            return true;
        }
        if key == UIKey::home() {
            if ctrl {
                self.moveDocumentStart(shift);
                return true;
            }
            let line: i64 = self.sel.borrow().caret.borrow().line;
            self.moveCaret(line, 0, shift);
            self.layout.borrow_mut().prefer_col = 0;
            return true;
        }
        if key == UIKey::end() {
            if ctrl {
                self.moveDocumentEnd(shift);
                return true;
            }
            let line: i64 = self.sel.borrow().caret.borrow().line;
            let __len: i64 = CeNative::str_len(&EditorBuffer::line_at(&self.buf, line));
            self.moveCaret(line, __len, shift);
            self.layout.borrow_mut().prefer_col = __len;
            return true;
        }
        if key == UIKey::backspace() {
            self.backspace();
            return true;
        }
        if key == UIKey::del() {
            self.deleteForward();
            return true;
        }
        if key == UIKey::enter() {
            self.insertNewline();
            return true;
        }
        if key == UIKey::tab() {
            // A range is a block to indent, not text to replace — the same
            // rule VS Code and CodeMirror use. A collapsed caret still inserts
            // two spaces; Shift+Tab outdents the line the caret is on.
            self.indentSelection(shift);
            return true;
        }
        false
    }
    /// Ctrl-chords the editor answers itself. Run and Save belong to the window
    /// around it, which is why they are not here.
    pub fn handleCtrlChar(&mut self, ch: i64) -> bool {
        if ch == 102 || ch == 70 {
            return self.openFind();
        }
        if ch == 103 || ch == 71 {
            if self.findOpen {
                return self.findNext(false);
            }
        }
        if ch == 97 || ch == 65 {
            self.selectAll();
            return true;
        }
        if ch == 122 || ch == 90 {
            self.undo();
            return true;
        }
        if ch == 121 || ch == 89 {
            self.redo();
            return true;
        }
        false
    }
    pub fn handleText(&mut self, s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        if self.findOpen {
            self.findType(s);
            return true;
        }
        self.insert(s);
        // Two word characters is the point where a guess is worth making. Under
        // that the list is everything, which is not a suggestion.
        if self.completionOpen {
            self.refreshCompletions();
        } else {
            if self.autoComplete {
                let line: String = EditorBuffer::line_at(&self.buf, self.sel.borrow().caret.borrow().line.clone());
                let last_ch: i64 = CeNative::char_at(&s, (CeNative::str_len(&s)) - 1);
                // A dot is a question on its own — `body.` is asking what a
                // body has — so it opens the list with no prefix at all.
                if last_ch == 46 {
                    self.openCompletions();
                } else {
                    let prefix: String = LanguageText::prefixAt(&line, self.sel.borrow().caret.borrow().col.clone());
                    if (CeNative::str_len(&prefix)) >= 2 {
                        self.openCompletions();
                    }
                }
            }
        }
        true
    }
    /// Ctrl+Space: ask for the list whatever the prefix is.
    pub fn requestCompletions(&mut self) -> bool {
        self.openCompletions()
    }
    pub(crate) fn pointerDown(&mut self, tr: Rc<RefCell<UITextRenderer>>, px: i64, py: i64, extend: bool) {
        self.focused = true;
        self.rememberTr(tr.clone());
        self.closeCompletions();
        if self.tryMinimapPointer(px, py) {
            return;
        }
        if self.tryHBarPointer(px, py) {
            return;
        }
        let mut _tmp_3873 = self.buf.clone();
        let mut hit: Rc<RefCell<EditorPos>> = EditorLayout::hit_test(
            &self.layout,
            &tr,
            &_tmp_3873,
            px,
            py
        );
        let mut _tmp_3874 = hit.borrow().line.clone();
        self.moveCaret(_tmp_3874, hit.borrow().col.clone(), extend);
        self.layout.borrow_mut().prefer_col = self.sel.borrow().caret.borrow().col;
        if !extend {
            let mut _tmp_3878 = hit.borrow().line.clone();
            let mut _tmp_3879 = hit.borrow().col.clone();
            EditorSelection::set_anchor(&self.sel.clone(), _tmp_3878, _tmp_3879);
        }
    }
    /// Double click takes the word under the pointer, triple click the line —
    /// the two selections every text surface has, and the ones `EditorWord`
    /// already knows how to find, so a word means the same thing here as it does
    /// to Ctrl+arrow.
    pub(crate) fn selectWordAt(&mut self, tr: &Rc<RefCell<UITextRenderer>>, px: i64, py: i64) {
        if self.tryMinimapPointer(px, py) {
            return;
        }
        let mut _tmp_3882 = self.buf.clone();
        let mut hit: Rc<RefCell<EditorPos>> = EditorLayout::hit_test(
            &self.layout,
            tr,
            &_tmp_3882,
            px,
            py
        );
        let mut _tmp_3883 = hit.borrow().line.clone();
        self.selectWordAtPos(_tmp_3883, hit.borrow().col.clone());
    }
    /// The same selection without a hit test — hosts that already know the
    /// column, and the tests, use this.
    pub fn selectWordAtPos(&mut self, line: i64, col: i64) {
        self.focused = true;
        self.closeCompletions();
        let mut p: Rc<RefCell<EditorPos>> = EditorPos::create(line, col);
        EditorBuffer::clamp_pos(&self.buf.clone(), &p);
        let s: String = EditorBuffer::line_at(&self.buf, p.borrow().line.clone());
        let mut from: i64 = EditorWord::word_start(&s, p.borrow().col.clone());
        let mut to: i64 = EditorWord::word_end(&s, p.borrow().col.clone());
        if from == to {
            // Clicked whitespace or punctuation — take the character under
            // the pointer, which is what a double-click on a gap does.
            if p.borrow().col < (CeNative::str_len(&s)) {
                from = p.borrow().col;
                to = p.borrow().col + 1;
            }
        }
        let mut _tmp_3892 = p.borrow().line.clone();
        EditorSelection::set_anchor(&self.sel.clone(), _tmp_3892, from);
        let mut _tmp_3895 = p.borrow().line.clone();
        EditorSelection::set_caret(&self.sel.clone(), _tmp_3895, to);
        self.layout.borrow_mut().prefer_col = to;
        let mut _tmp_3898 = self.buf.clone();
        let mut _tmp_3899 = self.sel.clone();
        EditorLayout::ensure_caret_visible(&self.layout.clone(), &_tmp_3898, &_tmp_3899);
        self.ensureCaretX();
    }
    pub(crate) fn selectLineAt(&mut self, tr: &Rc<RefCell<UITextRenderer>>, px: i64, py: i64) {
        if self.tryMinimapPointer(px, py) {
            return;
        }
        self.focused = true;
        self.closeCompletions();
        let mut _tmp_3902 = self.buf.clone();
        let mut hit: Rc<RefCell<EditorPos>> = EditorLayout::hit_test(
            &self.layout,
            tr,
            &_tmp_3902,
            px,
            py
        );
        let __len: i64 = CeNative::str_len(&EditorBuffer::line_at(&self.buf, hit.borrow().line.clone()));
        let mut _tmp_3906 = hit.borrow().line.clone();
        EditorSelection::set_anchor(&self.sel.clone(), _tmp_3906, 0);
        let mut _tmp_3909 = hit.borrow().line.clone();
        EditorSelection::set_caret(&self.sel.clone(), _tmp_3909, __len);
        self.layout.borrow_mut().prefer_col = __len;
        let mut _tmp_3912 = self.buf.clone();
        let mut _tmp_3913 = self.sel.clone();
        EditorLayout::ensure_caret_visible(&self.layout.clone(), &_tmp_3912, &_tmp_3913);
        self.ensureCaretX();
    }
    pub(crate) fn pointerDrag(&mut self, tr: Rc<RefCell<UITextRenderer>>, px: i64, py: i64) {
        self.rememberTr(tr.clone());
        if self.hBarDrag {
            self.jumpHBar(px);
            return;
        }
        if self.minimapDrag {
            self.jumpMinimap(py);
            return;
        }
        let mut _tmp_3916 = self.buf.clone();
        let mut hit: Rc<RefCell<EditorPos>> = EditorLayout::hit_test(
            &self.layout,
            &tr,
            &_tmp_3916,
            px,
            py
        );
        let mut _tmp_3917 = hit.borrow().line.clone();
        self.moveCaret(_tmp_3917, hit.borrow().col.clone(), true);
    }
    pub fn pointerUp(&mut self) {
        self.minimapDrag = false;
        self.hBarDrag = false;
    }
    pub fn scrollBy(&self, notches: i64) {
        let mut _tmp_3920 = self.buf.clone();
        EditorLayout::scroll_by_lines(&self.layout.clone(), &_tmp_3920, 0 - notches);
    }
}
