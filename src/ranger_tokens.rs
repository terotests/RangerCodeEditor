// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::code_tokens::CodeToken;
use crate::code_tokens::CodeTokenKind;
use crate::code_tokens::CodeTokenLine;
use crate::rgr::CeNative;

/// ==============================================================================
/// RangerTokens.rgr — the same token kinds, from a different language
/// ==============================================================================
///
/// Ranger is S-expressions: `;` runs to the end of the line, strings are double
/// quoted, and the words that matter are the declaration keywords rather than
/// JavaScript's. Twenty lines of difference, and the editor cannot tell — it is
/// handed the same `CodeTokenLine`s and paints them the same way.
///
/// That is the test of whether the seam is real. A language whose colours had to
/// be added to the painter would not be a plugin.
/// ==============================================================================
#[derive(Clone)]
pub struct RangerTokenizer {
}

impl RangerTokenizer {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn keywords() -> Vec<String> {
        vec!["class".to_string(), "def".to_string(), "fn".to_string(), "sfn".to_string(), "return".to_string(), "if".to_string(), "if!".to_string(), "while".to_string(), "for".to_string(), "switch".to_string(), "case".to_string(), "default".to_string(), "break".to_string(), "continue".to_string(), "new".to_string(), "this".to_string(), "Import".to_string(), "Extends".to_string(), "Constructor".to_string(), "record".to_string(), "Enum".to_string(), "true".to_string(), "false".to_string(), "null".to_string(), "static".to_string(), "throw".to_string(), "try".to_string(), "print".to_string(), "push".to_string(), "clear".to_string(), "set".to_string(), "get".to_string(), "has".to_string(), "unwrap".to_string(), "operator".to_string()].clone()
    }
    /// The operators a Ranger file is mostly made of. Colouring them as builtins
    /// is the same favour the JavaScript plugin does for the workbook API: a
    /// name that lights up is a name the language knows.
    pub fn builtins() -> Vec<String> {
        vec!["array_length".to_string(), "strlen".to_string(), "substring".to_string(), "charAt".to_string(), "strfromcode".to_string(), "to_string".to_string(), "to_int".to_string(), "to_double".to_string(), "itemAt".to_string(), "set_at".to_string(), "remove_index".to_string(), "removeLast".to_string(), "str_append".to_string(), "strsplit".to_string(), "trim".to_string(), "idiv".to_string(), "read_file".to_string(), "write_file".to_string(), "buffer_length".to_string(), "buffer_get".to_string(), "wall_clock_ms".to_string(), "keys".to_string(), "values".to_string()].clone()
    }
    pub fn isDigit(ch: i64) -> bool {
        if ch < 48 {
            return false;
        }
        ch <= 57
    }
    pub fn isWordChar(ch: i64) -> bool {
        if (ch >= 65 && ch <= 90) {
            return true;
        }
        if (ch >= 97 && ch <= 122) {
            return true;
        }
        if ch == 95 {
            return true;
        }
        if ch == 33 {
            // `if!` and `!null?` are words in this language.
            return true;
        }
        if ch == 63 {
            return true;
        }
        ch > 127
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
    pub fn tokenize(&self, lines: &[String]) -> Vec<Rc<RefCell<CodeTokenLine>>> {
        let mut out: Vec<Rc<RefCell<CodeTokenLine>>> = Vec::new();
        let mut i: i64 = 0;
        let n: i64 = lines.len() as i64;
        while i < n {
            out.push(self.tokenizeLine(&lines[i as usize].clone()));
            i += 1;
        }
        out
    }
    pub fn tokenizeLine(&self, line: &str) -> Rc<RefCell<CodeTokenLine>> {
        let mut out: Rc<RefCell<CodeTokenLine>> = Rc::new(RefCell::new(CodeTokenLine::new()));
        let n: i64 = CeNative::str_len(&line);
        let mut i: i64 = 0;
        let mut kw: Vec<String> = RangerTokenizer::keywords();
        let mut bi: Vec<String> = RangerTokenizer::builtins();
        while i < n {
            let ch: i64 = CeNative::char_at(&line, i);
            if ch == 32 || ch == 9 {
                i += 1;
                continue;
            }
            if ch == 59 {
                out.borrow_mut().tokens.push(CodeToken::create(i, n - i, CodeTokenKind::comment()));
                i = n;
                continue;
            }
            if ch == 34 {
                let close: i64 = self.stringEnd(line, i + 1);
                let mut stop: i64 = n;
                if close >= 0 {
                    stop = close + 1;
                }
                out.borrow_mut().tokens.push(CodeToken::create(i, stop - i, CodeTokenKind::text()));
                i = stop;
                continue;
            }
            if RangerTokenizer::isDigit(ch) {
                let num_end: i64 = self.readNumber(line, i);
                out.borrow_mut().tokens.push(CodeToken::create(i, num_end - i, CodeTokenKind::number()));
                i = num_end;
                continue;
            }
            if RangerTokenizer::isWordChar(ch) {
                let word_end: i64 = self.readWord(line, i);
                let word: String = CeNative::substr(&line, i, word_end);
                let mut kind: i64 = CodeTokenKind::ident();
                if RangerTokenizer::wordIn(&word, &kw) {
                    kind = CodeTokenKind::keyword();
                } else {
                    if RangerTokenizer::wordIn(&word, &bi) {
                        kind = CodeTokenKind::builtin();
                    } else {
                        // `name:type` — the annotation half is the type, and a
                        // type is the closest thing Ranger has to an attribute.
                        if word_end < n {
                            if CeNative::char_at(&line, word_end) == 58 {
                                kind = CodeTokenKind::attr();
                            }
                        }
                    }
                }
                out.borrow_mut().tokens.push(CodeToken::create(i, word_end - i, kind));
                i = word_end;
                continue;
            }
            out.borrow_mut().tokens.push(CodeToken::create(i, 1, CodeTokenKind::punct()));
            i += 1;
        }
        out
    }
    pub fn stringEnd(&self, line: &str, from: i64) -> i64 {
        let mut i: i64 = from;
        let n: i64 = CeNative::str_len(&line);
        while i < n {
            let ch: i64 = CeNative::char_at(&line, i);
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
    pub fn readWord(&self, line: &str, from: i64) -> i64 {
        let mut i: i64 = from;
        let n: i64 = CeNative::str_len(&line);
        while i < n {
            let ch: i64 = CeNative::char_at(&line, i);
            if !RangerTokenizer::isWordChar(ch) {
                if !RangerTokenizer::isDigit(ch) {
                    return i;
                }
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
            let mut ok: bool = RangerTokenizer::isDigit(ch);
            if ch == 46 {
                ok = true;
            }
            if !ok {
                return i;
            }
            i += 1;
        }
        n
    }
}
