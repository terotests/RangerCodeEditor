//! The .rgr classes the crate's modules use, imported once.
//!
//! Ranger reads the classes of each file named here. rustc includes the
//! Rust module build.rs writes for CeRgr.rgr, which imports all of them.
//! `cfg(any())` is never on for rustc, and Ranger reads it.

#[allow(dead_code, unused, non_snake_case)]
mod rgr {
    ranger::import_rgr!("./CeRgr");
    #[cfg(any())]
    ranger::import_rgr!("CeNative");
}
pub use rgr::CeNative;
