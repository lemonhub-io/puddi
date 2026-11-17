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

`pud` is written in Rust. It compiles Puddi to a WasmGC core module and runs programs with Node.js.

```sh
cargo build --release          # builds target/release/pud
pud new hello && cd hello
pud run                        # build and run src/main.pud
pud test                       # run test blocks, each in a fresh instance
pud build --target browser     # writes .wasm, puddi.mjs and index.html to target/browser
pud run some_file.pud          # single-file mode, no package needed
```

| Command | Purpose |
|---|---|
| `pud new <name>` | create a package |
| `pud build [--target browser\|wasi\|plugin] [--emit-wat] [-o dir]` | compile to `target/<target>/<name>.wasm` |
| `pud run [path] [-- args]` | build and run with Node.js |
| `pud test [path] [filter]` | run `test` blocks |
| `pud check [path]` | type-check only |
| `pud fmt [files] [--check]` | format in the official style |
| `pud doc [path]` | write Markdown docs for public items to `target/doc` |

Running needs Node.js 22 or newer, since it uses WebAssembly GC. The generated `puddi.mjs` also loads modules in browsers (`load(url)`), and `makeImports()` gives embedders the host imports.

## Implementation status

The compiler implements most of the draft 0.1 language.

- **Language:** structs, enums, tuples, generics with bounds (monomorphized), traits with default methods, associated types and supertraits, `dyn Trait`, operator traits, closures that capture by reference, pattern matching with exhaustiveness checking (guards, ranges, or-patterns, list patterns, `let ... else`, `if let`, `while let`), `?` with `From` conversion, `AnyErr` with `downcast`, `@derive(Eq, Ord, Hash, Show, Transfer)`, constants, type aliases, `async fn`/`.await`/`task.scope`, implicit numeric widening, checked integer overflow, modules and imports, `test` blocks, `@target`, `@extern`, `@export`/`export fn` and `@deprecated`.
- **Standard library:** the prelude (`Option`, `Result`, `List`, `Map`, `Set` and their views, `Str`, `Char`, `StrBuilder`, iterators and numeric methods) plus `std.math`, `std.log`, `std.time`, `std.random`, `std.io`, `std.fs`, `std.env` and `std.json`, each available on the targets listed in SPEC §10.2. The prelude is written in Puddi ([src/prelude.pud](src/prelude.pud), [std/](std)).
- **Output:** a single WasmGC core module, plus JS glue for Node.js and browsers. Structs and enums map to GC structs, generics are monomorphized, and closures and trait objects are GC structs that hold function references.

Not yet implemented:

- workers (`std.worker`). Async code runs on a sequential scheduler: futures are lazy and run when awaited, and children of a `task.scope` that were never awaited run when the scope ends. Since no standard module suspends yet, cancellation is never triggered and `task.is_cancelled()` is always false
- Component Model output, WIT imports and WASI 0.3 worlds. Targets currently produce core modules with a small host import interface, and `plugin` exports go through `@export`
- package dependencies, `pud.lock` and `pud add`
- `std.http`, `std.worker` and `std.dom`
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
