// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::code_tokens::CodeTokenLine;
use crate::editor_diagnostic::EditorDiagnostic;
use crate::editor_language::EditorLanguage;
use crate::editor_language::LanguageRegistry;
use crate::js_language::JsLanguage;
use crate::ranger_language::RangerLanguage;
use crate::rgr::CeNative;
use crate::rgr::EditorBuffer;
use crate::script_editor::ScriptEditor;
use crate::rgr::{FontManager, EVGColor, EVGDisplayList, EVGDrawCmd, SoftCanvas, UIContext, UIInput, UITextRenderer};

/// ==============================================================================
/// CodeEditorPage.rgr — the code editor, on its own page, on both backends
/// ==============================================================================
///
/// The editor is a panel that paints into a display list. This is the smallest
/// thing that can hold one: a header, the panel, a status bar, and the input
/// loop. It exists so the editor can be looked at and typed into WITHOUT a
/// spreadsheet around it — and so the same page can be run twice, once through
/// WebGL in a browser and once through SDL2 + OpenGL as a native binary, from
/// one source.
///
///   CodeEditorPage.buildDisplayList()  ──►  EVGDisplayList
///                                             ├─ evg-webgl.js      (browser)
///                                             └─ EvgGlPainter      (SDL2 + GL)
///
/// Nothing below the display list is in this file, and nothing above it is in
/// the hosts: `web/code_editor_web.rgr` and `platform/sdl/code_editor_sdl.rgr`
/// both do the same three things — make a page, feed it UIInput, draw its list.
/// ==============================================================================
pub struct CodeEditorPage {
    pub editor: Rc<RefCell<ScriptEditor>>,
    pub(crate) canvas: Rc<RefCell<SoftCanvas>>,
    pub(crate) ctx: Rc<RefCell<UIContext>>,
    pub(crate) input: Rc<RefCell<UIInput>>,
    pub W: i64,
    pub H: i64,
    pub headerH: i64,
    pub statusH: i64,
    pub frame: i64,
    pub status: String,
    pub docName: String,
    pub sample: i64,
    /// How long the last full re-lex took, in microseconds. A code editor that
    /// re-lexes the whole document on every keystroke is fine until it is not,
    /// and the only honest way to know which one this is, is to print the
    /// number. Milliseconds would read 0 for every document worth typing into.
    pub lexUs: i64,
    pub lastCmds: i64,
    pub fontFamily: String,
    /// The installed language plugins. THIS is the whole installation step: a
    /// host that wants a third language constructs it and adds it here, and
    /// nothing else in the editor changes. Nothing is spawned, connected to or
    /// serialized — see lang/EditorLanguage.rgr for why that is deliberate.
    pub languages: Rc<RefCell<LanguageRegistry>>,
    pub(crate) bg: Rc<RefCell<EVGColor>>,
    pub(crate) barBg: Rc<RefCell<EVGColor>>,
    pub(crate) barText: Rc<RefCell<EVGColor>>,
    pub(crate) barMuted: Rc<RefCell<EVGColor>>,
    pub(crate) accent: Rc<RefCell<EVGColor>>,
}

