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
| fib | 69 | 101 (1.5x) | 93 (1.3x) | 90 (1.3x) | 87 (1.3x) |
| sieve | 20 | 23 (1.2x) | 48 (2.4x) | 44 (2.2x) | 48 (2.4x) |
| nbody | 133 | 176 (1.3x) | 179 (1.3x) | 191 (1.4x) | 215 (1.6x) |
| trees | 3936 | 2022 (0.5x) | 525 (0.1x) | 565 (0.1x) | 946 (0.2x) |
| mandel | 135 | 138 (1.0x) | 335 (2.5x) | 137 (1.0x) | 197 (1.5x) |
| hashmap | 39 | 279 (7.3x) | 264 (6.9x) | 46 (1.2x) | 66 (1.7x) |
| strings | 14 | 115 (8.2x) | 63 (4.5x) | 74 (5.3x) | 92 (6.6x) |
| sort | 14 | 73 (5.1x) | 79 (5.5x) | 21 (1.4x) | 43 (3.0x) |
| iter | 7 | 59 (8.5x) | 58 (8.3x) | 10 (1.4x) | 12 (1.7x) |
| dyn | 40 | 22 (0.5x) | 75 (1.9x) | 52 (1.3x) | 187 (4.6x) |
| **geomean** | 1.00x | 2.10x | 2.28x | 1.25x | 1.85x |

`trees` is faster than Rust in every GC language because Rust frees each tree node by node, while a garbage collector discards a dead tree at once. The Rust programs are plain idiomatic code (std `HashMap` with SipHash, `Box`, `sort`), not tuned.

### Puddi 0.1.0 and the current compiler

| | 0.1.0 on V8 | now | 0.1.0 on Wasmtime | now |
|---|---|---|---|---|
| hello (wall) | 120 | 116 | 160 | 25 |
| fib | 235 | 90 (2.6x) | 397 | 87 (4.6x) |
| sieve | 183 | 44 (4.2x) | 230 | 48 (4.8x) |
| nbody | 269 | 191 (1.4x) | 1099 | 215 (5.1x) |
| trees | 993 | 565 (1.8x) | 2818 | 946 (3.0x) |
| mandel | 385 | 137 (2.8x) | 327 | 197 (1.7x) |
| hashmap | 89 | 46 (1.9x) | 160 | 66 (2.4x) |
| strings | 115 | 74 (1.6x) | 148 | 92 (1.6x) |
| sort | 260 | 21 (12.4x) | 5382 | 43 (125x) |
| iter | 67 | 10 (6.7x) | 492 | 12 (41x) |
| dyn | 147 | 52 (2.8x) | 539 | 187 (2.9x) |
| **geomean vs Rust** | 3.98x | 1.25x | 11.03x | 1.85x |

The changes are listed in the changelog under Unreleased. In short: Wasmtime's copying collector instead of reference counting (reference-count barriers made `sort` take 5 s), a 16 MiB starting GC heap, a compiled-code cache, shared instances for payload-less enum variants, inline overflow checks plus range analysis to drop impossible ones, direct `List` indexing, allocation-free `for` over lists, internal iteration for iterator chains, an IR optimizer that inlines closures and constructor matches (`src/opt.rs`), radix sort for integer lists, single-probe `Map` inserts, V8 without its Liftoff baseline tier, and a faster compiler (see Compile time below). Cranelift's own inliner is off: with `opt.rs` it no longer makes a measurable difference, and it made compiling after each edit 1.7x slower.

### Remaining gaps

- **`dyn` calls on Wasmtime** (4.6x): each call goes through a shim that casts `anyref` back to the concrete type, and Cranelift does not inline `call_ref`. Storing the vtable in the object would save the cast and one indirection.
- **`trees` on Wasmtime**: its copying collector grows the GC heap only when a collection frees too little, so it collects often near capacity; V8's generational collector is 1.7x faster here.
- **`strings`** (5-6x): string building copies through linear memory and `Map<Str, Int>` hashes byte by byte.
- **Start-up**: `pud run hello` takes 25 ms when nothing changed and 110 ms after an edit, against 12 ms for a Rust binary; most of it is the process launch on Windows and loading Wasmtime.
- **Output size**: every Puddi program is about 175 KB with its JavaScript loader, against 0.3 to 13 KB for MoonBit's wasm-gc output, because the platform layer and runtime are linked in whole.
- **Memory**: Puddi needs 26 MB (V8) or 34 MB (Wasmtime) for `hello`, against 6 MB for `moonrun`; most of it is the engine.

## Compile time

`compile.mjs` times the steps of the edit-compile-run loop with each language's own tool (`cargo`, `moon`, `pud`) on `hello` and on two generated programs with the same code in each language: 200 and 2,000 functions, each with a struct, an enum, a `match`, a loop, a closure and a generic call (about 3,800 and 38,000 lines of Rust). "cold" starts from a clean project; "edit" changes one line of one function in the warm project (always to text no earlier build has seen), so each tool can reuse its incremental state. `pud` gets a cache directory of its own. Median of 3, ms; raw data in `results/compile.json`.

```sh
node compile.mjs --reps 3 --pud path/to/pud
```

| program | step | cargo (Rust) | moon (MoonBit) | pud (Puddi) |
|---|---|---|---|---|
| hello | check (cold) | 254 | 179 | 49 |
| hello | edit, check | 179 | 160 | 48 |
| hello | build (cold) | 372 | 187 | 64 |
| hello | edit, run | 473 | 217 | 110 |
| hello | run again | 128 | 121 | 30 |
| gen200 | check (cold) | 382 | 224 | 61 |
| gen200 | edit, check | 226 | 207 | 61 |
| gen200 | build (cold) | 638 | 257 | 84 |
| gen200 | edit, run | 585 | 287 | 138 |
| gen200 | run again | 126 | 121 | 32 |
| gen2000 | check (cold) | 1556 | 770 | 143 |
| gen2000 | edit, check | 685 | 768 | 145 |
| gen2000 | build (cold) | 3113 | 1145 | 253 |
| gen2000 | edit, run | 2385 | 1178 | 523 |
| gen2000 | run again | 132 | 147 | 43 |

Release builds take about as long as debug builds for these programs (`cargo build --release` 3.1 s and `moon build --release` 1.2 s on gen2000); `moon build --target native --release` takes 13 s there, most of it in the C compiler. Puddi has a single build mode.

Puddi checks 4 to 11 times faster than `cargo` and 3 to 5 times faster than `moon`, and goes from an edit to a running program about 2 times faster than `moon` and 4 times faster than `cargo`, although it recompiles the whole program every time. What makes this work:

- `pud build` and `pud run` reuse the previous build when no input changed (sources, WIT files, target, compiler), so `run again` costs only start-up
- Wasmtime keeps Cranelift's output per function, so after an edit only changed functions are compiled to machine code again; an unchanged program loads its cached machine code directly
- large source files (the prelude, big user files) are lexed and parsed in pieces on several threads
- functions are turned into WebAssembly text on several threads, each with its own copy of the code generator's state, and the results are merged in function order, so the text is the same as from one thread
- `src/asm.rs` turns that text into binary on several threads, 10x faster than the `wat` crate and with the same output

For gen2000, `PUD_TIME=1` shows where the 215 ms of a build go: elaboration 60, optimization 30, generating WebAssembly text 25, assembling 15, the rest in parsing, WIT bindings and component encoding.
