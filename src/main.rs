//! `pud`: the Puddi toolchain.

#![allow(dead_code)]

mod ast;
mod codegen;
mod derive;
mod diag;
mod driver;
mod elab;
mod exhaust;
mod ir;
mod lexer;
mod numeric;
mod parser;
mod program;
mod tools;
mod types;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use diag::SourceMap;
use driver::{Mode, UserFile};

const RUNTIME: &str = include_str!("runtime.mjs");
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(serde::Deserialize)]
struct Manifest {
    package: PackageInfo,
    #[serde(default)]
    deps: toml::Table,
}

#[derive(serde::Deserialize)]
struct PackageInfo {
    name: String,
    #[allow(dead_code)]
    version: Option<String>,
    #[allow(dead_code)]
    puddi: Option<String>,
}

struct Project {
    name: String,
    /// Package directory (None for single-file mode).
    dir: Option<PathBuf>,
    files: Vec<UserFile>,
    root: String,
}

fn usage() -> &'static str {
    "pud - the Puddi toolchain

usage:
  pud new <name>                 create a package
  pud build [path] [--target t] [--emit-wat] [-o dir]
                                 build for browser, wasi (default) or plugin
  pud run [path|file.pud] [-- args...]
                                 build and run with Node.js
  pud test [path|file.pud] [filter]
                                 run test blocks
  pud check [path|file.pud]      type-check without generating code
  pud fmt [files...] [--check]   format sources in the single official style
  pud doc [path]                 write Markdown docs for public items to target/doc
  pud version                    print the version
"
}

fn fail(msg: impl AsRef<str>) -> ExitCode {
    eprintln!("error: {}", msg.as_ref());
    ExitCode::from(1)
}

fn collect_pud(dir: &Path, base: &Path, out: &mut Vec<(String, PathBuf)>) -> std::io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.path());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            collect_pud(&p, base, out)?;
        } else if p.extension().map(|x| x == "pud").unwrap_or(false) {
            let rel = p.strip_prefix(base).unwrap().with_extension("");
            let module = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>().join(".");
            out.push((module, p));
        }
    }
    Ok(())
}

