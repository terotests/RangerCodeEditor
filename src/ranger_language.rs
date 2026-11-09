// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::code_tokens::CodeTokenLine;
use crate::completion::Completion;
use crate::completion::CompletionKind;
use crate::editor_diagnostic::DiagnosticSeverity;
use crate::editor_diagnostic::EditorDiagnostic;
use crate::editor_language::EditorLanguage;
use crate::editor_language::LanguageText;
use crate::ranger_tokens::RangerTokenizer;
use crate::rgr::CeNative;

/// ==============================================================================
/// RangerLanguage.rgr — the same seam, a different language
/// ==============================================================================
///
/// This plugin exists to prove the seam is real. It shares no code with the
/// JavaScript one, has its own idea of what a comment is (`;` to the end of the
/// line), and reports errors JavaScript does not have — because in an
/// S-expression language a bracket that never closes is not one error among many,
/// it is nearly the whole category.
///
/// It is a LINT, not the Ranger compiler. Wiring the compiler in would be the
/// honest way to get real type errors, and it is also two megabytes of frontend
/// and a second of work per keystroke; when that is wanted it arrives as another
/// plugin next to this one — which is the point of there being a registry.
///
/// What it does check is the shape of the file and the three mistakes the
/// repository's own AGENTS.md says cost the most time.
/// ==============================================================================
#[derive(Clone)]
pub struct RangerLanguage {
    pub lexer: Rc<RefCell<RangerTokenizer>>,
}

