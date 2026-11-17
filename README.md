# Puddi

A statically typed, expression-oriented language that compiles to WebAssembly.

The language specification is in [SPEC.md](SPEC.md) (draft 0.1).

```puddi
@derive(Eq, Show)
struct Todo {
    title: Str
    var done: Bool
}

fn main() {
    let todos = [Todo { title: "spec", done: true }, Todo { title: "compiler", done: false }]
    let pending = todos.filter(|t| !t.done)
    print("{pending.len()} of {todos.len()} left")
}
```

## The `pud` toolchain

`pud` is written in Rust. It compiles Puddi to WasmGC. The `wasi` and `plugin` targets produce WebAssembly components; the `browser` target produces a core module with JavaScript glue.

```sh
cargo build --release          # builds target/release/pud (with the embedded Wasmtime runtime)
pud new hello && cd hello
pud run                        # build src/main.pud as a WASI 0.3 component and run it
pud test                       # run test blocks, each in a fresh instance
pud run --target browser       # build for the browser and serve it on http://127.0.0.1:8000
pud build --target plugin      # build a component for the world in [target.plugin]
pud add github.com/someone/lib@1.2   # add a dependency
pud run some_file.pud          # single-file mode, no package needed
```

| Command | Purpose |
|---|---|
| `pud new <name>` | create a package |
| `pud build [--target browser\|wasi\|plugin] [--emit-wat] [-o dir]` | compile to `target/<target>/<name>.wasm` |
| `pud run [path] [-- args]` | build and run (`wasi`: embedded Wasmtime; `browser`: dev server) |
| `pud test [path] [filter]` | run `test` blocks with Node.js |
| `pud check [path]` | type-check only |
| `pud add <repo@version\|path> [--name n]` | add a dependency to `pud.toml` and update `pud.lock` |
| `pud fmt [files] [--check]` | format in the official style |
| `pud doc [path]` | write Markdown docs for public items to `target/doc` |

`cargo build --no-default-features` leaves out the embedded runtime for a much faster build; `pud run` then asks for an external WASI 0.3 host with component-model async, threading, GC and exceptions enabled. `PUD_HOST=node pud run` runs the `wasi` target's core module with Node.js instead. Tests and the browser target need Node.js 22 or newer (WebAssembly GC; JSPI for async code in Node 24+ and current browsers).

### Targets

- **wasi:** a component of the `wasi:cli/command@0.3.2` world (or `[target.wasi] world` in `pud.toml`). The runtime talks to WASI directly: stdio goes through `stream<u8>`, files through `wasi:filesystem` preopens, time and randomness through `wasi:clocks` and `wasi:random`, and `std.http` through `wasi:http/client`. `run` is lifted with the stackful async ABI and every spawned task runs on its own component-model thread, so a blocking WASI call suspends only its task. Floats are formatted and parsed in software ([src/soft.pud](src/soft.pud)) with the same shortest round-trip output as JavaScript.
- **plugin:** a component for a host-defined world, set with `[target.plugin] world = "wit/host.wit#plugin"`. `export fn` items implement the world's exports; missing exports are compile errors. See [examples/plugin](examples/plugin).
- **browser:** a core module plus `puddi.mjs`, with `std.dom`, `std.http` (fetch) and `std.worker` (Web Workers).

### WIT imports

`import wasi:filesystem/preopens` (or any interface from the package's `wit/` directory) generates Puddi bindings: records become structs, variants and enums become enums, flags become structs of `Bool`, resources become handle structs with methods and `drop()`, and the canonical ABI lowering and lifting are generated as Puddi code. The interface is added to the component's world automatically.

```puddi
import wasi:clocks/system-clock as clock

fn main() {
    print("seconds since the epoch: {clock.now().seconds}")
}
```

### Dependencies

`[deps]` entries are git repositories with semver tags (`lib = "github.com/someone/lib@1.2"`) or local paths (`util = { path = "../util" }`). Versions are chosen by minimal version selection, the lock file records the commit of every tag, and modules of a dependency are imported through its name (`import lib.parser`).

## Implementation status

The compiler implements the draft 0.1 language.

- **Language:** structs, enums, tuples, generics with bounds (monomorphized), traits with default methods, associated types and supertraits, `dyn Trait`, operator traits, closures that capture by reference, pattern matching with exhaustiveness checking (guards, ranges, or-patterns, list patterns, `let ... else`, `if let`, `while let`), `?` with `From` conversion, `AnyErr` with `downcast`, `@derive(Eq, Ord, Hash, Show, Transfer)`, constants, type aliases, `async fn`/`.await`, `task.scope` with structured cancellation, workers, implicit numeric widening, checked integer overflow, modules and imports, WIT imports, `test` blocks, `@target`, `@extern`, `@export`/`export fn` and `@deprecated`.
- **Standard library:** the prelude (`Option`, `Result`, `List`, `Map`, `Set` and their views, `Str`, `Char`, `StrBuilder`, iterators and numeric methods) plus `std.math`, `std.log`, `std.time`, `std.random`, `std.io`, `std.fs`, `std.env`, `std.json`, `std.http`, `std.worker` and `std.dom`, each available on the targets listed in SPEC §10.2. The prelude is written in Puddi ([src/prelude.pud](src/prelude.pud), [std/](std)).
- **Concurrency:** tasks run concurrently on component-model threads in `wasi` components and on a JSPI scheduler on the JavaScript host; a scope whose body fails cancels its unfinished children at their next suspension point (in components, a wasm exception unwinds the cancelled task). Workers run in worker threads (Node.js) or Web Workers and exchange `Transfer` values.
- **Output:** WasmGC throughout. Structs and enums map to GC structs, generics are monomorphized, and closures and trait objects are GC structs that hold function references. Components are encoded with `wit-component`.

Limitations:

- `std.worker` is available on `browser`; per SPEC §9.4 other targets need a worker interface in their world, and WASI has none yet. On `plugin`, tasks run when they are spawned and the program has no ambient I/O
- the `browser` target emits a core module with JavaScript glue rather than a component, since browsers cannot load components natively
- the embedded runner links WASI 0.3.0 from Wasmtime 50, which satisfies the 0.3.2 imports through semver matching
- `pud test` runs tests on the JavaScript host, so tests cannot call WIT imports
- grapheme segmentation is an approximation (combining marks, variation selectors, ZWJ sequences), and case mapping covers ASCII only

## Tests

```sh
cargo test      # runs every program in tests/run and checks diagnostics for tests/fail
```

## License

Copyright (c) 2026 Puddi community

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
