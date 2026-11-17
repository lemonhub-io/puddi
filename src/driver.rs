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
pub const SOFT: &str = include_str!("soft.pud");
pub const PLATFORM_WASI: &str = include_str!("platform_wasi.pud");
/// Worker operations for `wasi` components, through the puddi:runtime interfaces.
const PLATFORM_WASI_WORKERS: &str = r#"
pub fn __plat_worker(op: Int, arg: Str) -> Str {
    let parts = arg.split("\0")
    match op {
        50 -> "{__w_puddi_runtime_workers.start(__cast<Int, U32>(arg.parse_int().unwrap()))}"
        51 -> {
            __w_puddi_runtime_workers.send(__cast<Int, U32>(parts[0].parse_int().unwrap()), parts.slice(1, parts.len()).join("\0"))
            ""
        }
        52 -> {
            __w_puddi_runtime_workers.close(__cast<Int, U32>(arg.parse_int().unwrap()))
            ""
        }
        53 -> {
            __w_puddi_runtime_port.send(arg)
            ""
        }
        54 -> {
            __w_puddi_runtime_port.done()
            ""
        }
        103 -> match __w_puddi_runtime_workers.recv(__cast<Int, U32>(arg.parse_int().unwrap())) {
            Some(m) -> "1" + m
            None -> ""
        }
        104 -> match __w_puddi_runtime_port.recv() {
            Some(m) -> "1" + m
            None -> ""
        }
        _ -> ""
    }
}

pub fn __plat_worker_start(entry: U32) { __worker_dispatch(__cast<U32, Int>(entry)) }
"#;

const PLATFORM_WASI_NO_WORKERS: &str = r#"
pub fn __plat_worker(op: Int, arg: Str) -> Str {
    __plat().ok = false
    "workers are not available in this component"
}
"#;

pub const PLATFORM_PLUGIN: &str = include_str!("platform_plugin.pud");

/// WASI interfaces the `wasi` platform layer uses.
const WASI_PLATFORM: &[&str] = &[
    "wasi:cli/stdout",
    "wasi:cli/stderr",
    "wasi:cli/stdin",
    "wasi:cli/environment",
    "wasi:cli/exit",
    "wasi:clocks/monotonic-clock",
    "wasi:clocks/system-clock",
    "wasi:random/random",
    "wasi:filesystem/types",
    "wasi:filesystem/preopens",
    "wasi:http/types",
    "wasi:http/client",
];

/// Functions of the platform layer that replace the JavaScript host imports.
pub const PLATFORM_FNS: &[&str] = &["__plat_write", "__plat_panic", "__plat_fmt", "__plat_parse", "__plat_math", "__plat_host", "__plat_ok", "__plat_host_async", "__plat_should_throw", "__plat_thread_main", "__plat_thread_done"];

/// How to build a component.
pub struct ComponentOpts {
    pub pkg_dir: Option<std::path::PathBuf>,
    /// World spec from pud.toml, e.g. `wasi:cli/command` or `wit/host.wit#plugin`.
    pub world: String,
}

pub const STD_MODULES: &[(&str, &str)] = &[
    ("std.math", include_str!("../std/math.pud")),
    ("std.log", include_str!("../std/log.pud")),
    ("std.time", include_str!("../std/time.pud")),
    ("std.random", include_str!("../std/random.pud")),
    ("std.io", include_str!("../std/io.pud")),
    ("std.fs", include_str!("../std/fs.pud")),
    ("std.env", include_str!("../std/env.pud")),
    ("std.json", include_str!("../std/json.pud")),
    ("std.http", include_str!("../std/http.pud")),
    ("std.worker", include_str!("../std/worker.pud")),
    ("std.dom", include_str!("../std/dom.pud")),
];

