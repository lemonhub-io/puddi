//! Compilation pipeline: load sources, parse, derive, resolve, elaborate,
//! generate code.

use crate::ast;
use crate::codegen;
use crate::derive;
use crate::diag::{Diag, SourceMap};
use crate::elab::Global;
use crate::ir::FnInst;
use crate::lexer;
use crate::parser;
use crate::program::{self, FnOwner, ModSource, Pkg};

pub const PRELUDE: &str = include_str!("prelude.pud");

pub const STD_MODULES: &[(&str, &str)] = &[
    ("std.math", include_str!("../std/math.pud")),
    ("std.log", include_str!("../std/log.pud")),
    ("std.time", include_str!("../std/time.pud")),
    ("std.random", include_str!("../std/random.pud")),
    ("std.io", include_str!("../std/io.pud")),
    ("std.fs", include_str!("../std/fs.pud")),
    ("std.env", include_str!("../std/env.pud")),
    ("std.json", include_str!("../std/json.pud")),
];

pub struct UserFile {
    /// Module path, e.g. `main` or `net.http`.
    pub module: String,
    /// Display path for diagnostics.
    pub display: String,
    pub text: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Build,
    Test,
    Check,
}

pub struct Compiled {
    pub wasm: Vec<u8>,
    pub wat: String,
    pub tests: Vec<(String, String)>,
    pub warnings: Vec<Diag>,
    pub has_main: bool,
}

fn parse_file(sm: &mut SourceMap, display: String, text: String) -> Result<ast::Module, Vec<Diag>> {
    let fid = sm.add(display, text);
    let text = sm.files[fid as usize].text.clone();
    let toks = lexer::lex(&text, fid).map_err(|d| vec![d])?;
    parser::parse_module(toks).map_err(|d| vec![d])
}

fn with_derives(sm: &mut SourceMap, module: &mut ast::Module, name: &str) -> Result<(), Vec<Diag>> {
    let src = derive::derive_source(module)?;
    if !src.is_empty() {
        let extra = parse_file(sm, format!("<derive in {name}>"), src)?;
        module.items.extend(extra.items);
    }
    Ok(())
}

pub fn compile(sm: &mut SourceMap, files: Vec<UserFile>, root: &str, target: &str, mode: Mode) -> Result<Compiled, Vec<Diag>> {
    let mut errs: Vec<Diag> = Vec::new();
    let mut sources: Vec<ModSource> = Vec::new();

    // prelude
    let prelude_text = format!("{PRELUDE}\n// ---- generated numeric prelude ----\n{}", crate::numeric::generate());
    match parse_file(sm, "<prelude>".into(), prelude_text) {
        Ok(m) => sources.push(ModSource { path: "prelude".into(), pkg: Pkg::Prelude, ast: m }),
        Err(e) => return Err(e),
    }
    // std
    for (path, text) in STD_MODULES {
        if program::std_available(path, target) != Some(true) {
            continue;
        }
        match parse_file(sm, format!("<{path}>"), text.to_string()) {
            Ok(mut m) => {
                if let Err(e) = with_derives(sm, &mut m, path) {
                    errs.extend(e);
                }
                sources.push(ModSource { path: path.to_string(), pkg: Pkg::Std, ast: m });
            }
            Err(e) => errs.extend(e),
        }
    }
    // user
    let mut has_main = false;
    for f in files {
        match parse_file(sm, f.display.clone(), f.text) {
            Ok(mut m) => {
                if let Err(e) = with_derives(sm, &mut m, &f.module) {
                    errs.extend(e);
                }
                if f.module == root && mode != Mode::Test {
                    let main = m.items.iter().find_map(|it| match &it.kind {
                        ast::ItemKind::Fn(fd) if fd.name.name == "main" => Some(fd.clone()),
                        _ => None,
                    });
                    if let Some(fd) = main {
                        has_main = true;
                        let call = if fd.is_async { "main().__await()" } else { "main()" };
                        let unit_entry = format!("fn __entry() -> Int {{\n    {call}\n    0\n}}\n");
                        let result_entry = format!("fn __entry() -> Int {{\n    match {call} {{\n        Ok(_) -> 0\n        Err(e) -> {{\n            eprint(\"error: {{e}}\")\n            1\n        }}\n    }}\n}}\n");
                        let entry = match &fd.ret {
                            None => Some(unit_entry),
                            Some(ast::TypeExpr::Tuple(ts, _)) if ts.is_empty() => Some(unit_entry),
                            Some(ast::TypeExpr::Path { segs, .. }) if segs.last().map(|s| s.name == "Result").unwrap_or(false) => Some(result_entry),
                            Some(t) => {
                                errs.push(Diag::new(t.span(), "`main` must return `()` or `Result<(), E>`"));
                                None
                            }
                        };
                        if !fd.params.is_empty() {
                            errs.push(Diag::new(fd.name.span, "`main` takes no parameters; use `std.env.args()` for command-line arguments"));
                        }
                        if let Some(src) = entry {
                            match parse_file(sm, "<entry>".into(), src) {
                                Ok(e) => m.items.extend(e.items),
                                Err(e) => errs.extend(e),
                            }
                        }
                    }
                }
                sources.push(ModSource { path: f.module.clone(), pkg: Pkg::User, ast: m });
            }
            Err(e) => errs.extend(e),
        }
    }
    if !errs.is_empty() {
        return Err(errs);
    }

    let prog = program::build(sources, target)?;
    let mut g = Global::new(&prog);
    let root_mod = prog.modules.iter().position(|m| m.path == root && m.pkg == Pkg::User);
    let mut entry = None;
    let mut tests = Vec::new();
    for (fid, f) in prog.fns.iter().enumerate() {
        if f.pkg != Pkg::User {
            continue;
        }
        if let Some(name) = &f.test_name {
            if mode != Mode::Build {
                let idx = g.request(FnInst { fn_id: fid, targs: Vec::new() });
                tests.push((name.clone(), idx));
            }
            continue;
        }
        let generic = !f.generics.is_empty();
        let ok = match f.owner {
            FnOwner::Free => !generic,
            FnOwner::Impl(_) => !generic,
            FnOwner::Trait(_) => false,
        };
        if !ok {
            continue;
        }
        let idx = g.request(FnInst { fn_id: fid, targs: Vec::new() });
        if f.name == "__entry" && Some(f.module) == root_mod {
            entry = Some(idx);
        }
    }
    g.run();
    let mut warnings = prog.warnings.clone();
    warnings.extend(g.warnings.clone());
    if !g.errors.is_empty() {
        let mut errs = g.errors.clone();
        errs.sort_by_key(|d| (d.span.file, d.span.lo));
        return Err(errs);
    }
    if mode == Mode::Check {
        return Ok(Compiled { wasm: Vec::new(), wat: String::new(), tests: Vec::new(), warnings, has_main });
    }
    let out = codegen::generate(&prog, &g, entry, &tests).map_err(|m| vec![Diag::new(crate::diag::Span::new(u32::MAX, 0, 0), m)])?;
    Ok(Compiled { wasm: out.wasm, wat: out.wat, tests: out.tests, warnings, has_main })
}