impl RangerLanguage {
    pub fn new() -> Self {
        Self {
            lexer: Rc::new(RefCell::new(RangerTokenizer::new())),
        }
    }
    /// `total:int (n:int) {` -> `total`. The name ends at the colon that starts
    /// its type, or at the space before its parameters.
    pub fn nameOf(s: &str) -> String {
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
                return CeNative::substr(&s, 0, i);
            }
            i += 1;
        }
        s.to_string()
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
    /// Declarations, read off the file. A parser would be better and would also
    /// be a second parser; this is what `fn name:` looks like, which is enough
    /// to offer the method you wrote eighty lines ago.
    pub fn membersOfFile(&self, lines: &[String], prefix: &str) -> Vec<Rc<RefCell<Completion>>> {
        let mut out: Vec<Rc<RefCell<Completion>>> = Vec::new();
        let mut li: i64 = 0;
        let n: i64 = lines.len() as i64;
        while li < n {
            let raw: String = lines[li as usize].clone();
            let s: String = raw.trim().to_string();
            let mut kind: String = String::new();
            let mut rest: String = String::new();
            if self.beginsWith(&s, "fn ") {
                kind = "method".to_string();
                rest = CeNative::substr(&s, 3, (CeNative::str_len(&s)));
            }
            if self.beginsWith(&s, "sfn ") {
                kind = "static".to_string();
                rest = CeNative::substr(&s, 4, (CeNative::str_len(&s)));
            }
            if self.beginsWith(&s, "def ") {
                kind = "field".to_string();
                rest = CeNative::substr(&s, 4, (CeNative::str_len(&s)));
            }
            if !kind.is_empty() {
                let name: String = RangerLanguage::nameOf(&rest);
                if !name.is_empty() {
                    if LanguageText::matchesPrefix(&name, prefix) {
                        out.push(Completion::create(
                            name.clone(),
                            kind.to_string(),
                            CompletionKind::member(),
                            0
                        ));
                    }
                }
            }
            li += 1;
        }
        out
    }
    /// --- shape -----------------------------------------------------------------
    /// One pass, carrying the two states that outlive a line: none. A Ranger
    /// string does not span lines and a comment ends with the line, so the state
    /// machine is per line and the bracket stack is per file.
    pub fn brackets(&self, lines: &[String], mut out: &mut Vec<Rc<RefCell<EditorDiagnostic>>>) {
        let mut stack_ch: Vec<i64> = Vec::new();
        let mut stack_line: Vec<i64> = Vec::new();
        let mut stack_col: Vec<i64> = Vec::new();
        let mut li: i64 = 0;
        let n: i64 = lines.len() as i64;
        while li < n {
            let s: String = lines[li as usize].clone();
            let __len: i64 = CeNative::str_len(&s);
            let mut i: i64 = 0;
            while i < __len {
                let ch: i64 = CeNative::char_at(&s, i);
                if ch == 59 {
                    // `;` — comment to the end of the line.
                    break;
                }
                if ch == 34 {
                    let close: i64 = self.stringEnd(&s, i + 1);
                    if close < 0 {
                        out.push(EditorDiagnostic::span(
                            li,
                            i,
                            __len,
                            DiagnosticSeverity::error(),
                            "string is never closed".to_string(),
                            "ranger".to_string()
                        ));
                        break;
                    }
                    i = close + 1;
                    continue;
                }
                if RangerLanguage::isOpen(ch) {
                    stack_ch.push(ch);
                    stack_line.push(li);
                    stack_col.push(i);
                }
                if RangerLanguage::isClose(ch) {
                    let depth: i64 = stack_ch.len() as i64;
                    if depth == 0 {
                        out.push(EditorDiagnostic::span(
                            li,
                            i,
                            i + 1,
                            DiagnosticSeverity::error(),
                            format!("stray '{}' — nothing opened here", char::from_u32(ch as u32).unwrap_or('\0')),
                            "ranger".to_string()
                        ));
                    } else {
                        let open_ch: i64 = stack_ch[(depth - 1) as usize];
                        let want: i64 = RangerLanguage::closerFor(open_ch);
                        if want != ch {
                            let msg: String = format!("'{}' closes a '{}' — expected '{}'", char::from_u32(ch as u32).unwrap_or('\0'), char::from_u32(open_ch as u32).unwrap_or('\0'), char::from_u32(want as u32).unwrap_or('\0'));
                            out.push(EditorDiagnostic::span(
                                li,
                                i,
                                i + 1,
                                DiagnosticSeverity::error(),
                                msg.clone(),
                                "ranger".to_string()
                            ));
                        }
                        stack_ch.pop();
                        stack_line.pop();
                        stack_col.pop();
                    }
                }
                i += 1;
            }
            li += 1;
        }
        let rest: i64 = stack_ch.len() as i64;
        if rest > 0 {
            let open_ch2: i64 = stack_ch[(rest - 1) as usize];
            let at_line: i64 = stack_line[(rest - 1) as usize];
            let at_col: i64 = stack_col[(rest - 1) as usize];
            out.push(EditorDiagnostic::span(
                at_line,
                at_col,
                at_col + 1,
                DiagnosticSeverity::error(),
                format!("'{}' is never closed", char::from_u32(open_ch2 as u32).unwrap_or('\0')),
                "ranger".to_string()
            ));
        }
    }
    pub fn stringEnd(&self, s: &str, from: i64) -> i64 {
        let mut i: i64 = from;
        let n: i64 = CeNative::str_len(&s);
        while i < n {
            let ch: i64 = CeNative::char_at(&s, i);
            if ch == 92 {
                i += 2;
                continue;
            }
            if ch == 34 {
                return i;
            }
            i += 1;
        }
        -1
    }
    /// --- the mistakes this repository actually makes ---------------------------
    /// From AGENTS.md, which lists them because they cost hours: a returned call
    /// needs its own parentheses, and a statement may not start with a
    /// parenthesised receiver. Both compile to misleading errors somewhere else,
    /// which is exactly the kind of thing an editor should say early.
    pub fn gotchas(&self, lines: &[String], mut out: &mut Vec<Rc<RefCell<EditorDiagnostic>>>) {
        let mut li: i64 = 0;
        let n: i64 = lines.len() as i64;
        while li < n {
            let raw: String = lines[li as usize].clone();
            let indent: i64 = self.indentOf(&raw);
            let body: String = CeNative::substr(&raw, indent, (CeNative::str_len(&raw)));
            if self.beginsWith(&body, "return ") && (CeNative::str_len(&body)) > 7 {
                let rest: String = CeNative::substr(&body, 7, (CeNative::str_len(&body)));
                let first: i64 = CeNative::char_at(&rest, 0);
                if first != 40 {
                    if self.looksLikeCall(&rest) {
                        let col: i64 = indent + 7;
                        out.push(EditorDiagnostic::span(
                            li,
                            col,
                            (CeNative::str_len(&raw)),
                            DiagnosticSeverity::warning(),
                            "a returned call needs its own parentheses: return (this.f())".to_string(),
                            "ranger".to_string()
                        ));
                    }
                }
            }
            if (CeNative::str_len(&body)) > 1 {
                let opener: i64 = CeNative::char_at(&body, 0);
                if opener == 40 {
                    // `(recv.method())` as a STATEMENT. Inside an expression it
                    // is fine, so only a line that starts with it is flagged.
                    if self.looksLikeReceiverCall(&body) {
                        out.push(EditorDiagnostic::span(
                            li,
                            indent,
                            (CeNative::str_len(&raw)),
                            DiagnosticSeverity::warning(),
                            "a statement may not start with a parenthesised receiver — bind it first".to_string(),
                            "ranger".to_string()
                        ));
                    }
                }
            }
            li += 1;
        }
    }
    pub fn indentOf(&self, s: &str) -> i64 {
        let mut i: i64 = 0;
        let n: i64 = CeNative::str_len(&s);
        while i < n {
            let ch: i64 = CeNative::char_at(&s, i);
            if ch != 32 && ch != 9 {
                return i;
            }
            i += 1;
        }
        n
    }
    pub fn beginsWith(&self, s: &str, prefix: &str) -> bool {
        let n: i64 = CeNative::str_len(&prefix);
        if (CeNative::str_len(&s)) < n {
            return false;
        }
        let head: String = CeNative::substr(&s, 0, n);
        head == prefix
    }
    /// `foo.bar()` or `foo(x)` — an unparenthesised call, rather than a value.
    pub fn looksLikeCall(&self, s: &str) -> bool {
        let mut saw_open: bool = false;
        let mut saw_close: bool = false;
        let mut i: i64 = 0;
        let n: i64 = CeNative::str_len(&s);
        while i < n {
            let ch: i64 = CeNative::char_at(&s, i);
            if ch == 40 {
                saw_open = true;
            }
            if ch == 41 {
                if saw_open {
                    saw_close = true;
                }
            }
            // An operator in the expression means it is arithmetic, not a bare
            // returned call, and those are legal.
            if ch == 43 {
                return false;
            }
            if ch == 63 {
                return false;
            }
            i += 1;
        }
        if !saw_open {
            return false;
        }
        saw_close
    }
    /// `(thing.method(` — the receiver form, as opposed to `(op a b)`, which is
    /// every other line in the language.
    pub fn looksLikeReceiverCall(&self, s: &str) -> bool {
        let mut i: i64 = 1;
        let n: i64 = CeNative::str_len(&s);
        let mut saw_dot: bool = false;
        while i < n {
            let ch: i64 = CeNative::char_at(&s, i);
            if ch == 32 {
                return false;
            }
            if ch == 40 {
                return saw_dot;
            }
            if ch == 46 {
                saw_dot = true;
            }
            i += 1;
        }
        false
    }
}

