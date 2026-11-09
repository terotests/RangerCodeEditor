// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::code_tokens::CodeToken;
use crate::code_tokens::CodeTokenKind;
use crate::code_tokens::CodeTokenLine;
use crate::rgr::CeNative;

/// ==============================================================================
/// JsTokens.rgr — enough JavaScript to colour it
/// ==============================================================================
///
/// A code editor needs to know what a run of characters IS before it can decide
/// what colour to draw it. This is that, and no more: a lexer that walks a line
/// at a time, carries the two states that outlive a line (a block comment and a
/// template literal), and hands back spans.
///
/// It is deliberately not a parser. Nothing here builds a tree, resolves a name
/// or reports an error — ComponentEngine does all three when the script runs,
/// and a second, disagreeing opinion about what the source means is worse than
/// no opinion at all. What this file gets wrong shows up as a colour, never as
/// behaviour.
///
/// JSX is handled with two local rules rather than a mode: `<Name` and `</Name`
/// are tag names, and an identifier followed by `=` is an attribute. That is a
/// lie in the general case (`a < b`, `x = 1`) and true in every line of a report
/// script, which is what it is for. `<` before a space or a digit stays
/// punctuation, so `i < rows.length` keeps its colour.
/// ==============================================================================
#[derive(Clone)]
pub struct JsTokenizer {
    /// Carried between lines: 0 code, 1 inside a block comment, 2 inside `…`
    pub state: i64,
}