fn find_manifest(start: &Path) -> Option<PathBuf> {
    let mut d = Some(start.to_path_buf());
    while let Some(dir) = d {
        if dir.join("pud.toml").is_file() {
            return Some(dir);
        }
        d = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

fn load_project(path: Option<&str>) -> Result<Project, String> {
    let p = PathBuf::from(path.unwrap_or("."));
    if p.is_file() {
        if p.file_name().map(|n| n == "pud.toml").unwrap_or(false) {
            return load_project(Some(p.parent().unwrap().to_str().unwrap_or(".")));
        }
        let text = std::fs::read_to_string(&p).map_err(|e| format!("cannot read {}: {e}", p.display()))?;
        let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "main".into());
        return Ok(Project { name, dir: None, files: vec![UserFile { module: "main".into(), display: p.display().to_string(), text }], root: "main".into() });
    }
    let dir = find_manifest(&p).ok_or_else(|| format!("could not find pud.toml in {} or any parent directory", p.display()))?;
    let mtext = std::fs::read_to_string(dir.join("pud.toml")).map_err(|e| format!("cannot read pud.toml: {e}"))?;
    let manifest: Manifest = toml::from_str(&mtext).map_err(|e| format!("invalid pud.toml: {e}"))?;
    if !manifest.deps.is_empty() {
        return Err("dependencies in [deps] are not supported by this compiler yet".into());
    }
    let src = dir.join("src");
    let mut found = Vec::new();
    collect_pud(&src, &src, &mut found).map_err(|e| format!("cannot read {}: {e}", src.display()))?;
    let root = if found.iter().any(|(m, _)| m == "main") {
        "main"
    } else if found.iter().any(|(m, _)| m == "lib") {
        "lib"
    } else {
        return Err("package has neither src/main.pud nor src/lib.pud".into());
    };
    let mut files = Vec::new();
    for (module, path) in found {
        let text = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let display = path.strip_prefix(&dir).unwrap_or(&path).display().to_string();
        files.push(UserFile { module, display, text });
    }
    Ok(Project { name: manifest.package.name, dir: Some(dir), files, root: root.into() })
}

fn report(sm: &SourceMap, diags: &[diag::Diag]) {
    for d in diags {
        eprint!("{}", sm.render(d));
    }
}

struct BuildResult {
    out_dir: PathBuf,
    wasm_path: PathBuf,
}

fn build(proj: Project, target: &str, mode: Mode, out: Option<PathBuf>, emit_wat: bool) -> Result<BuildResult, ExitCode> {
    let mut sm = SourceMap::default();
    let name = proj.name.clone();
    let compiled = match driver::compile(&mut sm, proj.files, &proj.root, target, mode) {
        Ok(c) => c,
        Err(errs) => {
            report(&sm, &errs);
            let n = errs.iter().filter(|d| !d.warning).count();
            eprintln!("error: could not compile `{name}` due to {n} previous error{}", if n == 1 { "" } else { "s" });
            return Err(ExitCode::from(1));
        }
    };
    report(&sm, &compiled.warnings);
    if mode == Mode::Check {
        return Ok(BuildResult { out_dir: PathBuf::new(), wasm_path: PathBuf::new() });
    }
    let out_dir = match out {
        Some(o) => o,
        None => match &proj.dir {
            Some(d) => d.join("target").join(if mode == Mode::Test { "test" } else { target }),
            None => std::env::temp_dir().join(format!("puddi-{}-{}", name, std::process::id())),
        },
    };
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!("error: cannot create {}: {e}", out_dir.display());
        return Err(ExitCode::from(1));
    }
    let wasm_path = out_dir.join(format!("{name}.wasm"));
    let write = |p: PathBuf, data: &[u8]| -> Result<(), ExitCode> {
        std::fs::write(&p, data).map_err(|e| {
            eprintln!("error: cannot write {}: {e}", p.display());
            ExitCode::from(1)
        })
    };
    write(wasm_path.clone(), &compiled.wasm)?;
    write(out_dir.join("puddi.mjs"), RUNTIME.as_bytes())?;
    if emit_wat {
        write(out_dir.join(format!("{name}.wat")), compiled.wat.as_bytes())?;
    }
    if mode == Mode::Test {
        let json: Vec<String> = compiled.tests.iter().map(|(n, e)| format!("[{},{}]", json_str(n), json_str(e))).collect();
        write(out_dir.join(format!("{name}.tests.json")), format!("[{}]", json.join(",")).as_bytes())?;
    }
    if target == "browser" && mode == Mode::Build {
        let html = format!(
            "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{name}</title>\n<style>body{{font:14px/1.5 ui-monospace,monospace;margin:16px;background:#fafafa;color:#222}}pre{{white-space:pre-wrap}}.err{{color:#b00}}</style>\n</head>\n<body>\n<pre id=\"out\"></pre>\n<script type=\"module\">\nimport {{ load }} from \"./puddi.mjs\";\nconst out = document.getElementById(\"out\");\nconst app = await load(\"./{name}.wasm\", {{ output(fd, line) {{ const s = document.createElement(\"span\"); if (fd === 2) s.className = \"err\"; s.textContent = line + \"\\n\"; out.append(s); (fd === 2 ? console.error : console.log)(line); }} }});\napp.run();\n</script>\n</body>\n</html>\n"
        );
        write(out_dir.join("index.html"), html.as_bytes())?;
    }
    Ok(BuildResult { out_dir, wasm_path })
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn node_run(res: &BuildResult, cmd: &str, args: &[String]) -> ExitCode {
    let status = Command::new("node").arg(res.out_dir.join("puddi.mjs")).arg(cmd).arg(&res.wasm_path).args(args).status();
    match status {
        Ok(s) => match s.code() {
            Some(c) => ExitCode::from(c.clamp(0, 255) as u8),
            None => ExitCode::from(1),
        },
        Err(e) => fail(format!("could not start `node` ({e}); Node.js 22 or newer is needed to run Puddi programs")),
    }
}

fn new_package(name: &str) -> ExitCode {
    let dir = PathBuf::from(name);
    if dir.exists() {
        return fail(format!("`{name}` already exists"));
    }
    let pkg = dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| name.to_string());
    let res = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(dir.join("src"))?;
        std::fs::write(dir.join("pud.toml"), format!("[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\npuddi = \"0.1\"\n\n[deps]\n"))?;
        std::fs::write(
            dir.join("src/main.pud"),
            "fn greet(name: Str) -> Str {\n    \"Hello, {name}!\"\n}\n\nfn main() {\n    print(greet(\"Puddi\"))\n}\n\ntest \"greet says hello\" {\n    assert_eq(greet(\"you\"), \"Hello, you!\")\n}\n",
        )?;
        std::fs::write(dir.join(".gitignore"), "target/\n")?;
        Ok(())
    })();
    match res {
        Ok(()) => {
            println!("created package `{pkg}` in {}", dir.display());
            ExitCode::SUCCESS
        }
        Err(e) => fail(format!("cannot create package: {e}")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().cloned() else {
        print!("{}", usage());
        return ExitCode::SUCCESS;
    };
    let rest = &args[1..];
    // split off `-- args`
    let (opts, prog_args): (&[String], &[String]) = match rest.iter().position(|a| a == "--") {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, &[]),
    };
    let mut target = "wasi".to_string();
    let mut emit_wat = false;
    let mut check_only = false;
    let mut out = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < opts.len() {
        match opts[i].as_str() {
            "--target" | "-t" => {
                i += 1;
                match opts.get(i) {
                    Some(t) if ["browser", "wasi", "plugin"].contains(&t.as_str()) => target = t.clone(),
                    Some(t) => return fail(format!("unknown target `{t}`; expected browser, wasi or plugin")),
                    None => return fail("--target needs a value"),
                }
            }
            "--emit-wat" => emit_wat = true,
            "--check" => check_only = true,
            "-o" | "--out" => {
                i += 1;
                match opts.get(i) {
                    Some(o) => out = Some(PathBuf::from(o)),
                    None => return fail("-o needs a directory"),
                }
            }
            s if s.starts_with("--target=") => target = s["--target=".len()..].to_string(),
            s if s.starts_with('-') && s.len() > 1 => return fail(format!("unknown option `{s}`")),
            s => positional.push(s.to_string()),
        }
        i += 1;
    }
    match cmd.as_str() {
        "new" => match positional.first() {
            Some(n) => new_package(n),
            None => fail("usage: pud new <name>"),
        },
        "build" | "run" | "test" | "check" => {
            let proj = match load_project(positional.first().map(|s| s.as_str())) {
                Ok(p) => p,
                Err(e) => return fail(e),
            };
            let mode = match cmd.as_str() {
                "test" => Mode::Test,
                "check" => Mode::Check,
                _ => Mode::Build,
            };
            let has_dir = proj.dir.is_some();
            let res = match build(proj, &target, mode, out, emit_wat) {
                Ok(r) => r,
                Err(c) => return c,
            };
            match cmd.as_str() {
                "build" => {
                    println!("built {}", res.wasm_path.display());
                    ExitCode::SUCCESS
                }
                "check" => ExitCode::SUCCESS,
                "run" => {
                    let code = node_run(&res, "run", prog_args);
                    if !has_dir {
                        let _ = std::fs::remove_dir_all(&res.out_dir);
                    }
                    code
                }
                _ => {
                    let filter: Vec<String> = positional.get(1).cloned().into_iter().collect();
                    let code = node_run(&res, "test", &filter);
                    if !has_dir {
                        let _ = std::fs::remove_dir_all(&res.out_dir);
                    }
                    code
                }
            }
        }
        "fmt" => run_fmt(&positional, check_only),
        "doc" => run_doc(positional.first().map(|s| s.as_str())),
        "add" => fail("`pud add` is not available: package dependencies are not supported by this version of the toolchain"),
        "version" | "--version" | "-V" => {
            println!("pud {VERSION}");
            ExitCode::SUCCESS
        }
        "help" | "--help" | "-h" => {
            print!("{}", usage());
            ExitCode::SUCCESS
        }
        other => fail(format!("unknown command `{other}`\n\n{}", usage())),
    }
}

