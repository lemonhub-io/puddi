//! Package dependencies: git repositories with semver tags, resolved with
//! minimal version selection, plus local path dependencies. `pud.lock`
//! records the exact commit of every git dependency.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub fn parse(s: &str) -> Option<Version> {
        let s = s.trim().trim_start_matches('v');
        let core = s.split(['-', '+']).next()?;
        let mut it = core.split('.');
        let major = it.next()?.parse().ok()?;
        let minor = it.next().map(|x| x.parse().ok()).unwrap_or(Some(0))?;
        let patch = it.next().map(|x| x.parse().ok()).unwrap_or(Some(0))?;
        if it.next().is_some() {
            return None;
        }
        Some(Version { major, minor, patch })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Where a dependency comes from.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    /// Git repository URL and major version (different majors are distinct packages).
    Git(String, u64),
    /// Canonical local directory.
    Path(PathBuf),
}

#[derive(Clone, Debug)]
pub struct DepSpec {
    pub source: Source,
    /// Minimum version for git dependencies.
    pub min: Option<Version>,
}

/// A resolved package ready to compile.
pub struct ResolvedPkg {
    pub name: String,
    /// Module path prefix used for this package's modules.
    pub prefix: String,
    pub source: Source,
    pub version: Option<Version>,
    pub commit: Option<String>,
    /// (module path relative to the package, display path, text)
    pub files: Vec<(String, String, String)>,
    /// Import name -> prefix of the package it refers to.
    pub aliases: HashMap<String, String>,
}

pub struct Resolution {
    /// Dependencies of the root package, import name -> prefix.
    pub root_aliases: HashMap<String, String>,
    pub packages: Vec<ResolvedPkg>,
}

/// Turn `github.com/user/repo` or a URL into a fetchable git URL.
pub fn git_url(s: &str) -> String {
    if s.contains("://") || s.starts_with('/') || s.starts_with("git@") {
        s.to_string()
    } else {
        format!("https://{s}")
    }
}

/// Parse one `[deps]` entry. `base` is the directory of the manifest.
pub fn parse_spec(name: &str, v: &toml::Value, base: &Path) -> Result<DepSpec, String> {
    match v {
        toml::Value::String(s) => {
            let (repo, ver) = match s.rsplit_once('@') {
                Some((r, v)) if !r.ends_with(':') && Version::parse(v).is_some() => (r, Some(v)),
                _ => (s.as_str(), None),
            };
            let Some(ver) = ver else {
                return Err(format!("dependency `{name}` needs a version, as in \"{repo}@1.0\""));
            };
            let v = Version::parse(ver).unwrap();
            Ok(DepSpec { source: Source::Git(git_url(repo), v.major), min: Some(v) })
        }
        toml::Value::Table(t) => {
            if let Some(p) = t.get("path").and_then(|p| p.as_str()) {
                let dir = base.join(p);
                let dir = dir.canonicalize().map_err(|e| format!("dependency `{name}`: cannot open {}: {e}", dir.display()))?;
                return Ok(DepSpec { source: Source::Path(dir), min: None });
            }
            let Some(g) = t.get("git").and_then(|g| g.as_str()) else {
                return Err(format!("dependency `{name}` needs `git` or `path`"));
            };
            let Some(v) = t.get("version").and_then(|v| v.as_str()).and_then(Version::parse) else {
                return Err(format!("dependency `{name}` needs a `version`"));
            };
            Ok(DepSpec { source: Source::Git(git_url(g), v.major), min: Some(v) })
        }
        _ => Err(format!("dependency `{name}` must be a string or a table")),
    }
}

