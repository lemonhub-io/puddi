// node compare.mjs base.json new.json: median kernel/wall/peak of the Puddi columns, before and after
import { readFileSync } from 'node:fs';
const [A, B] = process.argv.slice(2).map((f) => JSON.parse(readFileSync(f, 'utf8')));
const med = (a) => { const s = a.filter(Number.isFinite).sort((x, y) => x - y); return s.length ? s[(s.length - 1) >> 1] : NaN; };
const get = (R, b, k, f) => med((R.runs[b]?.[k] || []).filter((e) => !e.err).map((e) => e[f]));
const rows = [];
for (const k of ['puddi-node', 'puddi-wasmtime']) {
  for (const b of Object.keys(B.runs)) {
    const f = b === 'hello' ? 'wall' : 'kernel';
    const [x, y] = [get(A, b, k, f), get(B, b, k, f)];
    const [px, py] = [get(A, b, k, 'peak'), get(B, b, k, 'peak')].map((v) => v / 2 ** 20);
    const ck = [...new Set((B.runs[b]?.[k] || []).map((e) => e.check))].join(' ');
    rows.push(`${k.padEnd(15)} ${b.padEnd(8)} ${f.padEnd(6)} ${x.toFixed(0).padStart(6)} -> ${y.toFixed(0).padStart(6)} ms  ${(x / y).toFixed(2).padStart(6)}x   peak ${px.toFixed(0)} -> ${py.toFixed(0)} MB  ${ck}`);
  }
}
console.log(rows.join('\n'));
