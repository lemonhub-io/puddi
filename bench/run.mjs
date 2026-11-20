// Benchmark driver: builds every benchmark for Rust, Puddi and MoonBit, then runs them one
// at a time (never in parallel, since timings are sensitive to contention) and writes results.json.
//   node run.mjs [--reps 3] [--only fib,sieve] [--skip-build]
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, statSync, writeFileSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import os from 'node:os';

const argv = process.argv.slice(2);
const opt = (k, d) => { const i = argv.indexOf(k); return i < 0 ? d : argv[i + 1]; };
const reps = +opt('--reps', 3);
const PUD = opt('--pud', 'pud');
const only = opt('--runners', '') ? opt('--runners').split(',') : null;
const out = opt('--out', 'results.json');
const ALL = ['hello', 'fib', 'sieve', 'nbody', 'trees', 'mandel', 'hashmap', 'strings', 'sort', 'iter', 'dyn'];
const benches = opt('--only', '') ? opt('--only').split(',') : ALL;
const root = resolve('.');
const bin = join(root, 'bin');
mkdirSync(join(bin, 'rs'), { recursive: true });
mkdirSync(join(bin, 'pud'), { recursive: true });

const run = (cmd, args, o = {}) => {
  const t = performance.now();
  const r = spawnSync(cmd, args, { encoding: 'utf8', maxBuffer: 1 << 28, ...o });
  return { ms: performance.now() - t, code: r.status, out: (r.stdout || '') + (r.stderr || '') + (r.error ? String(r.error) : '') };
};
const ver = (cmd, args) => run(cmd, args).out.trim().split('\n')[0];
const findFile = (dir, name) => {
  if (!existsSync(dir)) return null;
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) { const f = findFile(p, name); if (f) return f; } else if (e.name === name) return p;
  }
  return null;
};
const dirSize = (d) => readdirSync(d).reduce((a, f) => a + statSync(join(d, f)).size, 0);

const R = {
  machine: { cpu: os.cpus()[0].model, cores: os.cpus().length, memGB: Math.round(os.totalmem() / 2 ** 30), os: `${os.type()} ${os.release()}` },
  versions: { rustc: ver('rustc', ['--version']), pud: ver(PUD, ['--version']), moon: ver('moon', ['version']), moonc: ver('moonc', ['-v']), node: process.version },
  build: {}, runs: {}, errors: [],
};
const log = (...a) => console.log(...a);

// ---------------------------------------------------------------- build
const meter = join(bin, 'meter.exe');
const m = run('rustc', ['-C', 'opt-level=3', 'meter.rs', '-o', meter]);
if (m.code !== 0) { console.error(m.out); process.exit(1); }

const wasmPaths = {}, nativePaths = {};
if (!argv.includes('--skip-build')) {
  for (const b of benches) {
    const exe = join(bin, 'rs', `${b}.exe`);
    if (!only || only.includes('rust')) {
    const r = run('rustc', ['-C', 'opt-level=3', '-C', 'codegen-units=1', `rs/${b}.rs`, '-o', exe]);
    R.build[`rs/${b}`] = { ms: r.ms, size: r.code === 0 ? statSync(exe).size : null };
    if (r.code !== 0) R.errors.push(`rust build ${b}: ${r.out}`);
    }

    const od = join(bin, 'pud', b);
    rmSync(od, { recursive: true, force: true });
    const p = run(PUD, ['build', `pud/${b}.pud`, '-o', od]);
    R.build[`pud/${b}`] = { ms: p.ms, size: p.code === 0 ? dirSize(od) : null, files: p.code === 0 ? Object.fromEntries(readdirSync(od).map((f) => [f, statSync(join(od, f)).size])) : null };
    if (p.code !== 0) R.errors.push(`pud build ${b}: ${p.out}`);
  }
  for (const target of ['wasm-gc', 'native']) {
    if (only && !only.some((k) => k.startsWith('moonbit'))) break;
    rmSync(join(root, 'mbt', '_build'), { recursive: true, force: true });
    const r = run('moon', ['build', '--target', target, '--release'], { cwd: join(root, 'mbt') });
    R.build[`mbt/${target}`] = { ms: r.ms, ok: r.code === 0 };
    if (r.code !== 0) { R.errors.push(`moon build ${target}: ${r.out.slice(-2000)}`); continue; }
    // keep the outputs: the next clean build would delete them
    const keep = join(bin, `mbt-${target}`);
    mkdirSync(keep, { recursive: true });
    for (const b of benches) {
      const f = findFile(join(root, 'mbt', '_build'), target === 'native' ? `${b}.exe` : `${b}.wasm`);
      if (!f) { R.errors.push(`moon ${target}: no output for ${b}`); continue; }
      const dst = join(keep, target === 'native' ? `${b}.exe` : `${b}.wasm`);
      spawnSync(process.execPath, ['-e', `require('fs').copyFileSync(${JSON.stringify(f)}, ${JSON.stringify(dst)})`]);
      R.build[`mbt-${target}/${b}`] = { size: statSync(dst).size };
    }
  }
}
for (const b of benches) {
  const w = join(bin, 'mbt-wasm-gc', `${b}.wasm`), n = join(bin, 'mbt-native', `${b}.exe`);
  if (existsSync(w)) wasmPaths[b] = w;
  if (existsSync(n)) nativePaths[b] = n;
}