pub struct UserFile {
    /// Module path, e.g. `main` or `net.http`; dependency modules carry their package prefix.
    pub module: String,
    pub pkg: Pkg,
    pub prefix: String,
    pub aliases: std::collections::HashMap<String, String>,
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
    /// The component, for component builds.
    pub component: Option<Vec<u8>>,
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

fn internal(m: String) -> Vec<Diag> {
    vec![Diag::new(crate::diag::Span::new(u32::MAX, 0, 0), m)]
}

pub fn compile(sm: &mut SourceMap, files: Vec<UserFile>, root: &str, target: &str, mode: Mode, comp: Option<ComponentOpts>) -> Result<Compiled, Vec<Diag>> {
    let mut errs: Vec<Diag> = Vec::new();
    let mut sources: Vec<ModSource> = Vec::new();

    // parse user files first: component builds need their WIT imports
    let mut user_mods = Vec::new();
    for f in files {
        match parse_file(sm, f.display.clone(), f.text.clone()) {
            Ok(m) => user_mods.push((f, m)),
            Err(e) => errs.extend(e),
        }
    }
    if !errs.is_empty() {
        return Err(errs);
    }

    // component setup: world, bindings for imported interfaces, platform layer
    let mut wit_ctx: Option<crate::wit::Wit> = None;
    let mut binding_sources: Vec<(String, String)> = Vec::new();
    let mut platform_imports = String::new();
    let mut platform_text = String::new();
    if let Some(c) = &comp {
        let (base, _) = crate::wit::Wit::new(c.pkg_dir.as_deref()).map_err(internal_user)?;
        let mut extra_names: Vec<String> = Vec::new();
        for (_, m) in &user_mods {
            for imp in &m.imports {
                if let Some(w) = &imp.wit {
                    match crate::wit::Wit::find_interface(&base, w) {
                        Some(_) => extra_names.push(w.clone()),
                        None => errs.push(Diag::new(imp.span, format!("cannot find WIT interface `{w}`")).note("the interface must be part of the bundled WASI 0.3 packages or of the package's wit/ directory")),
                    }
                }
            }
        }
        if !errs.is_empty() {
            return Err(errs);
        }
        // workers need the puddi:runtime worker interfaces from the host
        let uses_workers = target == "wasi" && user_mods.iter().any(|(_, m)| m.imports.iter().any(imports_worker));
        let mut plat: Vec<&str> = if target == "wasi" { WASI_PLATFORM.to_vec() } else { Vec::new() };
        if uses_workers {
            plat.extend(["puddi:runtime/workers", "puddi:runtime/port"]);
        }
        extra_names.extend(plat.iter().map(|s| s.to_string()));
        let extra_exports: Vec<String> = if uses_workers { vec!["puddi:runtime/worker-main".into()] } else { Vec::new() };
        let w = crate::wit::Wit::load(c.pkg_dir.as_deref(), &c.world, &extra_names, &extra_exports).map_err(internal_user)?;
        let extra: Vec<wit_parser::InterfaceId> = extra_names.iter().filter_map(|n| crate::wit::Wit::find_interface(&w.resolve, n)).collect();
        // generate binding modules for the needed interfaces and everything they use
        let mut todo: Vec<wit_parser::InterfaceId> = extra.clone();
        let mut done = std::collections::BTreeSet::new();
        while let Some(i) = todo.pop() {
            if !done.insert(i) {
                continue;
            }
            let mut g = crate::wit::Gen::new(&w, Some(i));
            let text = g.interface_module(i).map_err(internal_user)?;
            for dep in g.imports.keys() {
                todo.push(*dep);
            }
            binding_sources.push((crate::wit::module_path(&w.resolve, i), text));
        }
        if target == "wasi" {
            for p in &plat {
                if let Some(i) = crate::wit::Wit::find_interface(&w.resolve, p) {
                    let mp = crate::wit::module_path(&w.resolve, i);
                    let alias = format!("__w_{}", mp.trim_start_matches("std.__wit.").replace('.', "_"));
                    platform_imports.push_str(&format!("import {mp} as {alias}\n"));
                }
            }
            platform_text = format!("{SOFT}\n{PLATFORM_WASI}\n{}", if uses_workers { PLATFORM_WASI_WORKERS } else { PLATFORM_WASI_NO_WORKERS });
        } else {
            platform_text = format!("{SOFT}\n{PLATFORM_PLUGIN}");
        }
        wit_ctx = Some(w);
    }

    // prelude
    let prelude_text = format!("{platform_imports}{PRELUDE}\n{platform_text}\n// ---- generated numeric prelude ----\n{}", crate::numeric::generate());
    match parse_file(sm, "<prelude>".into(), prelude_text) {
        Ok(m) => sources.push(ModSource { path: "prelude".into(), pkg: Pkg::Prelude, ast: m, prefix: String::new(), aliases: Default::default() }),
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
                sources.push(ModSource { path: path.to_string(), pkg: Pkg::Std, ast: m, prefix: "std".into(), aliases: Default::default() });
            }
            Err(e) => errs.extend(e),
        }
    }
    // generated WIT bindings
    for (path, text) in binding_sources {
        match parse_file(sm, format!("<WIT bindings {path}>"), text) {
            Ok(mut m) => {
                if let Err(e) = with_derives(sm, &mut m, &path) {
                    errs.extend(e);
                }
                sources.push(ModSource { path, pkg: Pkg::Std, ast: m, prefix: "std".into(), aliases: Default::default() });
            }
            Err(e) => errs.extend(e),
        }
    }
    // user
    let mut has_main = false;
    let mut user_exports: Vec<String> = Vec::new();
    for (f, m) in user_mods {
        match Ok::<ast::Module, Vec<Diag>>(m) {
            Ok(mut m) => {
                if f.pkg == Pkg::User && f.module == root {
                    for it in &m.items {
                        if let ast::ItemKind::Fn(fd) = &it.kind {
                            if it.vis == ast::Vis::Export {
                                user_exports.push(fd.name.name.clone());
                            }
                        }
                    }
                }
                if let Err(e) = with_derives(sm, &mut m, &f.module) {
                    errs.extend(e);
                }
                if f.module == root && f.pkg == Pkg::User && mode != Mode::Test {
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
                if f.pkg == Pkg::User && f.module == root {
                    if let Some(w) = &wit_ctx {
                        match export_wrappers(w, target, has_main, &user_exports) {
                            Ok(src) => match parse_file(sm, "<component exports>".into(), src) {
                                Ok(e) => {
                                    m.imports.extend(e.imports);
                                    m.items.extend(e.items);
                                }
                                Err(e) => errs.extend(e),
                            },
                            Err(e) => errs.push(Diag::new(crate::diag::Span::new(u32::MAX, 0, 0), e)),
                        }
                    }
                }
                sources.push(ModSource { path: f.module.clone(), pkg: f.pkg, ast: m, prefix: f.prefix.clone(), aliases: f.aliases.clone() });
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
    let mut platform_fns = Vec::new();
    if wit_ctx.is_some() {
        for name in PLATFORM_FNS {
            match prog.lookup(prog.prelude, name) {
                Some(program::Def::Fn(fid)) => platform_fns.push(g.request(FnInst { fn_id: fid, targs: Vec::new() })),
                _ => return Err(internal(format!("internal error: platform function `{name}` is missing"))),
            }
        }
    }
    for (fid, f) in prog.fns.iter().enumerate() {
        if f.pkg != Pkg::User {
            // runtime entry points the JavaScript host calls (`@export` in the prelude and std)
            if mode != Mode::Check && wit_ctx.is_none() && f.export.is_some() && f.generics.is_empty() && matches!(f.owner, FnOwner::Free) {
                g.request(FnInst { fn_id: fid, targs: Vec::new() });
            }
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
        return Ok(Compiled { wasm: Vec::new(), component: None, wat: String::new(), tests: Vec::new(), warnings, has_main });
    }
    let plat = if wit_ctx.is_some() { Some(platform_fns.as_slice()) } else { None };
    let out = codegen::generate(&prog, &g, if plat.is_some() { None } else { entry }, &tests, plat).map_err(internal)?;
    let component = match &wit_ctx {
        Some(w) => Some(w.encode(&out.wasm).map_err(|e| internal(format!("internal error: component encoding failed: {e}")))?),
        None => None,
    };
    Ok(Compiled { wasm: out.wasm, component, wat: out.wat, tests: out.tests, warnings, has_main })
}

fn internal_user(m: String) -> Vec<Diag> {
    vec![Diag::new(crate::diag::Span::new(u32::MAX, 0, 0), m)]
}


/// Export wrappers for the world's exports.
fn export_wrappers(w: &crate::wit::Wit, target: &str, has_main: bool, user_exports: &[String]) -> Result<String, String> {
    let mut g = crate::wit::Gen::new(w, None);
    let mut items = String::new();
    let mut missing = Vec::new();
    for (k, (export, f, pname)) in crate::wit::world_exports(w).into_iter().enumerate() {
        let target_fn = if export.starts_with("puddi:runtime/worker-main@") {
            "__plat_worker_start".to_string()
        } else if target == "wasi" && export.starts_with("wasi:cli/run@") {
            if !has_main {
                missing.push("`fn main()` (the world exports `wasi:cli/run`)".to_string());
                continue;
            }
            items.push_str("fn __wasi_run() -> Result<(), ()> {\n    let code = __entry()\n    if code == 0 { return Ok(()) }\n    __plat_host(12, \"{code}\")\n    Err(())\n}\n\n");
            "__wasi_run".to_string()
        } else if user_exports.contains(&pname) {
            pname.clone()
        } else {
            missing.push(format!("`export fn {pname}` for `{export}`"));
            continue;
        };
        items.push_str(&g.export_fn(&f, &export, &target_fn, k));
    }
    if !missing.is_empty() {
        return Err(format!("the target world requires exports that the program does not provide: {}", missing.join(", ")));
    }
    let mut head = String::new();
    for (iid, alias) in &g.imports {
        head.push_str(&format!("import {} as {alias}\n", crate::wit::module_path(&w.resolve, *iid)));
    }
    Ok(head + &items)
}

/// Whether an import brings in `std.worker`.
fn imports_worker(i: &ast::Import) -> bool {
    let p: Vec<&str> = i.path.iter().map(|x| x.name.as_str()).collect();
    p == ["std", "worker"] || (p == ["std"] && i.names.as_ref().map(|ns| ns.iter().any(|n| n.name == "worker")).unwrap_or(false))
}