fn git(dir: Option<&Path>, args: &[&str]) -> Result<String, String> {
    let mut c = Command::new("git");
    if let Some(d) = dir {
        c.arg("--git-dir").arg(d);
    }
    c.args(args);
    let out = c.output().map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(format!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn cache_root() -> PathBuf {
    if let Ok(h) = std::env::var("PUDDI_HOME") {
        return PathBuf::from(h).join("git");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".cache").join("puddi").join("git")
}

pub struct Fetcher {
    fetched: HashSet<String>,
    tags: HashMap<String, Vec<(Version, String)>>,
}

impl Fetcher {
    pub fn new() -> Fetcher {
        Fetcher { fetched: HashSet::new(), tags: HashMap::new() }
    }

    fn repo_dir(url: &str) -> PathBuf {
        let clean: String = url.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
        cache_root().join(clean)
    }

    /// Clone or update the bare mirror of `url`.
    fn ensure(&mut self, url: &str) -> Result<PathBuf, String> {
        let dir = Self::repo_dir(url);
        if self.fetched.contains(url) {
            return Ok(dir);
        }
        if dir.join("HEAD").exists() {
            git(Some(&dir), &["fetch", "--quiet", "--tags", "--force", "origin"])?;
        } else {
            std::fs::create_dir_all(dir.parent().unwrap()).map_err(|e| e.to_string())?;
            git(None, &["clone", "--quiet", "--bare", url, dir.to_str().unwrap()])?;
        }
        self.fetched.insert(url.to_string());
        Ok(dir)
    }

    /// Semver tags of a repository, sorted ascending: (version, tag name).
    pub fn tags(&mut self, url: &str) -> Result<Vec<(Version, String)>, String> {
        if let Some(t) = self.tags.get(url) {
            return Ok(t.clone());
        }
        let dir = self.ensure(url)?;
        let mut v: Vec<(Version, String)> = git(Some(&dir), &["tag", "-l", "v*"])?
            .lines()
            .filter(|t| !t.contains('-'))
            .filter_map(|t| Version::parse(t).map(|v| (v, t.to_string())))
            .collect();
        v.sort();
        self.tags.insert(url.to_string(), v.clone());
        Ok(v)
    }

    /// The lowest tagged version of the same major that is at least `min`.
    fn lowest(&mut self, url: &str, min: Version) -> Result<(Version, String), String> {
        let tags = self.tags(url)?;
        tags.into_iter()
            .find(|(v, _)| v.major == min.major && *v >= min)
            .ok_or_else(|| format!("{url} has no tag v{}.x.x at or above v{min}", min.major))
    }

    fn commit(&mut self, url: &str, tag: &str) -> Result<String, String> {
        let dir = self.ensure(url)?;
        Ok(git(Some(&dir), &["rev-parse", &format!("{tag}^{{commit}}")])?.trim().to_string())
    }

    fn read(&mut self, url: &str, rev: &str, path: &str) -> Result<String, String> {
        let dir = self.ensure(url)?;
        git(Some(&dir), &["show", &format!("{rev}:{path}")])
    }

    fn list_src(&mut self, url: &str, rev: &str) -> Result<Vec<String>, String> {
        let dir = self.ensure(url)?;
        Ok(git(Some(&dir), &["ls-tree", "-r", "--name-only", rev, "--", "src"])?.lines().map(|s| s.to_string()).collect())
    }
}

#[derive(serde::Deserialize)]
struct DepManifest {
    package: DepPackage,
    #[serde(default)]
    deps: toml::Table,
}

#[derive(serde::Deserialize)]
struct DepPackage {
    name: String,
}

struct Node {
    name: String,
    deps: Vec<(String, DepSpec)>,
}

/// Read a package manifest at a given source and version.
fn load_node(f: &mut Fetcher, src: &Source, rev: Option<&str>) -> Result<Node, String> {
    let (text, base) = match src {
        Source::Git(url, _) => (f.read(url, rev.unwrap(), "pud.toml")?, PathBuf::from(".")),
        Source::Path(dir) => (std::fs::read_to_string(dir.join("pud.toml")).map_err(|e| format!("cannot read {}: {e}", dir.join("pud.toml").display()))?, dir.clone()),
    };
    let m: DepManifest = toml::from_str(&text).map_err(|e| format!("invalid pud.toml in {}: {e}", show_source(src)))?;
    let mut deps = Vec::new();
    for (k, v) in &m.deps {
        let spec = parse_spec(k, v, &base)?;
        if matches!(src, Source::Git(..)) && matches!(spec.source, Source::Path(_)) {
            return Err(format!("{} uses a path dependency `{k}`; published packages may only depend on git packages", show_source(src)));
        }
        deps.push((k.clone(), spec));
    }
    Ok(Node { name: m.package.name, deps })
}

pub fn show_source(s: &Source) -> String {
    match s {
        Source::Git(u, _) => u.clone(),
        Source::Path(p) => p.display().to_string(),
    }
}

/// Entry of `pud.lock`.
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq)]
pub struct LockEntry {
    pub name: String,
    pub source: String,
    pub version: String,
    pub commit: String,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Debug, PartialEq)]
pub struct LockFile {
    #[serde(default)]
    pub package: Vec<LockEntry>,
}