// ---------------------------------------------------------------- run
const runners = {
  'rust': (b) => [join(bin, 'rs', `${b}.exe`)],
  'puddi-wasmtime': (b) => [PUD, 'run', `pud/${b}.pud`],
  'puddi-node': (b) => [PUD, 'run', `pud/${b}.pud`],
  'puddi-wt-nocache': (b) => [PUD, 'run', `pud/${b}.pud`],
  'puddi-wt-drc': (b) => [PUD, 'run', `pud/${b}.pud`],
  'puddi-wt-noinline': (b) => [PUD, 'run', `pud/${b}.pud`],
  'puddi-wt-drc-noinline': (b) => [PUD, 'run', `pud/${b}.pud`],
  'puddi-wt-heap16': (b) => [PUD, 'run', `pud/${b}.pud`],
  'puddi-wt-heap64': (b) => [PUD, 'run', `pud/${b}.pud`],
  'puddi-wt-heap256': (b) => [PUD, 'run', `pud/${b}.pud`],
  'moonbit-wasm-gc': (b) => wasmPaths[b] && ['moonrun', wasmPaths[b]],
  'moonbit-native': (b) => nativePaths[b] && [nativePaths[b]],
};
const VARIANT_ENV = {
  'puddi-node': { PUD_HOST: 'node' },
  'puddi-wt-nocache': { PUD_NO_CACHE: '1' },
  'puddi-wt-drc': { PUD_WASMTIME_GC: 'drc' },
  'puddi-wt-noinline': { PUD_WASMTIME_INLINE: '0' },
  'puddi-wt-drc-noinline': { PUD_WASMTIME_GC: 'drc', PUD_WASMTIME_INLINE: '0' },
  'puddi-wt-heap16': { PUD_WASMTIME_HEAP_MB: '16' },
  'puddi-wt-heap64': { PUD_WASMTIME_HEAP_MB: '64' },
  'puddi-wt-heap256': { PUD_WASMTIME_HEAP_MB: '256' },
};
const envFor = (k) => ({ ...process.env, ...(VARIANT_ENV[k] || {}) });
// variants are opt-in through --runners
if (!only) for (const k of Object.keys(VARIANT_ENV)) if (k.startsWith('puddi-wt-')) delete runners[k];

for (const b of benches) {
  for (let i = -1; i < reps; i++) {
    for (const [k, f] of Object.entries(runners)) {
      if (only && !only.includes(k)) continue;
      const cmd = f(b);
      if (!cmd) continue;
      const r = run(meter, cmd, { env: envFor(k) });
      if (i < 0) continue; // warm-up: file caches and the virus scanner's first look at new binaries
      const lines = r.out.trim().split(/\r?\n/);
      const mline = lines.find((l) => l.startsWith('@@METER '));
      const met = mline ? JSON.parse(mline.slice(8)) : { code: -1 };
      const outl = lines.filter((l) => !l.startsWith('@@METER ')).map((l) => l.trim()).filter(Boolean);
      const res = outl.find((l) => l.startsWith(`${b} `)) || (b === 'hello' ? outl.find((l) => l === 'hi') : null);
      const toks = res ? res.split(' ') : [];
      const entry = { ...met, check: b === 'hello' ? res : toks[1], kernel: b === 'hello' ? null : +toks[2] };
      if (met.code !== 0 || !res) { entry.err = outl.slice(-15).join('\n'); R.errors.push(`${k} ${b}: ${entry.err}`); }
      ((R.runs[b] ??= {})[k] ??= []).push(entry);
      log(`${b.padEnd(8)} ${k.padEnd(16)} kernel=${entry.kernel ?? '-'} wall=${met.wall} peak=${Math.round((met.peak || 0) / 2 ** 20)}MB ${entry.err ? 'ERR' : ''}`);
      writeFileSync(out, JSON.stringify(R, null, 1));
    }
  }
}
writeFileSync(out, JSON.stringify(R, null, 1));
log(`errors: ${R.errors.length}`);
for (const e of R.errors) log('  ' + e.slice(0, 400));
