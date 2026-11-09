// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;


/// ==============================================================================
/// EditorDiagnostic.rgr — one complaint about one span of text
/// ==============================================================================
///
/// The record every language plugin produces and the editor paints. It is
/// deliberately the shape a Language Server Protocol diagnostic has — a range, a
/// severity, a message and the name of whatever said so — because that shape has
/// already been argued about by everyone, and because a later move to a real
/// server should be a change of TRANSPORT, not of meaning.
///
/// Positions are 0-based, like the editor's own. Every plugin converts into that
/// on its way out; nothing downstream has to remember whose parser counted from
/// one.
/// ==============================================================================
#[derive(Clone)]
pub struct DiagnosticSeverity {
}

impl DiagnosticSeverity {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn error() -> i64 {
        0
    }
    pub fn warning() -> i64 {
        1
    }
    pub fn info() -> i64 {
        2
    }
    pub fn hint() -> i64 {
        3
    }
    pub fn name(severity: i64) -> String {
        if severity == 0 {
            return "error".to_string();
        }
        if severity == 1 {
            return "warning".to_string();
        }
        if severity == 2 {
            return "info".to_string();
        }
        "hint".to_string()
    }
}

#[derive(Clone)]
pub struct EditorDiagnostic {
    pub line: i64,
    pub col: i64,
    pub endLine: i64,
    pub endCol: i64,
    pub severity: i64,
    pub message: String,
    /// Which plugin said it: "javascript", "ranger", "componentengine". Shown to
    /// the user and used by a host that wants to clear one source's complaints
    /// without touching another's.
    pub source: String,
}

impl EditorDiagnostic {
    pub fn new() -> Self {
        Self {
            line: 0,
            col: 0,
            endLine: 0,
            endCol: 0,
            severity: 0,
            message: String::new(),
            source: String::new(),
        }
    }
    /// The common case: a span inside one line. Not called `at` — that name is
    /// taken by an operator, and a method with it compiles but never resolves.
    pub fn span(line: i64, col: i64, end_col: i64, severity: i64, message: String, source: String) -> Rc<RefCell<EditorDiagnostic>> {
        let mut d: Rc<RefCell<EditorDiagnostic>> = Rc::new(RefCell::new(EditorDiagnostic::new()));
        d.borrow_mut().line = line;
        d.borrow_mut().col = col;
        d.borrow_mut().endLine = line;
        d.borrow_mut().endCol = end_col;
        d.borrow_mut().severity = severity;
        d.borrow_mut().message = message.clone();
        d.borrow_mut().source = source.clone();
        d
    }
    pub fn coversLine(&self, l: i64) -> bool {
        if l < self.line {
            return false;
        }
        l <= self.endLine
    }
    pub fn label(&self) -> String {
        let r#where: String = format!("{}:{}", (self.line + 1).to_string(), self.col + 1);
        format!("{} {}  {}", DiagnosticSeverity::name(self.severity.clone()), r#where, self.message)
    }
}
