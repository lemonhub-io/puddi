//! Runs the language conformance suite (conformance/run.mjs) against the
//! `pud` binary built by Cargo. Needs Node.js 22 or newer, like `pud test`.

use std::path::Path;
use std::process::Command;

#[test]
fn conformance_suite() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let status = Command::new("node")
        .arg(root.join("conformance/run.mjs"))
        .args(["--pud", env!("CARGO_BIN_EXE_pud"), "--strict-coverage"])
        .current_dir(root)
        .status()
        .expect("could not start `node`; the conformance suite needs Node.js 22 or newer");
    assert!(status.success(), "conformance suite failed; see the report above");
}
