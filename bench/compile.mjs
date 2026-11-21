// Compile-time benchmark: how long the steps of the edit-compile-run loop take with cargo (Rust),
// moon (MoonBit) and pud (Puddi), on hello and on generated programs of about 2,600 and 26,000 lines.
//   node compile.mjs [--reps 3] [--pud path/to/pud] [--only hello,gen200] [--out compile.json]
// Steps run one at a time. "cold" steps start from a clean project; "edit" steps change one line
// of one function and rerun the command on the warm project, as a developer would.
import { spawnSync } from 'node:child_process';
import { mkdirSync, rmSync, writeFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const argv = process.argv.slice(2);
const opt = (k, d) => { const i = argv.indexOf(k); return i < 0 ? d : argv[i + 1]; };
const REPS = +opt('--reps', 3);
const PUD = opt('--pud', 'pud');
const OUT = opt('--out', 'compile.json');
const PROGS = opt('--only', 'hello,gen200,gen2000').split(',');
const WORK = resolve('compile-work');

// ---------------------------------------------------------------- sources

// v (0 or 1) changes one line, so that consecutive builds see an edit
function gen(lang, prog, v) {
  if (prog === 'hello') {
    const s = v ? 'hi!' : 'hi';
    return { rust: `fn main() {\n    println!("${s}");\n}\n`, moon: `fn main {\n  println("${s}")\n}\n`, pud: `fn main() {\n    print("${s}")\n}\n` }[lang];
  }
  const n = +prog.slice(3);
  const lim = (i) => (i === n >> 1 ? 3 + v : 3);
  const out = [];
  if (lang === 'rust') {
    out.push('#![allow(dead_code, unused)]', 'fn pick<T>(c: bool, a: T, b: T) -> T { if c { a } else { b } }', 'fn f0(x: i64) -> i64 { x }');
    for (let i = 1; i <= n; i++) out.push(`
struct P${i} { a: i64, b: i64 }
enum E${i} { A(i64), B }
fn f${i}(x: i64) -> i64 {
    let p = P${i} { a: x, b: x * 2 };
    let e = if x % 2 == 0 { E${i}::A(p.a) } else { E${i}::B };
    let mut s = 0;
    let mut k = 0;
    while k < ${lim(i)} {
        let v = match e {
            E${i}::A(v) => v + k,
            E${i}::B => p.b,
        };
        s += v;
        k += 1;
    }
    let g = |y: i64| y + p.a;
    pick(s > 0, g(s), s) + f${i - 1}(x)
}`);
    out.push(`\nfn main() {\n    println!("{}", f${n}(7));\n}\n`);
  } else if (lang === 'moon') {
    out.push('fn[T] pick(c : Bool, a : T, b : T) -> T { if c { a } else { b } }', 'fn f0(x : Int) -> Int { x }');
    for (let i = 1; i <= n; i++) out.push(`
struct P${i} { a : Int; b : Int }
enum E${i} { A(Int); B }
fn f${i}(x : Int) -> Int {
  let p = P${i}::{ a: x, b: x * 2 }
  let e = if x % 2 == 0 { E${i}::A(p.a) } else { E${i}::B }
  let mut s = 0
  let mut k = 0
  while k < ${lim(i)} {
    let v = match e {
      A(v) => v + k
      B => p.b
    }
    s += v
    k += 1
  }
  let g = fn(y : Int) { y + p.a }
  pick(s > 0, g(s), s) + f${i - 1}(x)
}`);
    out.push(`\nfn main {\n  println(f${n}(7))\n}\n`);
  } else {
    out.push('fn pick<T>(c: Bool, a: T, b: T) -> T { if c { a } else { b } }', 'fn f0(x: Int) -> Int { x }');
    for (let i = 1; i <= n; i++) out.push(`
struct P${i} {
    a: Int
    b: Int
}
enum E${i} {
    A(Int)
    B
}
fn f${i}(x: Int) -> Int {
    let p = P${i} { a: x, b: x * 2 }
    let e = if x % 2 == 0 { E${i}.A(p.a) } else { E${i}.B }
    var s = 0
    var k = 0
    while k < ${lim(i)} {
        let v = match e {
            E${i}.A(v) -> v + k
            E${i}.B -> p.b
        }
        s += v
        k += 1
    }
    let g = |y: Int| y + p.a
    pick(s > 0, g(s), s) + f${i - 1}(x)
}`);
    out.push(`\nfn main() {\n    print(f${n}(7))\n}\n`);
  }
  return out.join('\n');
}

// ---------------------------------------------------------------- projects

const LANGS = {
  rust: {
    src: 'src/main.rs',
    setup: (d) => writeFileSync(join(d, 'Cargo.toml'), '[package]\nname = "p"\nversion = "0.1.0"\nedition = "2021"\n'),
    clean: (d) => rmSync(join(d, 'target'), { recursive: true, force: true }),
    check: ['cargo', 'check', '-q', '--offline'],
    build: ['cargo', 'build', '-q', '--offline'],
    release: ['cargo', 'build', '-q', '--offline', '--release'],
    run: ['cargo', 'run', '-q', '--offline'],
  },
  moon: {
    src: 'main/main.mbt',
    setup: (d) => {
      writeFileSync(join(d, 'moon.mod'), 'name = "c/p"\n\nversion = "0.1.0"\n');
      writeFileSync(join(d, 'main/moon.pkg'), 'options(\n  "is-main": true,\n)\n');
    },
    clean: (d) => rmSync(join(d, '_build'), { recursive: true, force: true }),
    check: ['moon', 'check', '-q'],
    build: ['moon', 'build', '-q', '--target', 'wasm-gc'],
    release: ['moon', 'build', '-q', '--target', 'wasm-gc', '--release'],
    'release native': ['moon', 'build', '-q', '--target', 'native', '--release'],
    run: ['moon', 'run', '-q', 'main', '--target', 'wasm-gc'],
  },
  pud: {
    src: 'main.pud',
    setup: () => {},
    clean: (d) => rmSync(join(d, 'out'), { recursive: true, force: true }),
    check: [PUD, 'check', 'main.pud'],
    build: [PUD, 'build', 'main.pud', '-o', 'out'],
    run: [PUD, 'run', 'main.pud'],
  },
};

function exec(cmd, cwd, env = {}) {
  const t = performance.now();
  const r = spawnSync(cmd[0], cmd.slice(1), { cwd, encoding: 'utf8', env: { ...process.env, ...env }, maxBuffer: 1 << 26 });
  const ms = performance.now() - t;
  if (r.status !== 0) throw new Error(`${cmd.join(' ')} in ${cwd}: ${r.status} ${r.error || ''}\n${(r.stdout + r.stderr).slice(-1500)}`);
  return { ms, out: (r.stdout + r.stderr).trim() };
}

const R = { reps: REPS, lines: {}, times: {}, pudPhases: {}, errors: [] };
const add = (prog, step, lang, ms) => ((R.times[`${prog}|${step}`] ??= {})[lang] ??= []).push(+ms.toFixed(1));

for (const prog of PROGS) {
  for (const [lang, L] of Object.entries(LANGS)) {
    const d = join(WORK, lang, prog);
    rmSync(d, { recursive: true, force: true });
    mkdirSync(join(d, L.src, '..'), { recursive: true });
    L.setup(d);
    let v = 0;
    const write = () => { v ^= 1; writeFileSync(join(d, L.src), gen(lang, prog, v)); };
    write();
    R.lines[prog] ??= {};
    R.lines[prog][lang] = gen(lang, prog, 0).split('\n').length;
    try {
      exec(L.build, d); // warm-up: tool start-up, file caches
      for (let rep = 0; rep < REPS; rep++) {
        L.clean(d);
        add(prog, 'check (cold)', lang, exec(L.check, d).ms);
        write();
        add(prog, 'edit, check', lang, exec(L.check, d).ms);
        L.clean(d);
        add(prog, 'build (cold)', lang, exec(L.build, d).ms);
        write();
        add(prog, 'edit, run', lang, exec(L.run, d).ms);
        add(prog, 'run again', lang, exec(L.run, d).ms);
        for (const step of ['release', 'release native']) {
          if (!L[step]) continue;
          L.clean(d);
          add(prog, `${step} (cold)`, lang, exec(L[step], d).ms);
        }
      }
      if (lang === 'pud') {
        L.clean(d);
        R.pudPhases[prog] = exec(L.build, d, { PUD_TIME: '1' }).out.split('\n').filter((l) => l.startsWith('[time]'));
      }
    } catch (e) {
      R.errors.push(String(e.message || e));
      console.error(e.message || e);
    }
  }
}
writeFileSync(OUT, JSON.stringify(R, null, 1));

// ---------------------------------------------------------------- report

const med = (a) => { if (!a?.length) return null; const s = [...a].sort((x, y) => x - y); return s[s.length >> 1]; };
const fmt = (x) => (x == null ? '-' : x >= 10000 ? `${(x / 1000).toFixed(1)} s` : `${Math.round(x)}`);
const lines = ['| program | step | cargo (Rust) | moon (MoonBit) | pud (Puddi) |', '|---|---|---|---|---|'];
for (const prog of PROGS) {
  for (const [key, t] of Object.entries(R.times)) {
    const [p, step] = key.split('|');
    if (p === prog) lines.push(`| ${prog} | ${step} | ${fmt(med(t.rust))} | ${fmt(med(t.moon))} | ${fmt(med(t.pud))} |`);
  }
}
console.log(`lines: ${JSON.stringify(R.lines)}\n\n${lines.join('\n')}\n\n${JSON.stringify(R.pudPhases, null, 1)}\nerrors: ${R.errors.length}`);
