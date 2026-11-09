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
use crate::editor_language::LanguageText;
use crate::js_tokens::JsTokenizer;
use crate::rgr::CeNative;
use crate::rgr::{TSLexer, TSNode, TSParserSimple, Token};

/// ==============================================================================
/// JsLanguage.rgr — JavaScript / TSX, checked by the parser that will run it
/// ==============================================================================
///
/// Two passes, and the order matters.
///
/// STRUCTURE, from the lexer. A bracket with no partner, a string with no end, a
/// block comment that is never closed. These are cheap, they have exact positions, and
/// they are the errors a document has WHILE IT IS BEING TYPED — half of them
/// exist for one keystroke.
///
/// THE PARSE, from `TSParserSimple` — the same parser `ComponentEngine` runs the
/// script with. That is the point: what the editor underlines is what the engine
/// will refuse, rather than a second opinion from a second grammar that agrees
/// with the first one most of the time. The parser reports its first diagnostic
/// with a position (`firstErrorLine` / `firstErrorCol`), recovers, and carries
/// on; we take that one.
///
/// The parse runs under a LINE BUDGET. It is a full parse on every change, and
/// on a large document that is milliseconds per keystroke — so above the budget
/// the structural pass stands alone, and the editor says so rather than quietly
/// going quiet.
/// ==============================================================================
#[derive(Clone)]
pub struct JsLanguage {
    /// Above this many lines, only the structural pass runs.
    pub maxParseLines: i64,
    /// Microseconds the last full parse took, for whoever wants to show it.
    pub parseUs: i64,
    pub parsedLastRun: bool,
    pub lexer: Rc<RefCell<JsTokenizer>>,
}

