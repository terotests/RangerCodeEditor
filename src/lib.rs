//! RangerCodeEditor: a code editor that paints into an EVG display list,
//! with JavaScript/TSX and Ranger language plugins, written in Ranger's Rust
//! syntax. Ranger compiles these modules, with the .rgr files beside them
//! that import this file, to every target; cargo builds them as one crate
//! (../Cargo.toml).

#![allow(non_snake_case, unused_imports)]

pub mod code_tokens;
pub mod completion;
pub mod editor_diagnostic;
pub mod editor_language;
pub mod js_tokens;
mod rgr;

/// Text in the target's own units, for crates that use this one.
pub use crate::rgr::CeNative;