fn fmt_targets(positional: &[String]) -> Result<Vec<PathBuf>, String> {
    if !positional.is_empty() && positional.iter().all(|p| p.ends_with(".pud")) {
        return Ok(positional.iter().map(PathBuf::from).collect());
    }
    let start = PathBuf::from(positional.first().map(|s| s.as_str()).unwrap_or("."));
    let dir = find_manifest(&start).ok_or_else(|| "could not find pud.toml; pass .pud files explicitly".to_string())?;
    let src = dir.join("src");
    let mut found = Vec::new();
    collect_pud(&src, &src, &mut found).map_err(|e| e.to_string())?;
    Ok(found.into_iter().map(|(_, p)| p).collect())
}

fn run_fmt(positional: &[String], check: bool) -> ExitCode {
    let files = match fmt_targets(positional) {
        Ok(f) => f,
        Err(e) => return fail(e),
    };
    let mut changed = 0;
    for f in &files {
        let text = match std::fs::read_to_string(f) {
            Ok(t) => t,
            Err(e) => return fail(format!("cannot read {}: {e}", f.display())),
        };
        let formatted = tools::format(&text);
        if formatted != text {
            changed += 1;
            if check {
                println!("would reformat {}", f.display());
            } else if let Err(e) = std::fs::write(f, formatted) {
                return fail(format!("cannot write {}: {e}", f.display()));
            } else {
                println!("formatted {}", f.display());
            }
        }
    }
    if check && changed > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS }
}

fn run_doc(path: Option<&str>) -> ExitCode {
    let proj = match load_project(path) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    let mut page = format!("# {}\n\n", proj.name);
    for f in &proj.files {
        if let Some(d) = tools::doc(&f.module, &f.text) {
            page.push_str(&d);
        }
    }
    let out = match &proj.dir {
        Some(d) => d.join("target").join("doc"),
        None => PathBuf::from("."),
    };
    if let Err(e) = std::fs::create_dir_all(&out) {
        return fail(format!("cannot create {}: {e}", out.display()));
    }
    let file = out.join(format!("{}.md", proj.name));
    match std::fs::write(&file, page) {
        Ok(()) => {
            println!("wrote {}", file.display());
            ExitCode::SUCCESS
        }
        Err(e) => fail(format!("cannot write {}: {e}", file.display())),
    }
}