impl JsTokenizer {
    pub fn new() -> Self {
        Self {
            state: 0,
        }
    }
    pub fn isDigit(ch: i64) -> bool {
        if ch < 48 {
            return false;
        }
        ch <= 57
    }
    pub fn isIdentStart(ch: i64) -> bool {
        if (ch >= 65 && ch <= 90) {
            return true;
        }
        if (ch >= 97 && ch <= 122) {
            return true;
        }
        if ch == 95 {
            return true;
        }
        if ch == 36 {
            return true;
        }
        // Anything above ASCII is a letter as far as this is concerned: a
        // variable named `hintaТ` should not split into three tokens.
        ch > 127
    }
    pub fn isIdentChar(ch: i64) -> bool {
        if JsTokenizer::isIdentStart(ch) {
            return true;
        }
        JsTokenizer::isDigit(ch)
    }
    pub fn isUpper(ch: i64) -> bool {
        if ch < 65 {
            return false;
        }
        ch <= 90
    }
    pub fn wordIn(word: &str, list: &[String]) -> bool {
        let mut i: i64 = 0;
        let n: i64 = list.len() as i64;
        while i < n {
            let w: String = list[i as usize].clone();
            if w == word {
                return true;
            }
            i += 1;
        }
        false
    }
    pub fn keywords() -> Vec<String> {
        vec!["const".to_string(), "let".to_string(), "var".to_string(), "function".to_string(), "return".to_string(), "if".to_string(), "else".to_string(), "for".to_string(), "while".to_string(), "do".to_string(), "break".to_string(), "continue".to_string(), "new".to_string(), "typeof".to_string(), "instanceof".to_string(), "in".to_string(), "of".to_string(), "class".to_string(), "extends".to_string(), "import".to_string(), "export".to_string(), "from".to_string(), "as".to_string(), "default".to_string(), "switch".to_string(), "case".to_string(), "this".to_string(), "delete".to_string(), "void".to_string(), "try".to_string(), "catch".to_string(), "finally".to_string(), "throw".to_string(), "async".to_string(), "await".to_string(), "yield".to_string(), "true".to_string(), "false".to_string(), "null".to_string(), "undefined".to_string()].clone()
    }
    /// The workbook API plus the handful of JavaScript globals a report reaches
    /// for. Colouring these differently is the cheapest documentation there is:
    /// a name that lights up is a name the engine knows.
    pub fn builtins() -> Vec<String> {
        vec!["sheetNames".to_string(), "sheetCount".to_string(), "sheetRows".to_string(), "sheetText".to_string(), "usedRows".to_string(), "usedCols".to_string(), "cell".to_string(), "text".to_string(), "range".to_string(), "rangeText".to_string(), "param".to_string(), "query".to_string(), "hasDatabase".to_string(), "formatNumber".to_string(), "log".to_string(), "render".to_string(), "Math".to_string(), "JSON".to_string(), "Number".to_string(), "String".to_string(), "Array".to_string(), "Object".to_string(), "Boolean".to_string(), "Date".to_string(), "parseInt".to_string(), "parseFloat".to_string(), "isNaN".to_string(), "console".to_string()].clone()
    }
    pub fn reset(&mut self) {
        self.state = 0;
    }
    /// Every line of the document, in order. State is carried, so this must see
    /// the lines from the top — which is what the editor does after each edit.
    pub fn tokenize(&mut self, lines: &[String]) -> Vec<Rc<RefCell<CodeTokenLine>>> {
        self.state = 0;
        let mut out: Vec<Rc<RefCell<CodeTokenLine>>> = Vec::new();
        let mut i: i64 = 0;
        let n: i64 = lines.len() as i64;
        while i < n {
            out.push(self.tokenizeLine(&lines[i as usize].clone()));
            i += 1;
        }
        out
    }
    pub fn tokenizeLine(&mut self, line: &str) -> Rc<RefCell<CodeTokenLine>> {
        let mut out: Rc<RefCell<CodeTokenLine>> = Rc::new(RefCell::new(CodeTokenLine::new()));
        let n: i64 = CeNative::str_len(&line);
        let mut i: i64 = 0;
        let mut kw: Vec<String> = JsTokenizer::keywords();
        let mut bi: Vec<String> = JsTokenizer::builtins();
        while i < n {
            if self.state == 1 {
                // Inside a block comment: everything to `*/` or to the end.
                let end_at: i64 = self.findBlockEnd(line, i);
                let mut stop: i64 = end_at;
                if end_at < 0 {
                    stop = n;
                } else {
                    self.state = 0;
                    stop = end_at + 2;
                }
                out.borrow_mut().tokens.push(CodeToken::create(i, stop - i, CodeTokenKind::comment()));
                i = stop;
                continue;
            }
            if self.state == 2 {
                let t_end: i64 = self.findTemplateEnd(line, i);
                let mut t_stop: i64 = t_end;
                if t_end < 0 {
                    t_stop = n;
                } else {
                    self.state = 0;
                    t_stop = t_end + 1;
                }
                out.borrow_mut().tokens.push(CodeToken::create(i, t_stop - i, CodeTokenKind::text()));
                i = t_stop;
                continue;
            }
            let ch: i64 = CeNative::char_at(&line, i);
            if ch == 32 {
                i += 1;
                continue;
            }
            if ch == 9 {
                i += 1;
                continue;
            }
            // // line comment
            if ch == 47 {
                let next: i64 = self.charOr(line, i + 1);
                if next == 47 {
                    out.borrow_mut().tokens.push(CodeToken::create(i, n - i, CodeTokenKind::comment()));
                    i = n;
                    continue;
                }
                if next == 42 {
                    self.state = 1;
                    let b_end: i64 = self.findBlockEnd(line, i + 2);
                    let mut b_stop: i64 = b_end;
                    if b_end < 0 {
                        b_stop = n;
                    } else {
                        self.state = 0;
                        b_stop = b_end + 2;
                    }
                    out.borrow_mut().tokens.push(CodeToken::create(i, b_stop - i, CodeTokenKind::comment()));
                    i = b_stop;
                    continue;
                }
            }
            // strings
            if ch == 34 || ch == 39 {
                let s_end: i64 = self.findQuoteEnd(line, i + 1, ch);
                let mut s_stop: i64 = s_end;
                if s_end < 0 {
                    s_stop = n;
                } else {
                    s_stop = s_end + 1;
                }
                out.borrow_mut().tokens.push(CodeToken::create(i, s_stop - i, CodeTokenKind::text()));
                i = s_stop;
                continue;
            }
            if ch == 96 {
                self.state = 2;
                let q_end: i64 = self.findTemplateEnd(line, i + 1);
                let mut q_stop: i64 = q_end;
                if q_end < 0 {
                    q_stop = n;
                } else {
                    self.state = 0;
                    q_stop = q_end + 1;
                }
                out.borrow_mut().tokens.push(CodeToken::create(i, q_stop - i, CodeTokenKind::text()));
                i = q_stop;
                continue;
            }
            if JsTokenizer::isDigit(ch) {
                let num_end: i64 = self.readNumber(line, i);
                out.borrow_mut().tokens.push(CodeToken::create(i, num_end - i, CodeTokenKind::number()));
                i = num_end;
                continue;
            }
            // <Tag  /  </Tag
            if ch == 60 {
                let after: i64 = self.charOr(line, i + 1);
                let mut name_at: i64 = i + 1;
                if after == 47 {
                    name_at = i + 2;
                }
                let name_ch: i64 = self.charOr(line, name_at);
                if JsTokenizer::isIdentStart(name_ch) {
                    let tag_end: i64 = self.readIdent(line, name_at);
                    out.borrow_mut().tokens.push(CodeToken::create(i, tag_end - i, CodeTokenKind::tag()));
                    i = tag_end;
                    continue;
                }
            }
            // `/>` and `>` closing a tag read as tag rather than as arithmetic.
            if ch == 62 {
                out.borrow_mut().tokens.push(CodeToken::create(i, 1, CodeTokenKind::tag()));
                i += 1;
                continue;
            }
            if JsTokenizer::isIdentStart(ch) {
                let id_end: i64 = self.readIdent(line, i);
                let word: String = CeNative::substr(&line, i, id_end);
                let mut kind: i64 = CodeTokenKind::ident();
                if JsTokenizer::wordIn(&word, &kw) {
                    kind = CodeTokenKind::keyword();
                } else {
                    if JsTokenizer::wordIn(&word, &bi) {
                        kind = CodeTokenKind::builtin();
                    } else {
                        if self.isAttributeAt(line, id_end) {
                            kind = CodeTokenKind::attr();
                        }
                    }
                }
                out.borrow_mut().tokens.push(CodeToken::create(i, id_end - i, kind));
                i = id_end;
                continue;
            }
            out.borrow_mut().tokens.push(CodeToken::create(i, 1, CodeTokenKind::punct()));
            i += 1;
        }
        out
    }
    pub fn charOr(&self, line: &str, at: i64) -> i64 {
        if at < 0 {
            return 0;
        }
        if at >= (CeNative::str_len(&line)) {
            return 0;
        }
        CeNative::char_at(&line, at)
    }
    /// `name=` is an attribute; `name ==` is a comparison. One character of
    /// lookahead is the whole difference.
    pub fn isAttributeAt(&self, line: &str, at: i64) -> bool {
        let ch: i64 = self.charOr(line, at);
        if ch != 61 {
            return false;
        }
        let after: i64 = self.charOr(line, at + 1);
        after != 61
    }
    pub fn readIdent(&self, line: &str, from: i64) -> i64 {
        let mut i: i64 = from;
        let n: i64 = CeNative::str_len(&line);
        while i < n {
            let ch: i64 = CeNative::char_at(&line, i);
            if !JsTokenizer::isIdentChar(ch) {
                return i;
            }
            i += 1;
        }
        n
    }
    pub fn readNumber(&self, line: &str, from: i64) -> i64 {
        let mut i: i64 = from;
        let n: i64 = CeNative::str_len(&line);
        while i < n {
            let ch: i64 = CeNative::char_at(&line, i);
            let mut ok: bool = JsTokenizer::isDigit(ch);
            if ch == 46 {
                ok = true;
            }
            if (ch >= 97 && ch <= 122) {
                ok = true;
            }
            if (ch >= 65 && ch <= 90) {
                ok = true;
            }
            if !ok {
                return i;
            }
            i += 1;
        }
        n
    }
    pub fn findBlockEnd(&self, line: &str, from: i64) -> i64 {
        let mut i: i64 = from;
        let n: i64 = CeNative::str_len(&line);
        while i < n - 1 {
            let a: i64 = CeNative::char_at(&line, i);
            let b: i64 = CeNative::char_at(&line, i + 1);
            if a == 42 && b == 47 {
                return i;
            }
            i += 1;
        }
        -1
    }
    pub fn findQuoteEnd(&self, line: &str, from: i64, quote: i64) -> i64 {
        let mut i: i64 = from;
        let n: i64 = CeNative::str_len(&line);
        while i < n {
            let ch: i64 = CeNative::char_at(&line, i);
            if ch == 92 {
                i += 2;
                continue;
            }
            if ch == quote {
                return i;
            }
            i += 1;
        }
        -1
    }
    pub fn findTemplateEnd(&self, line: &str, from: i64) -> i64 {
        self.findQuoteEnd(line, from, 96)
    }
}
