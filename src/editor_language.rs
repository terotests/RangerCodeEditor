// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::code_tokens::CodeToken;
use crate::code_tokens::CodeTokenKind;
use crate::code_tokens::CodeTokenLine;
use crate::completion::Completion;
use crate::editor_diagnostic::EditorDiagnostic;
use crate::rgr::CeNative;

/// Text helpers every plugin needs. Shared because every language that has
/// identifiers agrees about this much, and a plugin that disagrees can ignore
/// them.
#[derive(Clone)]
pub struct LanguageText {}

impl LanguageText {
    /// The word being typed, ending at the caret. Shared because every language
    /// that has identifiers agrees about this much, and a plugin that disagrees
    /// can ignore it.
    pub fn prefixAt(line: &str, col: i64) -> String {
        let start: i64 = LanguageText::prefixStart(line, col);
        CeNative::substr(&line, start, col)
    }
    pub fn prefixStart(line: &str, col: i64) -> i64 {
        let mut i: i64 = col;
        let n: i64 = CeNative::str_len(&line);
        if i > n {
            i = n;
        }
        while i > 0 {
            let ch: i64 = CeNative::char_at(&line, i - 1);
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
            if ch == 36 {
                word = true;
            }
            if !word {
                return i;
            }
            i -= 1;
        }
        0
    }
    /// Case-insensitive prefix match — what every completion list does, because
    /// nobody wants to remember whether the API said `sheetRows` or `SheetRows`.
    pub fn matchesPrefix(candidate: &str, prefix: &str) -> bool {
        let pn: i64 = CeNative::str_len(&prefix);
        if pn == 0 {
            return true;
        }
        if (CeNative::str_len(&candidate)) < pn {
            return false;
        }
        let head: String = CeNative::substr(&candidate, 0, pn);
        LanguageText::lower(&head) == LanguageText::lower(prefix)
    }
    pub fn lower(s: &str) -> String {
        let mut out: String = String::new();
        let mut i: i64 = 0;
        let n: i64 = CeNative::str_len(&s);
        while i < n {
            let mut ch: i64 = CeNative::char_at(&s, i);
            if (ch >= 65 && ch <= 90) {
                ch += 32;
            }
            out.push_str(&char::from_u32(ch as u32).unwrap_or('\0').to_string());
            i += 1;
        }
        out
    }
    /// Every distinct word in the document, so a name typed once can be typed
    /// again with three letters. Cheap: one pass, no parse, and the caller caps
    /// how many it keeps.
    pub fn wordsIn(lines: &[String], skip: &str, limit: i64) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut li: i64 = 0;
        let ln: i64 = lines.len() as i64;
        while li < ln {
            let s: String = lines[li as usize].clone();
            let n: i64 = CeNative::str_len(&s);
            let mut i: i64 = 0;
            while i < n {
                let start: i64 = i;
                let mut is_word: bool = false;
                while i < n {
                    let ch: i64 = CeNative::char_at(&s, i);
                    let mut w: bool = false;
                    if (ch >= 65 && ch <= 90) {
                        w = true;
                    }
                    if (ch >= 97 && ch <= 122) {
                        w = true;
                    }
                    if ch == 95 {
                        w = true;
                    }
                    if is_word {
                        if (ch >= 48 && ch <= 57) {
                            w = true;
                        }
                    }
                    if !w {
                        break;
                    }
                    is_word = true;
                    i += 1;
                }
                if is_word {
                    let word: String = CeNative::substr(&s, start, i);
                    if (CeNative::str_len(&word)) > 2 {
                        if word != skip {
                            if !LanguageText::isInList(&word, &out) {
                                if (out.len() as i64) < limit {
                                    out.push(word.clone());
                                }
                            }
                        }
                    }
                } else {
                    i += 1;
                }
            }
            li += 1;
        }
        out
    }
    pub fn isInList(word: &str, list: &[String]) -> bool {
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
    pub fn hasSuffix(s: &str, suffix: &str) -> bool {
        let n: i64 = CeNative::str_len(&suffix);
        let __len: i64 = CeNative::str_len(&s);
        if __len < n {
            return false;
        }
        let tail: String = CeNative::substr(&s, __len - n, __len);
        tail == suffix
    }
    /// A source string as lines, the way the editor holds it — so a plugin that
    /// works line by line does not each invent its own splitter.
    pub fn linesOf(source: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut cur: String = String::new();
        let mut i: i64 = 0;
        let n: i64 = CeNative::str_len(&source);
        while i < n {
            let ch: i64 = CeNative::char_at(&source, i);
            if ch == 10 {
                out.push(cur.clone());
                cur = String::new();
            } else {
                if ch != 13 {
                    cur.push_str(&char::from_u32(ch as u32).unwrap_or('\0').to_string());
                }
            }
            i += 1;
        }
        out.push(cur);
        out
    }
}