/// Resolve the dependency graph of the root manifest with minimal version selection.
pub fn resolve(root_deps: &[(String, DepSpec)], lock_path: Option<&Path>, update_lock: bool) -> Result<Resolution, String> {
    let mut f = Fetcher::new();
    let lock: LockFile = match lock_path {
        Some(p) if p.exists() => toml::from_str(&std::fs::read_to_string(p).map_err(|e| e.to_string())?).map_err(|e| format!("invalid pud.lock: {e}"))?,
        _ => LockFile::default(),
    };

    // Walk every (package, version) reachable from the root; each package's
    // selected version is the highest version required anywhere in the graph.
    let mut selected: BTreeMap<Source, (Version, String)> = BTreeMap::new();
    let mut paths: BTreeMap<Source, ()> = BTreeMap::new();
    let mut visited: HashSet<(Source, Option<Version>)> = HashSet::new();
    let mut queue: Vec<DepSpec> = root_deps.iter().map(|(_, s)| s.clone()).collect();
    let mut nodes: HashMap<(Source, Option<Version>), Node> = HashMap::new();
    while let Some(spec) = queue.pop() {
        let (key, rev) = match &spec.source {
            Source::Git(url, _) => {
                let (v, tag) = f.lowest(url, spec.min.unwrap())?;
                match selected.get(&spec.source) {
                    Some((sv, _)) if *sv >= v => {}
                    _ => {
                        selected.insert(spec.source.clone(), (v, tag.clone()));
                    }
                }
                ((spec.source.clone(), Some(v)), Some(tag))
            }
            Source::Path(_) => {
                paths.insert(spec.source.clone(), ());
                ((spec.source.clone(), None), None)
            }
        };
        if !visited.insert(key.clone()) {
            continue;
        }
        let node = load_node(&mut f, &spec.source, rev.as_deref())?;
        for (_, d) in &node.deps {
            queue.push(d.clone());
        }
        nodes.insert(key, node);
    }

    // Final package list: selected git versions plus path packages.
    let mut final_keys: Vec<(Source, Option<Version>, Option<String>)> = Vec::new();
    for (src, (v, tag)) in &selected {
        final_keys.push((src.clone(), Some(*v), Some(tag.clone())));
    }
    for src in paths.keys() {
        final_keys.push((src.clone(), None, None));
    }

    // Prefixes: the root's import name when it names the package directly,
    // otherwise the package name; made unique with a numeric suffix.
    let mut prefix_of: HashMap<Source, String> = HashMap::new();
    let mut used: HashSet<String> = ["main", "lib", "std", "prelude"].iter().map(|s| s.to_string()).collect();
    for (alias, spec) in root_deps {
        if !prefix_of.contains_key(&spec.source) && used.insert(alias.clone()) {
            prefix_of.insert(spec.source.clone(), alias.clone());
        }
    }
    for (src, v, _) in &final_keys {
        if prefix_of.contains_key(src) {
            continue;
        }
        let base = nodes[&(src.clone(), *v)].name.replace('-', "_");
        let mut name = base.clone();
        let mut n = 2;
        while !used.insert(name.clone()) {
            name = format!("{base}_{n}");
            n += 1;
        }
        prefix_of.insert(src.clone(), name);
    }

    let mut new_lock = LockFile::default();
    let mut packages = Vec::new();
    for (src, v, tag) in &final_keys {
        let node = &nodes[&(src.clone(), *v)];
        let mut files = Vec::new();
        let mut commit = None;
        match src {
            Source::Git(url, _) => {
                let tag = tag.as_ref().unwrap();
                let c = f.commit(url, tag)?;
                if let Some(e) = lock.package.iter().find(|e| &e.source == url && e.version == v.unwrap().to_string()) {
                    if e.commit != c {
                        return Err(format!("{url} tag {tag} now points to commit {c}, but pud.lock records {}; the tag was moved", e.commit));
                    }
                }
                for path in f.list_src(url, &c)? {
                    if !path.ends_with(".pud") {
                        continue;
                    }
                    let text = f.read(url, &c, &path)?;
                    let module = path.trim_start_matches("src/").trim_end_matches(".pud").replace('/', ".");
                    files.push((module, format!("{}@{}/{path}", url.trim_start_matches("https://"), v.unwrap()), text));
                }
                new_lock.package.push(LockEntry { name: node.name.clone(), source: url.clone(), version: v.unwrap().to_string(), commit: c.clone() });
                commit = Some(c);
            }
            Source::Path(dir) => {
                let srcdir = dir.join("src");
                collect(&srcdir, &srcdir, &mut files)?;
            }
        }
        if !files.iter().any(|(m, _, _)| m == "lib") {
            return Err(format!("dependency {} has no src/lib.pud", show_source(src)));
        }
        let mut aliases = HashMap::new();
        for (alias, spec) in &node.deps {
            aliases.insert(alias.clone(), prefix_of[&spec.source].clone());
        }
        packages.push(ResolvedPkg { name: node.name.clone(), prefix: prefix_of[src].clone(), source: src.clone(), version: *v, commit, files, aliases });
    }
    let root_aliases = root_deps.iter().map(|(a, s)| (a.clone(), prefix_of[&s.source].clone())).collect();

    if let Some(p) = lock_path {
        if update_lock && (new_lock != lock || !p.exists()) && (!new_lock.package.is_empty() || p.exists()) {
            let text = format!("# Generated by pud. Records the exact commit of every dependency.\n\n{}", toml::to_string(&new_lock).map_err(|e| e.to_string())?);
            std::fs::write(p, text).map_err(|e| format!("cannot write pud.lock: {e}"))?;
        }
    }
    Ok(Resolution { root_aliases, packages })
}

