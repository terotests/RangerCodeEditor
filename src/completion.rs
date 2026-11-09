// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;


/// ==============================================================================
/// Completion.rgr — one thing the editor could type for you
/// ==============================================================================
///
/// Deliberately thin. A completion is a word, a note about where it came from,
/// and a kind the painter can colour by — not a snippet engine, not a template
/// language, and not a promise about types. What makes the list useful is which
/// words are in it, and that is the plugin's business.
/// ==============================================================================
#[derive(Clone)]
pub struct CompletionKind {
}

impl CompletionKind {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn keyword() -> i64 {
        0
    }
    pub fn builtin() -> i64 {
        1
    }
    pub fn word() -> i64 {
        2
    }
    pub fn member() -> i64 {
        3
    }
    pub fn name(kind: i64) -> String {
        if kind == 0 {
            return "keyword".to_string();
        }
        if kind == 1 {
            return "api".to_string();
        }
        if kind == 3 {
            return "member".to_string();
        }
        "word".to_string()
    }
}

#[derive(Clone)]
pub struct Completion {
    pub label: String,
    /// What is written where the caret is. Usually the label; a plugin that
    /// completes `sheetRows(` rather than `sheetRows` puts that here.
    pub insert: String,
    /// The grey half of the row: where this came from, or what it is.
    pub detail: String,
    pub kind: i64,
    /// Lower sorts first. A plugin decides what "better" means for its language;
    /// the editor does not re-rank.
    pub rank: i64,
}

impl Completion {
    pub fn new() -> Self {
        Self {
            label: String::new(),
            insert: String::new(),
            detail: String::new(),
            kind: 0,
            rank: 0,
        }
    }
    pub fn create(label: String, detail: String, kind: i64, rank: i64) -> Rc<RefCell<Completion>> {
        let mut c: Rc<RefCell<Completion>> = Rc::new(RefCell::new(Completion::new()));
        c.borrow_mut().label = label.clone();
        c.borrow_mut().insert = label.clone();
        c.borrow_mut().detail = detail.clone();
        c.borrow_mut().kind = kind;
        c.borrow_mut().rank = rank;
        c
    }
}
