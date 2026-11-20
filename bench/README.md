# Benchmarks

Ten small programs, each written in Puddi, Rust and MoonBit with the same algorithm and the same checksum, plus `hello` for start-up. They compare Puddi with a native language (Rust) and with the closest language in design (MoonBit, which also compiles to WasmGC), and track Puddi's own performance across compiler changes.

| Program | Exercises |
|---|---|
| `fib` | recursive calls, checked `Int` arithmetic (`fib(38)`) |
| `sieve` | `List<Bool>` indexing in tight loops (primes below 10 million) |
| `nbody` | `Float` arithmetic on mutable struct fields (5 million steps) |
| `trees` | allocation and GC (binary trees, depth 18) |
| `mandel` | float loop with an early exit (2000 x 2000, 50 iterations) |
| `hashmap` | `Map<Int, Int>` inserts and lookups (1 million each) |
| `strings` | string building, splitting and `Map<Str, Int>` counting |
| `sort` | sorting 1 million `Int` |
| `iter` | `map`, `filter` and `fold` over a range of 10 million |
| `dyn` | `dyn Trait` method calls (20 million) |

## Running

`run.mjs` builds every program and runs them one at a time, never in parallel, since timings suffer from contention. Each program measures its own kernel with a monotonic clock and prints it after the checksum; `meter.rs` (Windows) runs each process in a job object and adds wall time, CPU time and peak committed memory of the whole process tree, so `PUD_HOST=node` counts the Node.js child too. One discarded warm-up run precedes the measured runs.

```sh
node run.mjs --reps 5                                  # everything, results.json
node run.mjs --only fib,sort --runners puddi-wasmtime  # a subset
node run.mjs --pud path/to/pud --out new.json          # another pud build
node report.mjs results.json > RESULTS.md              # tables
node compare.mjs before.json after.json                # Puddi before and after
```

It needs `rustc`, `pud`, `moon` and `moonrun` on `PATH`, and Node.js 22 or newer. The `puddi-wt-*` runners (opt in with `--runners`) measure Wasmtime settings through `PUD_WASMTIME_GC`, `PUD_WASMTIME_INLINE`, `PUD_WASMTIME_HEAP_MB` and `PUD_NO_CACHE`.

Columns: `rust` (rustc `-C opt-level=3`), `moonbit-native` and `moonbit-wasm-gc` (`moon build --release`, the latter run with `moonrun`), `puddi-wasmtime` (`pud run`, embedded Wasmtime) and `puddi-node` (`PUD_HOST=node pud run`, V8).

## Results

Intel Core Ultra 9 285H, Windows 11, rustc 1.100.0, moonc v0.10.22, Node.js 24.15. Median of 5 runs; full tables in [RESULTS.md](RESULTS.md), raw data in `results/`.

Kernel time in ms (multiple of Rust):

| | Rust | MoonBit native | MoonBit wasm-gc | Puddi on V8 | Puddi on Wasmtime |
|---|---|---|---|---|---|
| fib | 69 | 101 (1.5x) | 93 (1.3x) | 105 (1.5x) | 157 (2.3x) |
| sieve | 20 | 23 (1.2x) | 48 (2.4x) | 75 (3.8x) | 49 (2.5x) |
| nbody | 133 | 176 (1.3x) | 179 (1.3x) | 179 (1.3x) | 198 (1.5x) |
| trees | 3936 | 2022 (0.5x) | 525 (0.1x) | 573 (0.1x) | 1024 (0.3x) |
| mandel | 135 | 138 (1.0x) | 335 (2.5x) | 346 (2.6x) | 196 (1.5x) |
| hashmap | 39 | 279 (7.3x) | 264 (6.9x) | 73 (1.9x) | 110 (2.9x) |
| strings | 14 | 115 (8.2x) | 63 (4.5x) | 92 (6.5x) | 95 (6.7x) |
| sort | 14 | 73 (5.1x) | 79 (5.5x) | 141 (9.8x) | 141 (9.9x) |
| iter | 7 | 59 (8.5x) | 58 (8.3x) | 45 (6.4x) | 381 (55x) |
| dyn | 40 | 22 (0.5x) | 75 (1.9x) | 105 (2.6x) | 174 (4.3x) |
| **geomean** | 1.00x | 2.10x | 2.28x | 2.38x | 3.27x |

`trees` is faster than Rust in every GC language because Rust frees each tree node by node, while a garbage collector discards a dead tree at once. The Rust programs are plain idiomatic code (std `HashMap` with SipHash, `Box`, `sort`), not tuned.

### Puddi 0.1.0 and the current compiler

| | 0.1.0 on V8 | now | 0.1.0 on Wasmtime | now |
|---|---|---|---|---|
| hello (wall) | 120 | 129 | 160 | 110 |
| fib | 235 | 105 (2.2x) | 397 | 157 (2.5x) |
| sieve | 183 | 75 (2.4x) | 230 | 49 (4.7x) |
| nbody | 269 | 179 (1.5x) | 1099 | 198 (5.6x) |
| trees | 993 | 573 (1.7x) | 2818 | 1024 (2.8x) |
| mandel | 385 | 346 (1.1x) | 327 | 196 (1.7x) |
| hashmap | 89 | 73 (1.2x) | 160 | 110 (1.5x) |
| strings | 115 | 92 (1.3x) | 148 | 95 (1.6x) |
| sort | 260 | 141 (1.8x) | 5382 | 141 (38x) |
| iter | 67 | 45 (1.5x) | 492 | 381 (1.3x) |
| dyn | 147 | 105 (1.4x) | 539 | 174 (3.1x) |
| **geomean vs Rust** | 3.98x | 2.38x | 11.03x | 3.27x |

The changes are listed in the changelog under Unreleased. In short: Wasmtime's copying collector instead of reference counting (reference-count barriers made `sort` take 5 s), Cranelift inlining, a 16 MiB starting GC heap, a compiled-code cache, shared instances for payload-less enum variants, inline overflow checks plus range analysis to drop impossible ones, direct `List` indexing, allocation-free `for` over lists and internal iteration for iterator chains.

### Remaining gaps

- **Iterator chains on Wasmtime** (`iter`, 55x): each element passes through one `call_ref` per closure; V8 inlines them speculatively, Cranelift cannot. Specializing higher-order functions per closure at compile time would remove the indirect calls.
- **Output size**: every Puddi program is about 290 KB (124 KB core module), against 0.3 to 13 KB for MoonBit's wasm-gc output, because the platform layer and runtime are linked in whole. Dead-code elimination of unused functions and data is the next big step.
- **Memory**: Puddi needs 24 MB (V8) or 38 MB (Wasmtime) for `hello`, against 6 MB for `moonrun`; most of it is the engine.
- **`dyn` calls** go through a shim that casts `anyref` back to the concrete type; storing the vtable in the object would save the cast and one indirection.
