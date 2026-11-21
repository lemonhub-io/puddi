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
| fib | 69 | 101 (1.5x) | 93 (1.3x) | 88 (1.3x) | 85 (1.2x) |
| sieve | 20 | 23 (1.2x) | 48 (2.4x) | 43 (2.2x) | 48 (2.4x) |
| nbody | 133 | 176 (1.3x) | 179 (1.3x) | 190 (1.4x) | 216 (1.6x) |
| trees | 3936 | 2022 (0.5x) | 525 (0.1x) | 572 (0.1x) | 957 (0.2x) |
| mandel | 135 | 138 (1.0x) | 335 (2.5x) | 139 (1.0x) | 197 (1.5x) |
| hashmap | 39 | 279 (7.3x) | 264 (6.9x) | 48 (1.3x) | 75 (2.0x) |
| strings | 14 | 115 (8.2x) | 63 (4.5x) | 77 (5.5x) | 89 (6.3x) |
| sort | 14 | 73 (5.1x) | 79 (5.5x) | 21 (1.4x) | 41 (2.8x) |
| iter | 7 | 59 (8.5x) | 58 (8.3x) | 10 (1.4x) | 11 (1.6x) |
| dyn | 40 | 22 (0.5x) | 75 (1.9x) | 49 (1.2x) | 184 (4.5x) |
| **geomean** | 1.00x | 2.10x | 2.28x | 1.26x | 1.84x |

`trees` is faster than Rust in every GC language because Rust frees each tree node by node, while a garbage collector discards a dead tree at once. The Rust programs are plain idiomatic code (std `HashMap` with SipHash, `Box`, `sort`), not tuned.

### Puddi 0.1.0 and the current compiler

| | 0.1.0 on V8 | now | 0.1.0 on Wasmtime | now |
|---|---|---|---|---|
| hello (wall) | 120 | 123 | 160 | 75 |
| fib | 235 | 88 (2.7x) | 397 | 85 (4.7x) |
| sieve | 183 | 43 (4.3x) | 230 | 48 (4.8x) |
| nbody | 269 | 190 (1.4x) | 1099 | 216 (5.1x) |
| trees | 993 | 572 (1.7x) | 2818 | 957 (2.9x) |
| mandel | 385 | 139 (2.8x) | 327 | 197 (1.7x) |
| hashmap | 89 | 48 (1.9x) | 160 | 75 (2.1x) |
| strings | 115 | 77 (1.5x) | 148 | 89 (1.7x) |
| sort | 260 | 21 (12.6x) | 5382 | 41 (132x) |
| iter | 67 | 10 (7.0x) | 492 | 11 (45x) |
| dyn | 147 | 49 (3.0x) | 539 | 184 (2.9x) |
| **geomean vs Rust** | 3.98x | 1.26x | 11.03x | 1.84x |

The changes are listed in the changelog under Unreleased. In short: Wasmtime's copying collector instead of reference counting (reference-count barriers made `sort` take 5 s), Cranelift inlining, a 16 MiB starting GC heap, a compiled-code cache, shared instances for payload-less enum variants, inline overflow checks plus range analysis to drop impossible ones, direct `List` indexing, allocation-free `for` over lists, internal iteration for iterator chains, an IR optimizer that inlines closures and constructor matches (`src/opt.rs`), radix sort for integer lists, single-probe `Map` inserts, V8 without its Liftoff baseline tier, and a faster compiler start-up.

### Remaining gaps

- **`dyn` calls on Wasmtime** (4.5x): each call goes through a shim that casts `anyref` back to the concrete type, and Cranelift does not inline `call_ref`. Storing the vtable in the object would save the cast and one indirection.
- **`trees` on Wasmtime**: its copying collector grows the GC heap only when a collection frees too little, so it collects often near capacity; V8's generational collector is 1.7x faster here.
- **`strings`** (5-6x): string building copies through linear memory and `Map<Str, Int>` hashes byte by byte.
- **Start-up**: `pud run hello` takes 75 ms on Wasmtime, about 30 ms of it the process launch on Windows and 45 ms compiling (half of that in parsing the generated WIT bindings and the WAT text).
- **Output size**: every Puddi program is about 230 KB (98 KB core module), against 0.3 to 13 KB for MoonBit's wasm-gc output, because the platform layer and runtime are linked in whole.
- **Memory**: Puddi needs 26 MB (V8) or 52 MB (Wasmtime) for `hello`, against 6 MB for `moonrun`; most of it is the engine.