impl CodeEditorPage {
    pub fn new() -> Self {
        Self {
            editor: Rc::new(RefCell::new(ScriptEditor::new())),
            canvas: Rc::new(RefCell::new(SoftCanvas::new())),
            ctx: Rc::new(RefCell::new(UIContext::new())),
            input: Rc::new(RefCell::new(UIInput::new())),
            W: 1000,
            H: 700,
            headerH: 30,
            statusH: 24,
            frame: 0,
            status: String::new(),
            docName: "report.tsx".to_string(),
            sample: 0,
            lexUs: 0,
            lastCmds: 0,
            fontFamily: "Open Sans".to_string(),
            languages: Rc::new(RefCell::new(LanguageRegistry::new())),
            bg: EVGColor::rgb(15, 16, 22),
            barBg: EVGColor::rgb(30, 32, 44),
            barText: EVGColor::rgb(196, 205, 226),
            barMuted: EVGColor::rgb(118, 128, 154),
            accent: EVGColor::rgb(122, 162, 247),
        }
    }
    /// The first document: a short report script, the kind the editor is for.
    pub fn reportSampler() -> String {
        let mut out: String = String::new();
        out.push_str(&"// A report over a table of sales.\n".to_string());
        out.push_str(&"// It prints a header, one row per region and a total.\n".to_string());
        out.push_str(&"const ROWS = [\n".to_string());
        out.push_str(&"  { region: 'North', units: 1240, price: 19.5 },\n".to_string());
        out.push_str(&"  { region: 'South', units: 860, price: 21.0 },\n".to_string());
        out.push_str(&"  { region: 'East', units: 1515, price: 18.25 },\n".to_string());
        out.push_str(&"  { region: 'West', units: 990, price: 20.75 },\n".to_string());
        out.push_str(&"];\n".to_string());
        out.push_str(&"const INK = '#1f2933';\n".to_string());
        out.push_str(&"\n".to_string());
        out.push_str(&"function line(values, weight) {\n".to_string());
        out.push_str(&"  const cells = [];\n".to_string());
        out.push_str(&"  for (let c = 0; c < values.length; c++) {\n".to_string());
        out.push_str(&"    cells.push(<Label width='24%' fontWeight={weight} color={INK}>{values[c]}</Label>);\n".to_string());
        out.push_str(&"  }\n".to_string());
        out.push_str(&"  return (<View flexDirection='row' padding='4px'>{cells}</View>);\n".to_string());
        out.push_str(&"}\n".to_string());
        out.push_str(&"\n".to_string());
        out.push_str(&"function report() {\n".to_string());
        out.push_str(&"  let total = 0;\n".to_string());
        out.push_str(&"  const body = ROWS.map((r) => {\n".to_string());
        out.push_str(&"    const sum = r.units * r.price;\n".to_string());
        out.push_str(&"    total += sum;\n".to_string());
        out.push_str(&"    return line([r.region, String(r.units), r.price.toFixed(2), sum.toFixed(2)], 'normal');\n".to_string());
        out.push_str(&"  });\n".to_string());
        out.push_str(&"  return (\n".to_string());
        out.push_str(&"    <View padding='24px'>\n".to_string());
        out.push_str(&"      {line(['Region', 'Units', 'Price', 'Sales'], 'bold')}\n".to_string());
        out.push_str(&"      {body}\n".to_string());
        out.push_str(&"      {line(['Total', '', '', total.toFixed(2)], 'bold')}\n".to_string());
        out.push_str(&"    </View>\n".to_string());
        out.push_str(&"  );\n".to_string());
        out.push_str(&"}\n".to_string());
        out
    }
    pub fn syntaxSampler() -> String {
        let mut out: String = String::new();
        out.push_str(&"// Every token kind this lexer knows, on one page.\n".to_string());
        out.push_str(&"/* A block comment\n".to_string());
        out.push_str(&"   that runs over three lines\n".to_string());
        out.push_str(&"   and closes here. */\n".to_string());
        out.push_str(&"const NAME = 'single';\n".to_string());
        out.push_str(&"const other = \"double \\\" escaped\";\n".to_string());
        out.push_str(&"const tpl = `template ${NAME} with ${1 + 2}`;\n".to_string());
        out.push_str(&"let n = 42, f = 12.5, hex = 0x1f, exp = 1e9;\n".to_string());
        out.push_str(&"\n".to_string());
        out.push_str(&"function totals(rows) {\n".to_string());
        out.push_str(&"  let sum = 0;\n".to_string());
        out.push_str(&"  for (let i = 1; i < rows.length; i++) {\n".to_string());
        out.push_str(&"    if (rows[i][1] > 0 && rows[i][0] != null) {\n".to_string());
        out.push_str(&"      sum += rows[i][1];\n".to_string());
        out.push_str(&"    }\n".to_string());
        out.push_str(&"  }\n".to_string());
        out.push_str(&"  return sum;\n".to_string());
        out.push_str(&"}\n".to_string());
        out.push_str(&"\n".to_string());
        out.push_str(&"function render() {\n".to_string());
        out.push_str(&"  const rows = sheetText('Sales');\n".to_string());
        out.push_str(&"  const total = formatNumber(totals(sheetRows('Sales')), '#,##0.00');\n".to_string());
        out.push_str(&"  log('rows: ' + rows.length);\n".to_string());
        out.push_str(&"  return (\n".to_string());
        out.push_str(&"    <Print>\n".to_string());
        out.push_str(&"      <Section>\n".to_string());
        out.push_str(&"        <Page>\n".to_string());
        out.push_str(&"          <View flexDirection='column' padding='16px'>\n".to_string());
        out.push_str(&"            <Label fontSize='22px' fontWeight='bold'>{param('sheet')}</Label>\n".to_string());
        out.push_str(&"            <Label width={120} color=\"#7aa2f7\">Total {total}</Label>\n".to_string());
        out.push_str(&"          </View>\n".to_string());
        out.push_str(&"        </Page>\n".to_string());
        out.push_str(&"      </Section>\n".to_string());
        out.push_str(&"    </Print>\n".to_string());
        out.push_str(&"  );\n".to_string());
        out.push_str(&"}\n".to_string());
        out
    }
    /// A document with real mistakes in it, so the page shows what a diagnostic
    /// looks like without anyone having to type one.
    pub fn brokenSampler() -> String {
        let mut out: String = String::new();
        out.push_str(&"// Three mistakes, on purpose. The squiggles are the language plugin's.\n".to_string());
        out.push_str(&"const rows = sheetRows('Sales');\n".to_string());
        out.push_str(&"\n".to_string());
        out.push_str(&"function totals(rows) {\n".to_string());
        out.push_str(&"  let sum = 0;\n".to_string());
        out.push_str(&"  for (let i = 1; i < rows.length; i++ {\n".to_string());
        out.push_str(&"    sum += rows[i][1];\n".to_string());
        out.push_str(&"  }\n".to_string());
        out.push_str(&"  return sum;\n".to_string());
        out.push_str(&"}\n".to_string());
        out
    }
    /// …and one in another language entirely, which is the point of there being
    /// a registry rather than a JavaScript checker wired into the editor.
    pub fn rangerSampler() -> String {
        let mut out: String = String::new();
        out.push_str(&"; Ranger, checked by a different plugin through the same seam.\n".to_string());
        out.push_str(&"class Report {\n".to_string());
        out.push_str(&"    def rows:[string]\n".to_string());
        out.push_str(&"\n".to_string());
        out.push_str(&"    fn total:int () {\n".to_string());
        out.push_str(&"        def n:int (array_length rows)\n".to_string());
        out.push_str(&"        return this.countOf(n)\n".to_string());
        out.push_str(&"    }\n".to_string());
        out.push_str(&"\n".to_string());
        out.push_str(&"    fn countOf:int (n:int) {\n".to_string());
        out.push_str(&"        (this.log(n))\n".to_string());
        out.push_str(&"        return (n + 1\n".to_string());
        out.push_str(&"    }\n".to_string());
        out.push_str(&"}\n".to_string());
        out
    }
    /// --- starting up ----------------------------------------------------------
    pub fn init(&mut self, w: i64, h: i64, font_dir: &str) {
        self.W = w;
        self.H = h;
        SoftCanvas::init(&self.canvas.clone(), w, h);
        let mut _tmp_3890 = self.canvas.clone();
        UIContext::attach(&self.ctx.clone(), _tmp_3890);
        UITextRenderer::add_font_dir(&self.ctx.borrow().tr.clone(), font_dir);
        let mut _tmp_3893 = self.fontFamily.clone();
        let ok: bool = UITextRenderer::load_font(
            &self.ctx.borrow().tr,
            _tmp_3893,
            "Open_Sans/OpenSans-Regular.ttf"
        );
        let fm: Rc<RefCell<FontManager>> = self.ctx.borrow().tr.borrow().fm.clone();
        FontManager::load_font(&fm, "Open_Sans/OpenSans-Bold.ttf");
        FontManager::load_font(&fm, "Noto_Sans/NotoSans-Regular.ttf");
        if !ok {
            self.status = "bitmap-font fallback".to_string();
        } else {
            self.status = "ready".to_string();
        }
        self.installLanguages();
        self.loadSample(0);
    }
    /// The browser has no directory to read faces from: the page fetches them
    /// and hands over the bytes.
    pub fn initBare(&mut self, w: i64, h: i64) {
        self.W = w;
        self.H = h;
        SoftCanvas::init(&self.canvas.clone(), w, h);
        let mut _tmp_3899 = self.canvas.clone();
        UIContext::attach(&self.ctx.clone(), _tmp_3899);
        self.status = "no fonts yet".to_string();
        self.installLanguages();
        self.loadSample(0);
    }
    pub fn addFont(&mut self, family: &str, data: &[u8]) -> bool {
        let ok: bool = UITextRenderer::load_font_bytes(
            &self.ctx.borrow().tr,
            family.to_string(),
            data
        );
        if ok {
            self.status = "ready".to_string();
        }
        ok
    }
    pub fn addFace(&self, data: &[u8]) -> bool {
        return UITextRenderer::add_face_bytes(&self.ctx.borrow().tr, data);
    }
    pub fn resize(&mut self, w: i64, h: i64) {
        if w < 240 {
            return;
        }
        if h < 200 {
            return;
        }
        self.W = w;
        self.H = h;
        SoftCanvas::init(&self.canvas.clone(), w, h);
        let mut _tmp_3904 = self.canvas.clone();
        UIContext::attach(&self.ctx.clone(), _tmp_3904);
        UITextRenderer::clear_cache(&self.ctx.borrow().tr.clone());
    }
    /// --- languages -------------------------------------------------------------
    pub fn installLanguages(&self) {
        if self.languages.borrow().count() > 0 {
            return;
        }
        self.languages.borrow_mut().add(Rc::new(RefCell::new(JsLanguage::new())));
        self.languages.borrow_mut().add(Rc::new(RefCell::new(RangerLanguage::new())));
    }
    /// Pick by file name and hand it to the editor. A document nothing claims
    /// gets the base plugin, which complains about nothing — an unknown file
    /// type is not an error.
    pub fn applyLanguage(&self) {
        let mut lang: Rc<RefCell<dyn EditorLanguage>> = self.languages.borrow().forFile(&self.docName.clone());
        self.editor.clone().borrow_mut().setLanguage(lang.clone());
    }
    pub fn languageId(&self) -> String {
        return (self.editor.borrow().language).borrow().id();
    }
    /// --- documents ------------------------------------------------------------
    /// Two of them, because a highlighter is only ever wrong on the case nobody
    /// put in front of it: one real report script, and one page of nothing but
    /// awkward tokens.
    pub fn loadSample(&mut self, which: i64) {
        self.sample = which;
        if which == 1 {
            self.docName = "syntax.tsx".to_string();
            let arg_0 = CodeEditorPage::syntaxSampler().clone();
            self.editor.clone().borrow_mut().init(&arg_0);
        } else {
            if which == 2 {
                self.docName = "broken.tsx".to_string();
                let arg_0 = CodeEditorPage::brokenSampler().clone();
                self.editor.clone().borrow_mut().init(&arg_0);
            } else {
                if which == 3 {
                    self.docName = "sample.rgr".to_string();
                    let arg_0 = CodeEditorPage::rangerSampler().clone();
                    self.editor.clone().borrow_mut().init(&arg_0);
                } else {
                    self.docName = "report.tsx".to_string();
                    let arg_0 = CodeEditorPage::reportSampler().clone();
                    self.editor.clone().borrow_mut().init(&arg_0);
                }
            }
        }
        self.applyLanguage();
        self.editor.borrow_mut().focused = true;
        let problems: i64 = self.editor.borrow_mut().problemCount();
        self.status = format!("loaded {} · {}", self.docName, self.languageId());
        if problems > 0 {
            self.status.push_str(&format!(" · {} problem(s)", problems));
        }
    }
    pub fn nextSample(&mut self) {
        let mut next: i64 = self.sample + 1;
        if next > 3 {
            next = 0;
        }
        self.loadSample(next);
    }
    pub fn text(&self) -> String {
        return self.editor.borrow().text();
    }
    pub fn setText(&mut self, src: &str) {
        self.editor.clone().borrow_mut().init(src);
        self.docName = "untitled.tsx".to_string();
        self.applyLanguage();
    }
    pub fn setNamedText(&mut self, name: String, src: &str) {
        self.editor.clone().borrow_mut().init(src);
        if !name.is_empty() {
            self.docName = name.clone();
        }
        self.applyLanguage();
    }
    /// --- input ----------------------------------------------------------------
    pub fn editorTop(&self) -> i64 {
        self.headerH
    }
    pub fn editorHeight(&self) -> i64 {
        let h: i64 = (self.H - self.headerH) - self.statusH;
        if h < 40 {
            return 40;
        }
        h
    }
    pub fn selectWordAt(&self, px: i64, py: i64) {
        let mut _tmp_3927 = self.editorTop();
        let mut _tmp_3928 = self.W.clone();
        let mut _tmp_3929 = self.editorHeight();
        self.editor.clone().borrow().setBounds(0, _tmp_3927, _tmp_3928, _tmp_3929);
        let mut _tmp_3932 = self.ctx.borrow().tr.clone();
        self.editor.clone().borrow_mut().selectWordAt(&_tmp_3932, px, py);
    }
    pub fn selectLineAt(&self, px: i64, py: i64) {
        let mut _tmp_3935 = self.editorTop();
        let mut _tmp_3936 = self.W.clone();
        let mut _tmp_3937 = self.editorHeight();
        self.editor.clone().borrow().setBounds(0, _tmp_3935, _tmp_3936, _tmp_3937);
        let mut _tmp_3940 = self.ctx.borrow().tr.clone();
        self.editor.clone().borrow_mut().selectLineAt(&_tmp_3940, px, py);
    }
    pub(crate) fn update(&mut self, inp: &Rc<RefCell<UIInput>>) {
        let mut _tmp_3943 = self.editorTop();
        let mut _tmp_3944 = self.W.clone();
        let mut _tmp_3945 = self.editorHeight();
        self.editor.clone().borrow().setBounds(0, _tmp_3943, _tmp_3944, _tmp_3945);
        if inp.borrow().pointer_pressed {
            let mut _tmp_3948 = inp.borrow().pointer_x.clone();
            if self.editor.borrow().insideRect(_tmp_3948, inp.borrow().pointer_y.clone()) {
                let mut _tmp_3951 = self.ctx.borrow().tr.clone();
                let mut _tmp_3952 = inp.borrow().pointer_x.clone();
                let mut _tmp_3953 = inp.borrow().pointer_y.clone();
                let mut _tmp_3954 = inp.borrow().shift_down.clone();
                self.editor.clone().borrow_mut().pointerDown(_tmp_3951.clone(), _tmp_3952, _tmp_3953, _tmp_3954);
            }
        } else {
            if inp.borrow().pointer_down {
                if self.editor.borrow().focused {
                    let mut _tmp_3957 = self.ctx.borrow().tr.clone();
                    let mut _tmp_3958 = inp.borrow().pointer_x.clone();
                    let mut _tmp_3959 = inp.borrow().pointer_y.clone();
                    self.editor.clone().borrow_mut().pointerDrag(_tmp_3957.clone(), _tmp_3958, _tmp_3959);
                }
            }
        }
        if inp.borrow().scroll_delta != 0 {
            let mut _tmp_3962 = inp.borrow().scroll_delta.clone();
            self.editor.clone().borrow().scrollBy(_tmp_3962);
        }
        if inp.borrow().scroll_delta_x != 0 {
            let mut _tmp_3965 = inp.borrow().scroll_delta_x.clone();
            self.editor.clone().borrow().scrollByX(_tmp_3965);
        }
        if inp.borrow().ctrl_down {
            let t: String = inp.borrow().text_input.clone();
            let mut ti: i64 = 0;
            let tn: i64 = CeNative::str_len(&t);
            while ti < tn {
                let ch: i64 = CeNative::char_at(&t, ti);
                // The page's own two chords; everything else is the editor's.
                if ch == 32 {
                    // Ctrl+Space: the list, whatever has been typed so far.
                    let opened: bool = self.editor.borrow_mut().requestCompletions();
                    if !opened {
                        self.status = "nothing to suggest here".to_string();
                    }
                } else {
                    if ch == 101 || ch == 69 {
                        self.jumpToNextProblem();
                    } else {
                        if ch == 107 || ch == 75 {
                            self.nextSample();
                        } else {
                            if ch == 108 || ch == 76 {
                                self.loadSample(self.sample.clone());
                            } else {
                                self.editor.clone().borrow_mut().handleCtrlChar(ch);
                            }
                        }
                    }
                }
                ti += 1;
            }
        } else {
            if UIInput::has_text(&inp) {
                let mut _tmp_3972 = inp.borrow().text_input.clone();
                self.editor.clone().borrow_mut().handleText(&_tmp_3972);
            }
        }
        let mut ki: i64 = 0;
        let kn: i64 = UIInput::key_count(&inp);
        while ki < kn {
            let key: i64 = UIInput::key_at(&inp, ki);
            let mut _tmp_3975 = inp.borrow().shift_down.clone();
            let mut _tmp_3976 = inp.borrow().ctrl_down.clone();
            self.editor.clone().borrow_mut().handleKey(key, _tmp_3975, _tmp_3976);
            ki += 1;
        }
    }
    pub fn jumpToNextProblem(&mut self) {
        let moved: bool = self.editor.borrow_mut().goToNextProblem();
        if !moved {
            self.status = "no problems".to_string();
            return;
        }
        let mut d: Rc<RefCell<EditorDiagnostic>> = self.editor.borrow_mut().problemOnLine(self.editor.borrow().sel.borrow().caret.borrow().line.clone());
        if d.borrow().line >= 0 {
            self.status = d.borrow().label();
        }
    }
    /// --- when to paint at all ---------------------------------------------------
    /// The caret blinks off the CLOCK rather than off a frame counter, so a page
    /// that is not painting still knows when the next blink is due.
    pub fn blinkPhase(&self) -> i64 {
        let ms: f64 = wall_clock_ms();
        let phase: f64 = ms / 500.0;
        phase.floor() as i64
    }
    /// A cheap fingerprint of everything the picture depends on. Building the
    /// display list and serializing it is milliseconds; this is a handful of
    /// string joins — so the host asks this sixty times a second and builds a
    /// scene only when the answer changed. Before it existed the page rebuilt
    /// and re-serialized the whole document on every animation frame whether or
    /// not anything had happened, which is most of what "fps 4" was.
    pub fn revision(&self) -> String {
        let mut r: String = self.editor.borrow().buf.borrow().version.to_string();
        r.push_str(&format!("|{}", self.editor.borrow().sel.borrow().caret.borrow().line));
        r.push_str(&format!(":{}", self.editor.borrow().sel.borrow().caret.borrow().col));
        r.push_str(&format!("|{}", self.editor.borrow().sel.borrow().anchor.borrow().line));
        r.push_str(&format!(":{}", self.editor.borrow().sel.borrow().anchor.borrow().col));
        r.push_str(&format!("|{}", self.editor.borrow().layout.borrow().scroll_line));
        r.push_str(&format!(":{}", self.editor.borrow().layout.borrow().scroll_x));
        r.push_str(&format!("|{}", self.W));
        r.push_str(&format!("x{}", self.H));
        r.push_str(&format!("|{}", self.blinkPhase()));
        r.push_str(&format!("|{}", self.docName));
        r.push_str(&format!("|{}", self.status));
        if self.editor.borrow().focused {
            r.push_str(&"|f".to_string());
        }
        // Diagnostics can change without an edit — the full pass runs after a
        // pause in the typing — so what is currently shown is part of it. Read
        // directly rather than through problemCount(), which would run the
        // pass this is supposed to be cheaper than.
        r.push_str(&format!("|{}", self.editor.borrow().diagnostics.len() as i64));
        r.push_str(&format!(":{}", self.editor.borrow().diagVersion));
        // The completion list is part of the picture, so opening it has to
        // count as a change — otherwise the popup appears on the next blink.
        if self.editor.borrow().completionOpen {
            r.push_str(&format!("|c{}", self.editor.borrow().completions.len() as i64));
            r.push_str(&format!(":{}", self.editor.borrow().completionIndex));
        }
        r
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
    pub(crate) fn pushText(&self, dl: &Rc<RefCell<EVGDisplayList>>, s: String, x: f64, y: f64, size: f64, col: &Rc<RefCell<EVGColor>>) {
        if s.is_empty() {
            return;
        }
        let mut c: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        c.borrow_mut().kind = 3;
        c.borrow_mut().x = x;
        c.borrow_mut().y = y;
        c.borrow_mut().w = UITextRenderer::measure_width(&self.ctx.borrow().tr, &s, size);
        c.borrow_mut().h = UITextRenderer::line_height(&self.ctx.borrow().tr, size);
        c.borrow_mut().text = s.clone();
        c.borrow_mut().font_family = self.fontFamily.clone();
        c.borrow_mut().font_size = size;
        c.borrow_mut().r = EVGColor::red(&col);
        c.borrow_mut().g = EVGColor::green(&col);
        c.borrow_mut().b = EVGColor::blue(&col);
        c.borrow_mut().a = EVGColor::alpha(&col);
        dl.borrow_mut().cmds.push(c.clone());
    }
    /// Re-lex now rather than inside the paint, so the time it took is a number
    /// this page can show rather than a shrug.
    pub fn syncTokens(&mut self) {
        let before: i64 = self.editor.borrow().tokenVersion;
        if before == self.editor.borrow().buf.borrow().version {
            return;
        }
        let t0: f64 = wall_clock_ms();
        self.editor.clone().borrow_mut().ensureTokens();
        let t1: f64 = wall_clock_ms();
        let took: f64 = (t1 - t0) * 1000.0;
        self.lexUs = took.floor() as i64;
    }
    pub fn tokenCount(&self) -> i64 {
        let mut n: i64 = 0;
        let mut i: i64 = 0;
        let cnt: i64 = self.editor.borrow().lines.len() as i64;
        while i < cnt {
            let mut line: Rc<RefCell<CodeTokenLine>> = self.editor.borrow()
                .lines[i as usize].clone();
            n += line.borrow().count();
            i += 1;
        }
        n
    }
    pub(crate) fn buildDisplayList(&mut self) -> Rc<RefCell<EVGDisplayList>> {
        self.syncTokens();
        let mut dl: Rc<RefCell<EVGDisplayList>> = Rc::new(RefCell::new(EVGDisplayList::new()));
        let mut _tmp_3994 = self.bg.clone();
        let arg_0 = self.W as f64;
        self.pushRect(&dl, 0.0, 0.0, arg_0, self.H as f64, &_tmp_3994);
        // header
        let mut _tmp_3995 = self.barBg.clone();
        let arg_0 = self.W as f64;
        self.pushRect(&dl, 0.0, 0.0, arg_0, self.headerH as f64, &_tmp_3995);
        let arg_0 = (self.headerH - 1) as f64;
        let arg_1 = self.W as f64;
        self.pushRect(&dl, 0.0, arg_0, arg_1, 1.0, &EVGColor::rgb(48, 52, 70).clone());
        let mut _tmp_3996 = self.docName.clone();
        self.pushText(&dl, _tmp_3996, 12.0, 7.0, 13.0, &self.barText.clone());
        let mut _tmp_3998 = self.docName.clone();
        let name_w: i64 = (UITextRenderer::measure_width(
            &self.ctx.borrow().tr,
            &_tmp_3998,
            13.0
        )).floor() as i64;
        let hint: String = "Ctrl+K next sample · Ctrl+L reload · Ctrl+Z undo · Ctrl+A select all".to_string();
        let mut _tmp_3999 = self.barMuted.clone();
        self.pushText(&dl,
            hint.to_string(),
            (name_w + 28) as f64,
            8.0,
            12.0,
            &_tmp_3999
        );
        let mut _tmp_4001 = self.editorTop();
        let mut _tmp_4002 = self.W.clone();
        let mut _tmp_4003 = self.editorHeight();
        self.editor.clone().borrow().setBounds(0, _tmp_4001, _tmp_4002, _tmp_4003);
        let mut _tmp_4006 = self.ctx.borrow().tr.clone();
        let mut _tmp_4007 = self.blinkPhase();
        self.editor.clone().borrow_mut().paint(&dl, _tmp_4006.clone(), _tmp_4007);
        // status
        let sy: i64 = self.H - self.statusH;
        let mut _tmp_4009 = self.barBg.clone();
        let arg_0 = self.W as f64;
        self.pushRect(&dl, 0.0, sy as f64, arg_0, self.statusH as f64, &_tmp_4009);
        let arg_0 = self.W as f64;
        self.pushRect(&dl, 0.0, sy as f64, arg_0, 1.0, &EVGColor::rgb(48, 52, 70).clone());
        let caret: String = format!("L{}:{}", self.editor.borrow().sel.borrow().caret.borrow().line + 1, self.editor.borrow().sel.borrow().caret.borrow().col + 1);
        let mut _tmp_4010 = self.accent.clone();
        self.pushText(&dl,
            caret.clone(),
            12.0,
            (sy + 5) as f64,
            12.0,
            &_tmp_4010
        );
        let line_count: i64 = EditorBuffer::line_count(&self.editor.borrow().buf);
        let toks: i64 = self.tokenCount();
        let problems: i64 = self.editor.borrow_mut().problemCount();
        let mut info: String = format!("{} lines · {} tokens · lex {} us · {} draw cmds", line_count.to_string(), toks, self.lexUs, self.lastCmds);
        if problems > 0 {
            info.push_str(&format!(" · {} problem(s)", problems));
        }
        info.push_str(&format!(" · check {} us", self.editor.borrow().diagnoseUs));
        if self.editor.borrow().quickOnly {
            // Said out loud: what is on screen is the fast pass, and the full
            // one is waiting for the typing to stop.
            info.push_str(" (fast pass)");
        }
        let mut _tmp_4012 = self.barMuted.clone();
        self.pushText(&dl,
            info.clone(),
            84.0,
            (sy + 5) as f64,
            12.0,
            &_tmp_4012
        );
        // The complaint about the line the caret is on, in the colour of its
        // severity — a message that stays out of the way until it is wanted.
        let mut here: Rc<RefCell<EditorDiagnostic>> = self.editor.borrow_mut().problemOnLine(self.editor.borrow().sel.borrow().caret.borrow().line.clone());
        let mut right: String = self.status.clone();
        if here.borrow().line >= 0 {
            right = here.borrow().message.clone();
        }
        if !right.is_empty() {
            let mut right_col: Rc<RefCell<EVGColor>> = self.barMuted.clone().clone();
            if here.borrow().line >= 0 {
                let _tmp_4016 = (self.editor.borrow().theme).borrow().forSeverity(here.borrow().severity.clone());
                right_col = _tmp_4016;
            }
            let rw: i64 = (UITextRenderer::measure_width(
                &self.ctx.borrow().tr,
                &right,
                12.0
            )).floor() as i64;
            let arg_0 = ((self.W - rw) - 12) as f64;
            self.pushText(&dl,
                right.clone(),
                arg_0,
                (sy + 5) as f64,
                12.0,
                &right_col
            );
        }
        self.lastCmds = EVGDisplayList::count(&dl);
        self.frame += 1;
        dl
    }
    pub fn sceneJson(&mut self) -> String {
        let mut dl: Rc<RefCell<EVGDisplayList>> = self.buildDisplayList();
        let body: String = EVGDisplayList::to_json(&dl);
        format!("{}{},\"height\":{},\"list\":{}{}", "{\"width\":".to_string(), EVGDisplayList::num(self.W as f64), EVGDisplayList::num(self.H as f64), body, "}".to_string())
    }
}