/// ==============================================================================
/// EditorLanguage.rgr — the seam a language plugs into
/// ==============================================================================
///
/// The editor knows how to draw a squiggle under a range. It does not know what
/// a syntax error is, and it must not: JavaScript is one answer, Ranger is a
/// different one, and a third language should not need the editor to change.
///
/// So: a plugin answers three questions.
///
///     id()               what it calls itself
///     handlesFile(name)  is this document mine?
///     diagnose(source)   what is wrong with it, as ranges
///
/// This is the Language Server idea with the server taken out. Nothing here
/// spawns a process, opens a socket or serializes JSON — a plugin is a class in
/// the same binary, and `LanguageRegistry.add` is the whole installation step.
/// That is a deliberate limit rather than a stage: the editor runs in a browser
/// tab and inside a native SDL binary as well as under Node, and two of those
/// three have nowhere to run a server. When one is wanted later, the seam is
/// already the right shape — `diagnose` becomes a request and the record it
/// returns is already an LSP diagnostic.
///
/// What a plugin must not do is take long. `diagnose` runs on the document as it
/// is being typed, so a plugin that cannot answer in a millisecond should answer
/// with less: the JavaScript one parses under a line budget and falls back to
/// structural checks above it.
/// ==============================================================================
pub trait EditorLanguage {
    fn id(&self) -> String {
        "text".to_string()
    }
    fn label(&self) -> String {
        "Plain text".to_string()
    }
    /// By file name, because that is what a host knows before it has read the
    /// document. A plugin that wants to sniff the content can look at the source
    /// in `diagnose` and return nothing.
    fn handlesFile(&self, name: &str) -> bool {
        false
    }
    /// The full pass. It may be expensive — a real parse, a type check, a
    /// compiler — and the editor will keep it off the keystroke path when it is.
    fn diagnose(&mut self, source: &str) -> Vec<Rc<RefCell<EditorDiagnostic>>> {
        let mut none: Vec<Rc<RefCell<EditorDiagnostic>>> = Vec::new();
        none
    }
    /// The pass that is allowed to run on every keystroke. A plugin whose full
    /// answer is already cheap can point this at the same code; one that parses
    /// should answer here with whatever it can be sure of without parsing.
    ///
    /// Two speeds rather than one, because the alternative is a slow plugin
    /// making the editor slow: measured on a 2 000-line document, a full
    /// JavaScript parse per keystroke cost 72 ms, and the structural pass 1 ms.
    fn quickDiagnose(&self, lines: &[String]) -> Vec<Rc<RefCell<EditorDiagnostic>>> {
        let mut none: Vec<Rc<RefCell<EditorDiagnostic>>> = Vec::new();
        none
    }
    /// What the text IS, for the painter. The base answer is "one plain run per
    /// line", which is what an unknown file should look like: readable, and
    /// obviously not claiming to understand itself.
    fn tokenize(&self, lines: &[String]) -> Vec<Rc<RefCell<CodeTokenLine>>> {
        let mut out: Vec<Rc<RefCell<CodeTokenLine>>> = Vec::new();
        let mut i: i64 = 0;
        let n: i64 = lines.len() as i64;
        while i < n {
            let text: String = lines[i as usize].clone();
            let mut line: Rc<RefCell<CodeTokenLine>> = Rc::new(RefCell::new(CodeTokenLine::new()));
            let __len: i64 = CeNative::str_len(&text);
            if __len > 0 {
                line.borrow_mut().tokens.push(CodeToken::create(0, __len, CodeTokenKind::plain()));
            }
            out.push(line.clone());
            i += 1;
        }
        out
    }
    /// What could be typed here. `line` and `col` are where the caret is; the
    /// plugin decides what counts as the word being completed and what may
    /// follow it, because "what is a word" is already a language question.
    ///
    /// Called on a keystroke, so the same rule as quickDiagnose applies: answer
    /// from what is cheap to know. Neither plugin here parses to do it.
    fn complete(&self, lines: &[String], line: i64, col: i64) -> Vec<Rc<RefCell<Completion>>> {
        let mut none: Vec<Rc<RefCell<Completion>>> = Vec::new();
        none
    }
}

/// The plugin a document nothing claims gets: plain runs, no complaints. An
/// unknown file type is not an error.
#[derive(Clone)]
pub struct PlainText {}

impl PlainText {
    pub fn new() -> Self {
        Self {}
    }
}

impl EditorLanguage for PlainText {}


/// The installed plugins, in the order they were added. First one that claims a
/// file gets it; a document nothing claims gets the fallback, which complains
/// about nothing.
#[derive(Clone)]
pub struct LanguageRegistry {
    pub plugins: Vec<Rc<RefCell<dyn EditorLanguage>>>,
    pub none: Rc<RefCell<dyn EditorLanguage>>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            none: Rc::new(RefCell::new(PlainText::new())),
        }
    }
    pub fn add(&mut self, lang: Rc<RefCell<dyn EditorLanguage>>) {
        self.plugins.push(lang.clone());
    }
    pub fn count(&self) -> i64 {
        self.plugins.len() as i64
    }
    pub fn at(&self, i: i64) -> Rc<RefCell<dyn EditorLanguage>> {
        self.plugins[i as usize].clone()
    }
    pub fn byId(&self, id: &str) -> Rc<RefCell<dyn EditorLanguage>> {
        let mut i: i64 = 0;
        let n: i64 = self.plugins.len() as i64;
        while i < n {
            let mut lang: Rc<RefCell<dyn EditorLanguage>> = self.plugins[i as usize].clone();
            if lang.borrow().id() == id {
                return lang;
            }
            i += 1;
        }
        self.none.clone()
    }
    pub fn forFile(&self, name: &str) -> Rc<RefCell<dyn EditorLanguage>> {
        let mut i: i64 = 0;
        let n: i64 = self.plugins.len() as i64;
        while i < n {
            let mut lang: Rc<RefCell<dyn EditorLanguage>> = self.plugins[i as usize].clone();
            if lang.borrow().handlesFile(name) {
                return lang;
            }
            i += 1;
        }
        self.none.clone()
    }
}
