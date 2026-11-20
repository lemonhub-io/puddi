// Summarises results.json into Markdown tables: node report.mjs results.json > RESULTS.md
import { readFileSync } from 'node:fs';

const R = JSON.parse(readFileSync(process.argv[2] || 'results.json', 'utf8'));
const K = ['rust', 'moonbit-native', 'moonbit-wasm-gc', 'puddi-node', 'puddi-wasmtime'];
const med = (a) => { const s = a.filter((x) => Number.isFinite(x)).sort((x, y) => x - y); return s.length ? (s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2) : NaN; };
const spread = (a) => { const s = a.filter(Number.isFinite); return s.length ? (Math.max(...s) - Math.min(...s)) / med(s) * 100 : NaN; };
const f = (x, d = 0) => (Number.isFinite(x) ? x.toFixed(d) : '-');
const MB = (b) => b / 2 ** 20;
const geo = (a) => { const s = a.filter((x) => Number.isFinite(x) && x > 0); return s.length ? Math.exp(s.reduce((p, x) => p + Math.log(x), 0) / s.length) : NaN; };
const ok = (e) => e && !e.err;
const benches = Object.keys(R.runs).filter((b) => b !== 'hello');
const out = [];
const table = (head, rows) => { out.push(`| ${head.join(' | ')} |`, `|${head.map(() => '---').join('|')}|`, ...rows.map((r) => `| ${r.join(' | ')} |`), ''); };
const col = (b, k, field) => (R.runs[b]?.[k] || []).filter(ok).map((e) => e[field]);

out.push(`Machine: ${R.machine.cpu}, ${R.machine.cores} threads, ${R.machine.memGB} GB, ${R.machine.os}`, '');
out.push(Object.entries(R.versions).map(([k, v]) => `${k}: ${v}`).join('  \n'), '');

out.push('## Kernel time (ms, median, measured inside the program)', '');
const ratios = Object.fromEntries(K.map((k) => [k, []]));
table(['bench', ...K], benches.map((b) => {
  const rust = med(col(b, 'rust', 'kernel'));
  return [b, ...K.map((k) => {
    const m = med(col(b, k, 'kernel'));
    if (Number.isFinite(m) && rust > 0) ratios[k].push(m / rust);
    return k === 'rust' ? f(m, 1) : `${f(m, 0)} (${f(m / rust, 1)}x)`;
  })];
}));
table(['', ...K], [['geomean vs Rust', ...K.map((k) => `${f(geo(ratios[k]), 2)}x`)]]);

out.push('## Spread of kernel time across runs (max-min)/median, %', '');
table(['bench', ...K], benches.map((b) => [b, ...K.map((k) => f(spread(col(b, k, 'kernel')), 0))]));

out.push('## End-to-end wall time (ms, median, includes start-up; Puddi `pud run` also compiles)', '');
table(['bench', ...K], ['hello', ...benches].map((b) => [b, ...K.map((k) => f(med(col(b, k, 'wall')), 0))]));

out.push('## CPU time of the whole process tree (ms, median)', '');
table(['bench', ...K], ['hello', ...benches].map((b) => [b, ...K.map((k) => f(med(col(b, k, 'cpu')), 0))]));

out.push('## Peak committed memory of the process tree (MB, median)', '');
table(['bench', ...K], ['hello', ...benches].map((b) => [b, ...K.map((k) => f(MB(med(col(b, k, 'peak'))), 1))]));

out.push('## Build time (ms) and output size (KB)', '');
const B = R.build;
table(['bench', 'rustc -O3', 'Rust exe', 'pud build', 'Puddi wasm', 'MoonBit wasm-gc', 'MoonBit native'], ['hello', ...benches].map((b) => [
  b, f(B[`rs/${b}`]?.ms), f(B[`rs/${b}`]?.size / 1024), f(B[`pud/${b}`]?.ms), f(B[`pud/${b}`]?.size / 1024),
  f(B[`mbt-wasm-gc/${b}`]?.size / 1024, 1), f(B[`mbt-native/${b}`]?.size / 1024),
]));
out.push(`MoonBit builds all ${benches.length + 1} programs as one module: wasm-gc ${f(B['mbt/wasm-gc']?.ms)} ms, native ${f(B['mbt/native']?.ms)} ms (clean, release).`, '');
const pf = B['pud/fib']?.files;
if (pf) out.push(`Puddi output files for fib: ${Object.entries(pf).map(([k, v]) => `${k} ${f(v / 1024, 1)} KB`).join(', ')}`, '');

out.push('## Checksums', '');
table(['bench', ...K], benches.map((b) => [b, ...K.map((k) => [...new Set((R.runs[b]?.[k] || []).map((e) => e.err ? 'ERR' : e.check))].join(' '))]));
if (R.errors.length) { out.push('## Errors', '', ...R.errors.map((e) => `- ${e.replace(/\n/g, ' ').slice(0, 300)}`), ''); }
console.log(out.join('\n'));