impl EditorLanguage for RangerLanguage {
    fn id(&self) -> String {
        "ranger".to_string()
    }
    fn label(&self) -> String {
        "Ranger".to_string()
    }
    fn handlesFile(&self, name: &str) -> bool {
        LanguageText::hasSuffix(name, ".rgr")
    }
    fn diagnose(&mut self, source: &str) -> Vec<Rc<RefCell<EditorDiagnostic>>> {
        let mut lines: Vec<String> = LanguageText::linesOf(source);
        self.quickDiagnose(&lines)
    }
    /// This plugin has one speed: it never parses, so its full answer is already
    /// cheap enough for every keystroke.
    fn quickDiagnose(&self, lines: &[String]) -> Vec<Rc<RefCell<EditorDiagnostic>>> {
        let mut out: Vec<Rc<RefCell<EditorDiagnostic>>> = Vec::new();
        self.brackets(lines, &mut out);
        self.gotchas(lines, &mut out);
        out
    }
    fn tokenize(&self, lines: &[String]) -> Vec<Rc<RefCell<CodeTokenLine>>> {
        return self.lexer.borrow().tokenize(lines);
    }
    /// Ranger's own: the operators first, because they are what a line is made
    /// of and they are the ones nobody remembers the spelling of.
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
        if prefix.is_empty() {
            return out;
        }
        // `this.` and `Class.` ask about members, and in a language with no
        // imports to chase, the file itself is the answer: every `fn`, `sfn`
        // and `def` in it.
        let start: i64 = LanguageText::prefixStart(&text, col);
        if start > 0 {
            if CeNative::char_at(&text, start - 1) == 46 {
                return self.membersOfFile(lines, &prefix);
            }
        }
        let mut ops: Vec<String> = RangerTokenizer::builtins();
        let mut i: i64 = 0;
        let n: i64 = ops.len() as i64;
        while i < n {
            let name: String = ops[i as usize].clone();
            if LanguageText::matchesPrefix(&name, &prefix) {
                out.push(Completion::create(
                    name.clone(),
                    "operator".to_string(),
                    CompletionKind::builtin(),
                    0
                ));
            }
            i += 1;
        }
        let mut kw: Vec<String> = RangerTokenizer::keywords();
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
                if !LanguageText::isInList(&word2, &ops) {
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
