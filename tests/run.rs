//! Conformance tests: compile and run every program in tests/run with Node.js
//! and compare its output; check that programs in tests/fail are rejected
//! with the expected diagnostics.

use std::path::Path;
use std::process::Command;

fn pud() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pud"))
}

fn sorted(dir: &str, ext: &str) -> Vec<std::path::PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().map(|x| x == ext).unwrap_or(false))
        .collect();
    v.sort();
    v
}

#[test]
fn run_programs() {
    let mut failures = Vec::new();
    for src in sorted("tests/run", "pud") {
        let expected_out = src.with_extension("out");
        let is_test = std::fs::read_to_string(&src).unwrap().contains("\ntest \"") && !expected_out.exists();
        let out = if is_test { pud().arg("test").arg(&src).output().unwrap() } else { pud().arg("run").arg(&src).output().unwrap() };
        let code = out.status.code().unwrap_or(-1);
        let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        let want_code: i32 = std::fs::read_to_string(src.with_extension("code")).map(|s| s.trim().parse().unwrap()).unwrap_or(if is_test { 1 } else { 0 });
        if code != want_code {
            failures.push(format!("{}: exit code {code}, expected {want_code}\n{text}", src.display()));
            continue;
        }
        if let Ok(want) = std::fs::read_to_string(&expected_out) {
            if text != want {
                failures.push(format!("{}: output differs\n--- got\n{text}\n--- expected\n{want}", src.display()));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn reject_programs() {
    let mut failures = Vec::new();
    for src in sorted("tests/fail", "pud") {
        let out = pud().arg("check").arg(&src).output().unwrap();
        let text = String::from_utf8_lossy(&out.stderr).to_string();
        if out.status.success() {
            failures.push(format!("{}: compiled but should have failed", src.display()));
            continue;
        }
        let want = std::fs::read_to_string(src.with_extension("err")).unwrap();
        for line in want.lines().filter(|l| !l.trim().is_empty()) {
            if !text.contains(line) {
                failures.push(format!("{}: missing diagnostic `{line}`\n{text}", src.display()));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
