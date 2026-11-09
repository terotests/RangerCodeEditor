//! The .rgr classes the crate's modules use, imported once.
//!
//! Ranger reads the classes of each file named here. rustc includes the
//! Rust module build.rs writes for CeRgr.rgr, which imports all of them.
//! `cfg(any())` is never on for rustc, and Ranger reads it.

#[allow(dead_code, unused, non_snake_case)]
mod rgr {
    ranger::import_rgr!("./CeRgr");
    #[cfg(any())]
    ranger::import_rgr!("../../../lib/evg/EVGColor");
    #[cfg(any())]
    ranger::import_rgr!("../../../lib/evg/EVGDisplayList");
    #[cfg(any())]
    ranger::import_rgr!("../../game_engine/framebuffer");
    #[cfg(any())]
    ranger::import_rgr!("../../game_engine/ui/UIContext");
    #[cfg(any())]
    ranger::import_rgr!("../../game_engine/ui/UIInput");
    #[cfg(any())]
    ranger::import_rgr!("../../game_engine/ui/UITextRenderer");
    #[cfg(any())]
    ranger::import_rgr!("../../text_editor/src/EditorBuffer");
    #[cfg(any())]
    ranger::import_rgr!("../../text_editor/src/EditorLayout");
    #[cfg(any())]
    ranger::import_rgr!("../../text_editor/src/EditorSelection");
    #[cfg(any())]
    ranger::import_rgr!("../../text_editor/src/EditorStyle");
    #[cfg(any())]
    ranger::import_rgr!("../../ts_parser/ts_lexer");
    #[cfg(any())]
    ranger::import_rgr!("../../ts_parser/ts_parser_simple");
    #[cfg(any())]
    ranger::import_rgr!("../../ts_parser/ts_token");
    #[cfg(any())]
    ranger::import_rgr!("../../pdf_writer/src/fonts/FontManager");
    #[cfg(any())]
    ranger::import_rgr!("CeNative");
}
pub use rgr::{CeNative, FontManager, EVGColor, EVGDisplayList, EVGDrawCmd, EditorBuffer, EditorLayout, EditorPos, EditorSelection, EditorWord, SoftCanvas, TSLexer, TSNode, TSParserSimple, Token, UIContext, UIInput, UIKey, UITextRenderer};
