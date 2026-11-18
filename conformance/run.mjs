#!/usr/bin/env node
// Puddi conformance suite runner.
//
//   node conformance/run.mjs [--pud <path>] [--jobs <n>] [--filter <text>]
//                            [--coverage] [--strict-coverage] [--json <file>] [--list]
//
// Every case under conformance/cases is a `.pud` file, or a package directory
// with a `pud.toml` whose header lives in `src/main.pud` (or `src/lib.pud`).
// The header is the leading block of `//` comments; see conformance/README.md.

import { spawn } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { cpus } from "node:os";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const casesDir = join(here, "cases");
const exe = process.platform === "win32" ? ".exe" : "";

function parseArgs(argv) {
  const o = { jobs: Math.max(1, Math.min(cpus().length, 8)), filter: [], coverage: false, strict: false, json: null, list: false, pud: null, timeout: 120 };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const val = () => {
      if (i + 1 >= argv.length) die(`${a} needs a value`);
      return argv[++i];
    };
    if (a === "--pud") o.pud = val();
    else if (a === "--jobs" || a === "-j") o.jobs = Math.max(1, parseInt(val(), 10) || 1);
    else if (a === "--filter") o.filter.push(val());
    else if (a === "--coverage") o.coverage = true;
    else if (a === "--strict-coverage") o.coverage = o.strict = true;
    else if (a === "--json") o.json = val();
    else if (a === "--timeout") o.timeout = parseInt(val(), 10) || o.timeout;
    else if (a === "--list") o.list = true;
    else if (a === "-h" || a === "--help") {
      console.log(readFileSync(fileURLToPath(import.meta.url), "utf8").split("\n").slice(2, 6).map((l) => l.replace(/^\/\/ ?/, "")).join("\n"));
      process.exit(0);
    } else die(`unknown option ${a}`);
  }
  return o;
}

function die(msg) {
  console.error(`conformance: ${msg}`);
  process.exit(2);
}

function findPud(explicit) {
  const given = explicit ?? process.env.PUD;
  if (given) {
    if (!/[\\/]/.test(given)) return given; // a bare name is looked up on PATH
    const full = resolve(given);
    return exe && !existsSync(full) && existsSync(full + exe) ? full + exe : full;
  }
  for (const p of ["target/release/pud", "target/debug/pud"]) {
    const full = join(root, p + exe);
    if (existsSync(full)) return full;
  }
  return "pud";
}

// ---- discovery -----------------------------------------------------------

function discover(dir) {
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) {
      if (name === "target" || name.startsWith("_")) continue; // `_name`: fixtures used by other cases
      if (existsSync(join(p, "pud.toml"))) {
        const main = ["src/main.pud", "src/lib.pud"].map((f) => join(p, f)).find(existsSync);
        if (!main) die(`${relative(root, p)}: package case without src/main.pud or src/lib.pud`);
        out.push({ path: p, headerFile: main, pkg: true });
      } else out.push(...discover(p));
    } else if (name.endsWith(".pud")) out.push({ path: p, headerFile: p, pkg: false });
  }
  return out;
}

const MODES = { run: "run", test: "test", reject: "check", check: "check", build: "build", fmt: "fmt", doc: "doc" };

function parseCase(c) {
  const text = readFileSync(c.headerFile, "utf8").replace(/\r\n/g, "\n");
  const meta = { spec: [], mode: "run", exit: null, env: {}, args: [], progArgs: [], match: null, xfail: null, out: [], diag: [], absent: [] };
  for (const line of text.split("\n")) {
    if (line.startsWith("//>")) meta.out.push(line.slice(3).replace(/^ /, ""));
    else if (line.startsWith("//~")) meta.diag.push(line.slice(3).trim());
    else if (line.startsWith("//!~")) meta.absent.push(line.slice(4).trim());
  }
  for (const line of text.split("\n")) {
    if (!line.startsWith("//")) break;
    const m = /^\/\/ ([a-z-]+):\s*(.*)$/.exec(line);
    if (!m) continue;
    const [, key, value] = m;
    switch (key) {
      case "spec": meta.spec.push(...value.split(/[\s,]+/).filter(Boolean).map((s) => s.replace(/^§/, ""))); break;
      case "mode": if (!(value in MODES)) die(`${rel(c)}: unknown mode ${value}`); meta.mode = value; break;
      case "exit": meta.exit = parseInt(value, 10); break;
      case "host": meta.env.PUD_HOST = value; break;
      case "env": { const i = value.indexOf("="); meta.env[value.slice(0, i)] = value.slice(i + 1); break; }
      case "args": meta.args.push(...value.split(/\s+/).filter(Boolean)); break;
      case "prog-args": meta.progArgs.push(...value.split(/\s+/).filter(Boolean)); break;
      case "match": if (value !== "exact" && value !== "contains") die(`${rel(c)}: match must be exact or contains`); meta.match = value; break;
      case "xfail": meta.xfail = value || "known failure"; break;
      default: die(`${rel(c)}: unknown header key ${key}`);
    }
  }
  if (meta.spec.length === 0) die(`${rel(c)}: missing \`// spec:\` header`);
  if (meta.mode === "reject" && meta.diag.length === 0) die(`${rel(c)}: reject case needs at least one \`//~\` diagnostic`);
  meta.match ??= meta.mode === "run" ? "exact" : "contains";
  return { ...c, ...meta };
}