impl JsLanguage {
    pub fn new() -> Self {
        Self {
            maxParseLines: 4000,
            parseUs: 0,
            parsedLastRun: false,
            lexer: Rc::new(RefCell::new(JsTokenizer::new())),
        }
    }
    /// --- members, and a guess at what they belong to ----------------------------
    ///
    /// `body.` should offer what an array can do, and it does — from a guess
    /// made by reading BACKWARDS for where `body` was declared. There is no type
    /// checker here and there is not going to be one on a keystroke; what there
    /// is, is the handful of ways a value gets its type in a report script:
    ///
    ///     const body = [];              an array literal
    ///     const name = 'Sales';         a string literal
    ///     const rows = sheetRows(…);    a workbook call whose result is known
    ///     const cell = rows[i];         an element of one of those
    ///
    /// Those four cover nearly every line anyone writes here. When none of them
    /// matches, the guess is `any` and the list is everything — the same answer
    /// a real type checker gives when it does not know either, and better than
    /// an empty popup.
    ///
    /// The member lists are the methods THIS ENGINE IMPLEMENTS, taken from
    /// ComponentEngine rather than from the JavaScript standard. Offering
    /// `.flatMap()` because JavaScript has it would be offering to write a line
    /// that cannot run.
    pub fn arrayMembers() -> Vec<String> {
        vec!["length".to_string(), "push".to_string(), "pop".to_string(), "shift".to_string(), "unshift".to_string(), "slice".to_string(), "splice".to_string(), "concat".to_string(), "join".to_string(), "indexOf".to_string(), "lastIndexOf".to_string(), "includes".to_string(), "map".to_string(), "filter".to_string(), "forEach".to_string(), "reduce".to_string(), "find".to_string(), "findIndex".to_string(), "some".to_string(), "every".to_string(), "sort".to_string(), "reverse".to_string(), "flat".to_string(), "fill".to_string(), "at".to_string(), "keys".to_string(), "values".to_string(), "entries".to_string()].clone()
    }
    pub fn stringMembers() -> Vec<String> {
        vec!["length".to_string(), "charAt".to_string(), "charCodeAt".to_string(), "at".to_string(), "concat".to_string(), "startsWith".to_string(), "endsWith".to_string(), "includes".to_string(), "indexOf".to_string(), "lastIndexOf".to_string(), "padStart".to_string(), "padEnd".to_string(), "repeat".to_string(), "replace".to_string(), "replaceAll".to_string(), "slice".to_string(), "split".to_string(), "substring".to_string(), "toLowerCase".to_string(), "toUpperCase".to_string(), "trim".to_string(), "trimStart".to_string(), "trimEnd".to_string(), "toString".to_string()].clone()
    }
    pub fn numberMembers() -> Vec<String> {
        vec!["toFixed".to_string(), "toString".to_string()].clone()
    }
    /// Calls whose result type is known because the workbook API defines it.
    pub fn rowsCalls() -> Vec<String> {
        vec!["sheetRows".to_string(), "sheetText".to_string(), "range".to_string(), "rangeText".to_string(), "query".to_string()].clone()
    }
    pub fn stringCalls() -> Vec<String> {
        vec!["formatNumber".to_string(), "param".to_string(), "text".to_string(), "currentLine".to_string()].clone()
    }
    pub fn numberCalls() -> Vec<String> {
        vec!["usedRows".to_string(), "usedCols".to_string(), "sheetCount".to_string()].clone()
    }
    /// Is the caret completing a MEMBER — that is, is the character before the
    /// word being typed a dot? Returns its column, or -1.
    pub fn dotBefore(text: &str, col: i64) -> i64 {
        let start: i64 = LanguageText::prefixStart(text, col);
        if start == 0 {
            return -1;
        }
        let ch: i64 = CeNative::char_at(&text, start - 1);
        if ch != 46 {
            return -1;
        }
        start - 1
    }
    /// What is being dereferenced, as written: `body`, `rows[i]`, `'x'`.
    /// `indexed` says whether it ended in a subscript, which is what turns "an
    /// array of rows" into "a row".
    pub fn receiverBefore(text: &str, dot_col: i64, mut indexed_out: &mut Vec<bool>) -> String {
        indexed_out.clear();
        indexed_out.push(false);
        let mut i: i64 = dot_col - 1;
        if i < 0 {
            return String::new();
        }
        let mut ch: i64 = CeNative::char_at(&text, i);
        // A subscript: skip back over it to the name it belongs to.
        if ch == 93 {
            indexed_out[0] = true;
            let mut depth: i64 = 0;
            while i >= 0 {
                let c2: i64 = CeNative::char_at(&text, i);
                if c2 == 93 {
                    depth += 1;
                }
                if c2 == 91 {
                    depth -= 1;
                    if depth == 0 {
                        i -= 1;
                        break;
                    }
                }
                i -= 1;
            }
            if i < 0 {
                return String::new();
            }
            ch = CeNative::char_at(&text, i);
        }
        // A literal receiver types itself.
        if ch == 39 {
            return "'".to_string();
        }
        if ch == 34 {
            return "'".to_string();
        }
        if ch == 96 {
            return "'".to_string();
        }
        if (ch >= 48 && ch <= 57) {
            return "0".to_string();
        }
        let end: i64 = i + 1;
        let start: i64 = LanguageText::prefixStart(text, end);
        if start >= end {
            return String::new();
        }
        CeNative::substr(&text, start, end)
    }
    /// The column just after the `=` of `const name = …` on this line, or -1.
    /// A declaration and not an assignment, because an assignment can be to
    /// anything and a declaration is where a value gets its shape.
    pub fn declarationOf(text: &str, name: &str) -> i64 {
        let n: i64 = CeNative::str_len(&text);
        let name_len: i64 = CeNative::str_len(&name);
        let mut i: i64 = 0;
        while i < n {
            let rest: i64 = n - i;
            if rest < name_len {
                return -1;
            }
            let here: String = CeNative::substr(&text, i, i + name_len);
            if here == name {
                let mut ok_before: bool = false;
                if i >= 6 {
                    let head: String = CeNative::substr(&text, i - 6, i);
                    if head == "const " {
                        ok_before = true;
                    }
                }
                if i >= 4 {
                    let head2: String = CeNative::substr(&text, i - 4, i);
                    if head2 == "let " {
                        ok_before = true;
                    }
                    if head2 == "var " {
                        ok_before = true;
                    }
                }
                if ok_before {
                    // …and an `=` after it, with nothing but spaces between.
                    let mut j: i64 = i + name_len;
                    while j < n {
                        let ch: i64 = CeNative::char_at(&text, j);
                        if ch == 32 {
                            j += 1;
                            continue;
                        }
                        if ch == 61 {
                            let after: i64 = j + 1;
                            if after < n {
                                if CeNative::char_at(&text, after) == 61 {
                                    return -1;
                                }
                            }
                            return after;
                        }
                        break;
                    }
                }
            }
            i += 1;
        }
        -1
    }
    pub fn leadingName(s: &str) -> String {
        let mut i: i64 = 0;
        let n: i64 = CeNative::str_len(&s);
        while i < n {
            let ch: i64 = CeNative::char_at(&s, i);
            let mut word: bool = false;
            if (ch >= 65 && ch <= 90) {
                word = true;
            }
            if (ch >= 97 && ch <= 122) {
                word = true;
            }
            if (ch >= 48 && ch <= 57) {
                word = true;
            }
            if ch == 95 {
                word = true;
            }
            if !word {
                if ch == 40 {
                    return CeNative::substr(&s, 0, i);
                }
                return String::new();
            }
            i += 1;
        }
        String::new()
    }
    /// The last `.method(` in an expression.
    pub fn trailingCall(s: &str) -> String {
        let n: i64 = CeNative::str_len(&s);
        let mut i: i64 = n - 1;
        let mut last_dot: i64 = -1;
        while i >= 0 {
            let ch: i64 = CeNative::char_at(&s, i);
            if ch == 46 {
                last_dot = i;
                break;
            }
            i -= 1;
        }
        if last_dot < 0 {
            return String::new();
        }
        let mut j: i64 = last_dot + 1;
        let start: i64 = j;
        while j < n {
            let c2: i64 = CeNative::char_at(&s, j);
            if c2 == 40 {
                return CeNative::substr(&s, start, j);
            }
            let mut word: bool = false;
            if (c2 >= 65 && c2 <= 90) {
                word = true;
            }
            if (c2 >= 97 && c2 <= 122) {
                word = true;
            }
            if !word {
                return String::new();
            }
            j += 1;
        }
        String::new()
    }
    pub fn isOpen(ch: i64) -> bool {
        if ch == 40 {
            return true;
        }
        if ch == 91 {
            return true;
        }
        ch == 123
    }
    pub fn isClose(ch: i64) -> bool {
        if ch == 41 {
            return true;
        }
        if ch == 93 {
            return true;
        }
        ch == 125
    }
    pub fn closerFor(open: i64) -> i64 {
        if open == 40 {
            return 41;
        }
        if open == 91 {
            return 93;
        }
        125
    }
    /// The guess. Reads backwards for a declaration of `name` and classifies
    /// whatever is on the right of the `=`.
    pub fn inferKind(&self, lines: &[String], upto: i64, name: &str, indexed: bool) -> String {
        if name == "'" {
            return "string".to_string();
        }
        if name == "0" {
            return "number".to_string();
        }
        if name.is_empty() {
            return "any".to_string();
        }
        let mut li: i64 = upto;
        while li >= 0 {
            let text: String = lines[li as usize].clone();
            let at: i64 = JsLanguage::declarationOf(&text, name);
            if at >= 0 {
                let rhs: String = CeNative::substr(&text, at, (CeNative::str_len(&text)));
                return self.kindOfExpression(&rhs, indexed);
            }
            li -= 1;
        }
        "any".to_string()
    }
    /// The keys of a one-line object literal, so `const o = { a: 1, b: 2 }`
    /// can answer `o.` with what it actually has in it.
    pub fn objectKeysOf(&self, lines: &[String], upto: i64, name: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut li: i64 = upto;
        while li >= 0 {
            let text: String = lines[li as usize].clone();
            let at: i64 = JsLanguage::declarationOf(&text, name);
            if at >= 0 {
                let rhs: String = CeNative::substr(&text, at, (CeNative::str_len(&text))).trim().to_string();
                if !rhs.is_empty() {
                    if CeNative::char_at(&rhs, 0) == 123 {
                        let mut i: i64 = 1;
                        let n: i64 = CeNative::str_len(&rhs);
                        let mut word: String = String::new();
                        while i < n {
                            let ch: i64 = CeNative::char_at(&rhs, i);
                            let mut is_word: bool = false;
                            if (ch >= 65 && ch <= 90) {
                                is_word = true;
                            }
                            if (ch >= 97 && ch <= 122) {
                                is_word = true;
                            }
                            if (ch >= 48 && ch <= 57) {
                                is_word = true;
                            }
                            if ch == 95 {
                                is_word = true;
                            }
                            if is_word {
                                word.push_str(&char::from_u32(ch as u32).unwrap_or('\0').to_string());
                            } else {
                                if ch == 58 {
                                    if !word.is_empty() {
                                        out.push(word.clone());
                                    }
                                }
                                word = String::new();
                            }
                            i += 1;
                        }
                    }
                }
                return out;
            }
            li -= 1;
        }
        out
    }
    /// The members of a guessed kind, as completions.
    pub fn membersFor(&self, kind: &str, prefix: &str, keys: &[String]) -> Vec<Rc<RefCell<Completion>>> {
        let mut out: Vec<Rc<RefCell<Completion>>> = Vec::new();
        let mut own: i64 = 0;
        let kn: i64 = keys.len() as i64;
        while own < kn {
            let key: String = keys[own as usize].clone();
            if LanguageText::matchesPrefix(&key, prefix) {
                out.push(Completion::create(
                    key.clone(),
                    "own property".to_string(),
                    CompletionKind::member(),
                    0
                ));
            }
            own += 1;
        }
        let mut names: Vec<String> = Vec::new();
        let mut detail: String = kind.to_string();
        if kind == "array" {
            names = JsLanguage::arrayMembers();
        }
        if kind == "string" {
            names = JsLanguage::stringMembers();
        }
        if kind == "number" {
            names = JsLanguage::numberMembers();
        }
        if kind == "any" {
            // Nothing is known, so everything is offered — which is what a type
            // checker says about `any` too, and better than an empty popup.
            names = JsLanguage::arrayMembers();
            let mut more: Vec<String> = JsLanguage::stringMembers();
            let mut mi: i64 = 0;
            while mi < (more.len() as i64) {
                let m: String = more[mi as usize].clone();
                if !LanguageText::isInList(&m, &names) {
                    names.push(m.clone());
                }
                mi += 1;
            }
            detail = "any".to_string();
        }
        let mut i: i64 = 0;
        let n: i64 = names.len() as i64;
        while i < n {
            let member: String = names[i as usize].clone();
            if LanguageText::matchesPrefix(&member, prefix) {
                out.push(Completion::create(member.clone(), detail.clone(), CompletionKind::member(), 1));
            }
            i += 1;
        }
        out
    }
    /// What an initialiser evaluates to, as far as anyone can tell by looking.
    pub fn kindOfExpression(&self, rhs: &str, indexed: bool) -> String {
        let s: String = rhs.trim().to_string();
        if s.is_empty() {
            return "any".to_string();
        }
        let ch: i64 = CeNative::char_at(&s, 0);
        if ch == 91 {
            if indexed {
                return "any".to_string();
            }
            return "array".to_string();
        }
        if ch == 39 || ch == 34 {
            if indexed {
                return "string".to_string();
            }
            return "string".to_string();
        }
        if ch == 96 {
            return "string".to_string();
        }
        if ch == 123 {
            return "object".to_string();
        }
        if (ch >= 48 && ch <= 57) {
            return "number".to_string();
        }
        // A call: the workbook API's results are known.
        let call_name: String = JsLanguage::leadingName(&s);
        if !call_name.is_empty() {
            if LanguageText::isInList(&call_name, &JsLanguage::rowsCalls()) {
                // Rows of cells: the whole thing is an array, and one of its
                // elements is a row — also an array.
                return "array".to_string();
            }
            if LanguageText::isInList(&call_name, &JsLanguage::stringCalls()) {
                return "string".to_string();
            }
            if LanguageText::isInList(&call_name, &JsLanguage::numberCalls()) {
                return "number".to_string();
            }
            if call_name == "sheetNames" {
                if indexed {
                    return "string".to_string();
                }
                return "array".to_string();
            }
        }
        // `x.map(…)` is an array; `x.join(…)` is a string. One dot deep, which
        // is as far as a guess like this should reach.
        let method: String = JsLanguage::trailingCall(&s);
        if !method.is_empty() {
            if LanguageText::isInList(&method, &vec!["map".to_string(), "filter".to_string(), "slice".to_string(), "concat".to_string(), "split".to_string(), "sort".to_string(), "reverse".to_string(), "flat".to_string()]) {
                if method == "split" {
                    return "array".to_string();
                }
                return "array".to_string();
            }
            if LanguageText::isInList(&method, &vec!["join".to_string(), "toUpperCase".to_string(), "toLowerCase".to_string(), "trim".to_string(), "substring".to_string(), "replace".to_string(), "padStart".to_string(), "padEnd".to_string(), "toFixed".to_string(), "toString".to_string()]) {
                return "string".to_string();
            }
        }
        "any".to_string()
    }
    /// --- pass one: what the lexer can be sure about ----------------------------
    pub fn structure(&self, lines: &[String], mut out: &mut Vec<Rc<RefCell<EditorDiagnostic>>>) {
        let mut toks: Vec<Rc<RefCell<CodeTokenLine>>> = self.lexer.borrow_mut().tokenize(lines);
        // Open brackets, as three parallel stacks: what, and where.
        let mut stack_ch: Vec<i64> = Vec::new();
        let mut stack_line: Vec<i64> = Vec::new();
        let mut stack_col: Vec<i64> = Vec::new();
        let mut li: i64 = 0;
        let n: i64 = toks.len() as i64;
        while li < n {
            let text: String = lines[li as usize].clone();
            let mut line: Rc<RefCell<CodeTokenLine>> = toks[li as usize].clone();
            let mut ti: i64 = 0;
            let tn: i64 = line.borrow().count();
            while ti < tn {
                let mut t: Rc<RefCell<CodeToken>> = line.borrow().at(ti);
                if t.borrow().kind == CodeTokenKind::punct() {
                    let ch: i64 = CeNative::char_at(&text, t.borrow().start);
                    if JsLanguage::isOpen(ch) {
                        stack_ch.push(ch);
                        stack_line.push(li);
                        stack_col.push(t.borrow().start);
                    }
                    if JsLanguage::isClose(ch) {
                        let depth: i64 = stack_ch.len() as i64;
                        if depth == 0 {
                            let mut _tmp_741 = t.borrow().start.clone();
                            out.push(EditorDiagnostic::span(
                                li,
                                _tmp_741,
                                t.borrow().start + 1,
                                DiagnosticSeverity::error(),
                                format!("stray '{}' — nothing opened here", char::from_u32(ch as u32).unwrap_or('\0')),
                                "javascript".to_string()
                            ));
                        } else {
                            let open_ch: i64 = stack_ch[(depth - 1) as usize];
                            let want: i64 = JsLanguage::closerFor(open_ch);
                            if want != ch {
                                let msg: String = format!("'{}' closes a '{}' — expected '{}'", char::from_u32(ch as u32).unwrap_or('\0'), char::from_u32(open_ch as u32).unwrap_or('\0'), char::from_u32(want as u32).unwrap_or('\0'));
                                let mut _tmp_742 = t.borrow().start.clone();
                                out.push(EditorDiagnostic::span(
                                    li,
                                    _tmp_742,
                                    t.borrow().start + 1,
                                    DiagnosticSeverity::error(),
                                    msg.clone(),
                                    "javascript".to_string()
                                ));
                            }
                            stack_ch.pop();
                            stack_line.pop();
                            stack_col.pop();
                        }
                    }
                }
                ti += 1;
            }
            li += 1;
        }
        // Whatever is still open at the end of the document is missing its
        // partner, and the useful place to say so is where it was OPENED.
        let rest: i64 = stack_ch.len() as i64;
        if rest > 0 {
            let open_ch_1: i64 = stack_ch[(rest - 1) as usize];
            let at_line: i64 = stack_line[(rest - 1) as usize];
            let at_col: i64 = stack_col[(rest - 1) as usize];
            let msg2: String = format!("'{}' is never closed", char::from_u32(open_ch_1 as u32).unwrap_or('\0'));
            out.push(EditorDiagnostic::span(
                at_line,
                at_col,
                at_col + 1,
                DiagnosticSeverity::error(),
                msg2.clone(),
                "javascript".to_string()
            ));
        }
        // The lexer carries two states past the end of a line; either of them
        // still open at the end of the document is an unterminated literal.
        if self.lexer.borrow().state == 1 {
            let last: i64 = (lines.len() as i64) - 1;
            let last_len: i64 = CeNative::str_len(&lines[last as usize]);
            out.push(EditorDiagnostic::span(
                last,
                0,
                last_len,
                DiagnosticSeverity::error(),
                "block comment is never closed — /* with no */".to_string(),
                "javascript".to_string()
            ));
        }
        if self.lexer.borrow().state == 2 {
            let last2: i64 = (lines.len() as i64) - 1;
            let last_len2: i64 = CeNative::str_len(&lines[last2 as usize]);
            out.push(EditorDiagnostic::span(
                last2,
                0,
                last_len2,
                DiagnosticSeverity::error(),
                "template literal is never closed — ` with no `".to_string(),
                "javascript".to_string()
            ));
        }
    }
    /// --- pass two: the parser the engine will use ------------------------------
    pub fn parse(&mut self, source: &str, mut out: &mut Vec<Rc<RefCell<EditorDiagnostic>>>) {
        let t0: f64 = wall_clock_ms();
        let mut lex: Rc<RefCell<TSLexer>> = Rc::new(RefCell::new(TSLexer::new(source.to_string())));
        let mut tokens: Vec<Rc<RefCell<Token>>> = TSLexer::tokenize(&lex);
        let mut parser: Rc<RefCell<TSParserSimple>> = Rc::new(RefCell::new(TSParserSimple::new()));
        // A diagnostic on stdout is no use to a page with no console open, and
        // it is printed once per keystroke. The host reads the fields instead.
        TSParserSimple::set_quiet(&parser, true);
        TSParserSimple::init_parser(&parser, tokens.clone());
        parser.borrow_mut().tsx_mode = true;
        let mut _ast: Rc<RefCell<TSNode>> = TSParserSimple::parse_program(&parser);
        let t1: f64 = wall_clock_ms();
        let took: f64 = (t1 - t0) * 1000.0;
        self.parseUs = took.floor() as i64;
        self.parsedLastRun = true;
        if parser.borrow().error_count == 0 {
            return;
        }
        let mut line: i64 = parser.borrow().first_error_line;
        let mut col: i64 = parser.borrow().first_error_col;
        // The parser counts from one; the editor counts from zero.
        if line > 0 {
            line -= 1;
        } else {
            line = 0;
        }
        if col > 0 {
            col -= 1;
        } else {
            col = 0;
        }
        let mut msg: String = TSParserSimple::first_error(&parser);
        if msg.is_empty() {
            msg = "the script does not parse".to_string();
        }
        out.push(EditorDiagnostic::span(
            line,
            col,
            col + 1,
            DiagnosticSeverity::error(),
            msg.clone(),
            "javascript".to_string()
        ));
    }
}

