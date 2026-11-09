// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;


/// ==============================================================================
/// CodeTokens.rgr — what a coloured run of characters is, in no language
/// ==============================================================================
///
/// The record every lexer produces and the editor paints. It names KINDS, not
/// JavaScript: a keyword is a keyword in Ranger too, and a plugin for a third
/// language should not have to invent a fourth token type to say "this bit is a
/// string".
///
/// A kind the painter does not know would simply be drawn plain, so a language
/// may use as few of these as it likes.
/// ==============================================================================
#[derive(Clone)]
pub struct CodeTokenKind {
}

impl CodeTokenKind {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn plain() -> i64 {
        0
    }
    pub fn ident() -> i64 {
        1
    }
    pub fn keyword() -> i64 {
        2
    }
    pub fn builtin() -> i64 {
        3
    }
    pub fn number() -> i64 {
        4
    }
    pub fn text() -> i64 {
        5
    }
    pub fn comment() -> i64 {
        6
    }
    pub fn punct() -> i64 {
        7
    }
    pub fn tag() -> i64 {
        8
    }
    pub fn attr() -> i64 {
        9
    }
}

#[derive(Clone)]
pub struct CodeToken {
    pub start: i64,
    pub length: i64,
    pub kind: i64,
}

impl CodeToken {
    pub fn new() -> Self {
        Self {
            start: 0,
            length: 0,
            kind: 0,
        }
    }
    pub fn create(start: i64, length: i64, kind: i64) -> Rc<RefCell<CodeToken>> {
        let mut t: Rc<RefCell<CodeToken>> = Rc::new(RefCell::new(CodeToken::new()));
        t.borrow_mut().start = start;
        t.borrow_mut().length = length;
        t.borrow_mut().kind = kind;
        t
    }
}

#[derive(Clone)]
pub struct CodeTokenLine {
    pub tokens: Vec<Rc<RefCell<CodeToken>>>,
}

impl CodeTokenLine {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
        }
    }
    pub fn count(&self) -> i64 {
        self.tokens.len() as i64
    }
    pub fn at(&self, i: i64) -> Rc<RefCell<CodeToken>> {
        self.tokens[i as usize].clone()
    }
}