const rel = (c) => relative(root, c.path).split(sep).join("/");

// ---- execution -----------------------------------------------------------

function exec(pud, args, env, cwd, timeoutSec) {
  return new Promise((done) => {
    const child = spawn(pud, args, { cwd, env: { ...process.env, ...env }, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "", stderr = "", timedOut = false;
    child.stdout.on("data", (d) => (stdout += d));
    child.stderr.on("data", (d) => (stderr += d));
    const timer = setTimeout(() => { timedOut = true; child.kill("SIGKILL"); }, timeoutSec * 1000);
    child.on("error", (e) => { clearTimeout(timer); done({ code: -1, stdout, stderr: stderr + String(e), timedOut }); });
    child.on("close", (code) => { clearTimeout(timer); done({ code: code ?? -1, stdout, stderr, timedOut }); });
  });
}

const norm = (s) => s.replace(/\r\n/g, "\n");

async function runCase(pud, c, timeout) {
  const args = [MODES[c.mode], c.path, ...c.args];
  if (c.progArgs.length) args.push("--", ...c.progArgs);
  const r = await exec(pud, args, c.env, c.pkg ? c.path : dirname(c.path), timeout);
  const stdout = norm(r.stdout), stderr = norm(r.stderr);
  const all = stdout + stderr;
  const problems = [];
  if (r.timedOut) problems.push(`timed out after ${timeout}s`);
  if (c.mode === "reject") {
    if (r.code === 0) problems.push("compiled, but the program must be rejected");
    for (const d of c.diag) if (!stderr.includes(d)) problems.push(`missing diagnostic: ${d}`);
  } else {
    const want = c.exit ?? (c.mode === "run" || c.mode === "test" ? 0 : 0);
    if (r.code !== want) problems.push(`exit code ${r.code}, expected ${want}`);
    if (c.out.length) {
      if (c.match === "exact") {
        const expected = c.out.join("\n") + "\n";
        if (all !== expected) problems.push(`output differs\n${diff(expected, all)}`);
      } else {
        for (const l of c.out) if (!all.includes(l)) problems.push(`missing output line: ${l}`);
      }
    }
    for (const d of c.diag) if (!stderr.includes(d)) problems.push(`missing diagnostic: ${d}`);
  }
  for (const d of c.absent) if (all.includes(d)) problems.push(`unexpected output: ${d}`);
  return { problems, output: all };
}

function diff(want, got) {
  const a = want.split("\n"), b = got.split("\n");
  const lines = [];
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    if (a[i] === b[i]) continue;
    if (a[i] !== undefined) lines.push(`  line ${i + 1} expected: ${JSON.stringify(a[i])}`);
    if (b[i] !== undefined) lines.push(`  line ${i + 1}      got: ${JSON.stringify(b[i])}`);
    if (lines.length >= 12) { lines.push("  ..."); break; }
  }
  return lines.join("\n");
}

async function pool(items, n, fn) {
  const results = new Array(items.length);
  let next = 0;
  await Promise.all(Array.from({ length: Math.min(n, items.length) }, async () => {
    while (next < items.length) {
      const i = next++;
      results[i] = await fn(items[i], i);
    }
  }));
  return results;
}

// ---- coverage ------------------------------------------------------------

// Normative sections are the numbered subsections of chapters 2 to 10.
function specSections() {
  const spec = readFileSync(join(root, "SPEC.md"), "utf8");
  const out = [];
  for (const m of spec.matchAll(/^### (\d+)\.(\d+) (.+)$/gm)) {
    const ch = parseInt(m[1], 10);
    if (ch >= 2 && ch <= 10) out.push({ id: `${m[1]}.${m[2]}`, title: m[3].trim() });
  }
  return out;
}

// ---- main ----------------------------------------------------------------

const opts = parseArgs(process.argv.slice(2));
const pud = findPud(opts.pud);
let cases = discover(casesDir).map(parseCase);
const sections = specSections();
const known = new Set(sections.map((s) => s.id));
for (const c of cases) for (const s of c.spec) if (!known.has(s)) die(`${rel(c)}: \`spec: ${s}\` is not a section of SPEC.md`);

if (opts.filter.length) cases = cases.filter((c) => opts.filter.some((f) => rel(c).includes(f) || c.spec.includes(f)));
if (opts.list) {
  for (const c of cases) console.log(`${c.spec.join(",").padEnd(12)} ${c.mode.padEnd(7)} ${rel(c)}${c.xfail ? "  (xfail)" : ""}`);
  process.exit(0);
}

const version = await exec(pud, ["version"], {}, root, 30);
if (version.code !== 0) die(`cannot run \`${pud} version\`: ${version.stderr.trim() || "exit " + version.code}`);
console.log(`# ${version.stdout.trim()} (${pud})`);
console.log(`# ${cases.length} cases, ${opts.jobs} jobs`);

const t0 = Date.now();
const results = await pool(cases, opts.jobs, async (c) => {
  const t = Date.now();
  const r = await runCase(pud, c, opts.timeout);
  const failed = r.problems.length > 0;
  let status;
  if (c.xfail) status = failed ? "xfail" : "xpass";
  else status = failed ? "fail" : "pass";
  const ms = Date.now() - t;
  const tag = { pass: "ok     ", fail: "FAIL   ", xfail: "xfail  ", xpass: "XPASS  " }[status];
  console.log(`${tag} ${rel(c)} [${c.spec.join(", ")}] ${ms}ms`);
  if (status === "fail") {
    for (const p of r.problems) console.log("    " + p.replace(/\n/g, "\n    "));
    const tail = r.output.trimEnd().split("\n").slice(-15).join("\n      ");
    if (tail) console.log("    output:\n      " + tail);
  }
  if (status === "xpass") console.log(`    passes now; remove \`// xfail: ${c.xfail}\``);
  return { case: rel(c), spec: c.spec, mode: c.mode, status, ms, problems: r.problems, xfail: c.xfail };
});

const count = (s) => results.filter((r) => r.status === s).length;
const [pass, fail, xfail, xpass] = ["pass", "fail", "xfail", "xpass"].map(count);

let uncovered = [];
if (opts.coverage) {
  const all = discover(casesDir).map(parseCase);
  console.log("\n# coverage by SPEC.md section");
  for (const s of sections) {
    const cs = all.filter((c) => c.spec.includes(s.id));
    const ok = cs.filter((c) => !c.xfail).length;
    if (ok === 0) uncovered.push(s);
    console.log(`  ${s.id.padEnd(5)} ${String(ok).padStart(3)} case(s)${cs.length > ok ? ` + ${cs.length - ok} xfail` : ""}  ${s.title}`);
  }
  console.log(`# ${sections.length - uncovered.length}/${sections.length} normative sections covered`);
  for (const s of uncovered) console.log(`#   uncovered: ${s.id} ${s.title}`);
}

if (opts.json) {
  writeFileSync(opts.json, JSON.stringify({ pud: version.stdout.trim(), pass, fail, xfail, xpass, uncovered: uncovered.map((s) => s.id), results }, null, 2));
}

console.log(`\n# ${pass} passed, ${fail} failed, ${xfail} expected failures, ${xpass} unexpected passes (${((Date.now() - t0) / 1000).toFixed(1)}s)`);
const ok = fail === 0 && xpass === 0 && !(opts.strict && uncovered.length);
if (!ok && opts.strict && uncovered.length) console.log("# strict coverage: every normative section needs a passing case");
process.exit(ok ? 0 : 1);