fn collect(dir: &Path, base: &Path, out: &mut Vec<(String, String, String)>) -> Result<(), String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.path());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            collect(&p, base, out)?;
        } else if p.extension().map(|x| x == "pud").unwrap_or(false) {
            let rel = p.strip_prefix(base).unwrap().with_extension("");
            let module = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>().join(".");
            let text = std::fs::read_to_string(&p).map_err(|e| format!("cannot read {}: {e}", p.display()))?;
            out.push((module, p.display().to_string(), text));
        }
    }
    Ok(())
}

/// `pud add`: work out the manifest entry for a dependency argument.
/// Returns (import name, TOML value text).
pub fn add_entry(arg: &str, name: Option<&str>, base: &Path) -> Result<(String, String), String> {
    let local = Path::new(arg);
    if base.join(local).join("pud.toml").is_file() {
        let dir = base.join(local).canonicalize().map_err(|e| e.to_string())?;
        let node = load_node(&mut Fetcher::new(), &Source::Path(dir), None)?;
        let n = name.map(|s| s.to_string()).unwrap_or_else(|| node.name.replace('-', "_"));
        return Ok((n, format!("{{ path = {} }}", toml_str(arg))));
    }
    let (repo, ver) = match arg.rsplit_once('@') {
        Some((r, v)) if Version::parse(v).is_some() => (r.to_string(), Some(Version::parse(v).unwrap())),
        _ => (arg.to_string(), None),
    };
    let url = git_url(&repo);
    let mut f = Fetcher::new();
    let tags = f.tags(&url)?;
    let v = match ver {
        Some(v) => f.lowest(&url, v)?.0,
        None => tags.last().map(|t| t.0).ok_or_else(|| format!("{url} has no semver tags like v1.0.0"))?,
    };
    let tag = tags.iter().find(|t| t.0 == v).map(|t| t.1.clone()).unwrap();
    let node = load_node(&mut f, &Source::Git(url.clone(), v.major), Some(&tag))?;
    let n = name.map(|s| s.to_string()).unwrap_or_else(|| node.name.replace('-', "_"));
    let short = if v.patch == 0 { format!("{}.{}", v.major, v.minor) } else { v.to_string() };
    Ok((n, toml_str(&format!("{repo}@{short}"))))
}

fn toml_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Insert or replace `name = value` in the `[deps]` table of a manifest, keeping the rest of the text.
pub fn edit_manifest(text: &str, name: &str, value: &str) -> String {
    let line = format!("{name} = {value}");
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|l| l.trim() == "[deps]") else {
        let mut out = text.trim_end().to_string();
        out.push_str(&format!("\n\n[deps]\n{line}\n"));
        return out;
    };
    let end = lines[start + 1..].iter().position(|l| l.trim_start().starts_with('[')).map(|i| start + 1 + i).unwrap_or(lines.len());
    let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    if let Some(i) = (start + 1..end).find(|&i| out[i].split('=').next().map(|k| k.trim() == name).unwrap_or(false)) {
        out[i] = line;
    } else {
        let mut at = end;
        while at > start + 1 && out[at - 1].trim().is_empty() {
            at -= 1;
        }
        out.insert(at, line);
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}
