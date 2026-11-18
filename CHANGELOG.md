# Changelog

## [0.1.0] - 2026-11-18

First release of Puddi and its `pud` toolchain, implementing the draft 0.1 language specification.

### Language

- Structs, enums, tuples, generics with bounds, traits with default methods, associated types and supertraits, `dyn Trait`, operator traits and closures
- Pattern matching with exhaustiveness checking, `if let`, `while let` and `let ... else`
- `?` with `From` conversion, the `Error` trait and `AnyErr`
- `@derive(Eq, Ord, Hash, Show, Transfer)`, constants, type aliases, modules and imports
- `async fn` and `.await`, `task.scope` with structured cancellation, and workers
- WIT imports, `export fn`, `@extern` and `@export`, `@target`, `@deprecated` and `test` blocks

### Toolchain

- `pud new`, `build`, `run`, `test`, `check`, `fmt`, `doc` and `add`
- `browser`, `wasi` (WASI 0.3) and `plugin` targets, all emitted as WebAssembly components on WasmGC
- An embedded Wasmtime runtime for `pud run`
- Git dependencies with minimal version selection and `pud.lock`, plus local path dependencies

### Testing and distribution

- A conformance suite in `conformance/` with cases for every normative section of SPEC.md
- CI on Linux, macOS and Windows
- Prebuilt binaries for Linux (x86_64, arm64), macOS (x86_64, arm64) and Windows (x86_64), with install scripts for each

### Known gaps

The conformance suite marks these differences from the spec as expected failures:

- no warnings for naming-convention violations, and struct literals need an uppercase type name (2.4)
- a float literal next to an unsigned integer does not default to `Float` (3.7)
- `Self.Item` does not resolve inside an impl block, and a duplicate impl is accepted (3.12)
- unannotated closure parameters are not inferred from the body or later calls (4.6)
- plugin exports that use WIT records, variants, enums or flags fail to compile (8.3)

[0.1.0]: https://github.com/puddi-lang/puddi/releases/tag/v0.1.0