impl EditorLanguage for JsLanguage {
    fn id(&self) -> String {
        "javascript".to_string()
    }
    fn label(&self) -> String {
        "JavaScript / TSX".to_string()
    }
    fn handlesFile(&self, name: &str) -> bool {
        if LanguageText::hasSuffix(name, ".tsx") {
            return true;
        }
        if LanguageText::hasSuffix(name, ".ts") {
            return true;
        }
        if LanguageText::hasSuffix(name, ".jsx") {
            return true;
        }
        if LanguageText::hasSuffix(name, ".js") {
            return true;
        }
        LanguageText::hasSuffix(name, ".mjs")
    }
    fn diagnose(&mut self, source: &str) -> Vec<Rc<RefCell<EditorDiagnostic>>> {
        let mut out: Vec<Rc<RefCell<EditorDiagnostic>>> = Vec::new();
        let mut lines: Vec<String> = LanguageText::linesOf(source);
        self.structure(&lines, &mut out);
        self.parsedLastRun = false;
        if !out.is_empty() {
            // A document with an unclosed brace parses as nonsense; the second
            // opinion would be a cascade of positions inside code that is fine.
            return out;
        }
        if (lines.len() as i64) > self.maxParseLines {
            return out;
        }
        self.parse(source, &mut out);
        out
    }
    /// What the lexer alone can be sure of. Cheap enough for every keystroke.
    fn quickDiagnose(&self, lines: &[String]) -> Vec<Rc<RefCell<EditorDiagnostic>>> {
        let mut out: Vec<Rc<RefCell<EditorDiagnostic>>> = Vec::new();
        self.structure(lines, &mut out);
        out
    }
    fn tokenize(&self, lines: &[String]) -> Vec<Rc<RefCell<CodeTokenLine>>> {
        return self.lexer.borrow_mut().tokenize(lines);
    }
    /// The workbook API first, then the language's own words, then whatever is
    /// already written in this document. That order is the whole opinion: a
    /// report script is mostly `sheetRows`, `formatNumber` and names it just
    /// made up, and neither of those is `instanceof`.
    fn complete(&self, lines: &[String], line: i64, col: i64) -> Vec<Rc<RefCell<Completion>>> {
        let mut out: Vec<Rc<RefCell<Completion>>> = Vec::new();
        if line < 0 {
            return out;
        }
        if line >= (lines.len() as i64) {
            return out;
        }
        let text: String = lines[line as usize].clone();
        let prefix: String = LanguageText::prefixAt(&text, col);
        // A dot before the word means members of whatever is on its left, and
        // an empty prefix is fine there: `body.` is a question already.
        let dot: i64 = JsLanguage::dotBefore(&text, col);
        if dot >= 0 {
            let mut indexed: Vec<bool> = Vec::new();
            let receiver: String = JsLanguage::receiverBefore(&text, dot, &mut indexed);
            let kind: String = self.inferKind(lines, line, &receiver, indexed[0]);
            let mut keys: Vec<String> = Vec::new();
            if kind == "object" {
                keys = self.objectKeysOf(lines, line, &receiver);
            }
            return self.membersFor(&kind, &prefix, &keys);
        }
        if prefix.is_empty() {
            return out;
        }
        let mut api: Vec<String> = JsTokenizer::builtins();
        let mut i: i64 = 0;
        let n: i64 = api.len() as i64;
        while i < n {
            let name: String = api[i as usize].clone();
            if LanguageText::matchesPrefix(&name, &prefix) {
                out.push(Completion::create(
                    name.clone(),
                    "workbook API".to_string(),
                    CompletionKind::builtin(),
                    0
                ));
            }
            i += 1;
        }
        let mut kw: Vec<String> = JsTokenizer::keywords();
        let mut k: i64 = 0;
        let kn: i64 = kw.len() as i64;
        while k < kn {
            let word: String = kw[k as usize].clone();
            if LanguageText::matchesPrefix(&word, &prefix) {
                out.push(Completion::create(
                    word.clone(),
                    "keyword".to_string(),
                    CompletionKind::keyword(),
                    1
                ));
            }
            k += 1;
        }
        let mut words: Vec<String> = LanguageText::wordsIn(lines, &prefix, 400);
        let mut w: i64 = 0;
        let wn: i64 = words.len() as i64;
        while w < wn {
            let word2: String = words[w as usize].clone();
            if LanguageText::matchesPrefix(&word2, &prefix) {
                if !LanguageText::isInList(&word2, &api) {
                    if !LanguageText::isInList(&word2, &kw) {
                        out.push(Completion::create(
                            word2.clone(),
                            "in this file".to_string(),
                            CompletionKind::word(),
                            2
                        ));
                    }
                }
            }
            w += 1;
        }
        out
    }
}
