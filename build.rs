// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Writes the .rgr classes the crate includes with `ranger::import_rgr!`
// (src/rgr.rs: src/CeRgr.rgr and what it imports: EVG, the UI context, the
// text buffer, the TypeScript parser) as a Rust module into OUT_DIR, with the
// Ranger checkout's compiler (RANGER_HOME, else the checkout this directory
// sits in at gallery/codeeditor). Needs node.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let ranger = std::env::var("RANGER_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest.join("../.."));
    let rgrc = ranger.join("dist/rgrc.js");
    let out = std::env::var("OUT_DIR").unwrap();
    let run = Command::new("node")
        .arg("--max-old-space-size=8192")
        .arg(&rgrc)
        .args(["-l=rust", "-rust-module"])
        .arg(ranger.join("gallery/codeeditor/src/CeRgr.rgr"))
        .arg(format!("-d={}", out))
        .arg("-o=CeRgr.rs")
        .current_dir(&ranger)
        .output()
        .expect("node could not run Ranger's rgrc");
    let log = String::from_utf8_lossy(&run.stdout).to_string() + &String::from_utf8_lossy(&run.stderr);
    if !run.status.success() || log.contains("[FAIL]") {
        panic!("rgrc could not compile src/CeRgr.rgr ({}):\n{}", rgrc.display(), log);
    }
    // EVGHostTextMeasurer keeps a host's measuring function as a boxed
    // closure in an optional field, which Ranger's Rust writer cannot write
    // yet (MFilesAPI's builds patch it the same way). Only a JavaScript host
    // installs one; the editor measures through UITextRenderer.
    let file = PathBuf::from(&out).join("CeRgr.rs");
    let src = std::fs::read_to_string(&file).expect("rgrc wrote no CeRgr.rs");
    let src = src.replace("    self.metric_fn = Some(f);\n", "    let _ = f;\n").replace(
        "self.metric_fn.clone().unwrap();",
        "&mut |_: i64, _: String, _: String, _: f64, _: bool, _: bool| -> f64 { 0.0 };",
    );
    std::fs::write(&file, src).unwrap();
    println!("cargo:rerun-if-changed=src/CeRgr.rgr");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RANGER_HOME");
    println!("cargo:rerun-if-changed={}", ranger.join("lib/evg").display());
}
