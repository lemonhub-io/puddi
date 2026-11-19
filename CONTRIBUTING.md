# Contributing to Puddi

Thanks for helping. Bug reports, conformance cases, compiler fixes, standard library work, examples and documentation are all welcome.

- User documentation: https://puddi-lang.github.io/puddi-docs/ (source in [puddi-lang/puddi-docs](https://github.com/puddi-lang/puddi-docs))
- How the compiler is put together: [docs/architecture.md](docs/architecture.md)
- How releases are made: [docs/releasing.md](docs/releasing.md)
- Code of conduct: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
- Security issues: [SECURITY.md](SECURITY.md), not public issues

## Reporting bugs

Open an [issue](https://github.com/puddi-lang/puddi/issues/new/choose) with:

- the output of `pud version` and your platform;
- the smallest program that shows the problem, and the command you ran;
- what happened and what you expected, with reference to [SPEC.md](SPEC.md) when it is about language behavior.

## Building

You need current stable Rust. Node.js 22 or newer is needed for `pud test`, the browser target and the conformance suite.

```sh
cargo build --release                     # target/release/pud, with the embedded Wasmtime runtime
cargo build --no-default-features         # much faster: no embedded runtime
```

Without the embedded runtime, `pud run` needs an external WASI 0.3 host, or `PUD_HOST=node` to run the core module with Node.js. That is enough for most compiler work; build with the runtime before you send a pull request that touches the `wasi` target or the runner.

## Testing

```sh
cargo test --release                              # everything below
cargo test --release -- --skip conformance_suite  # only the regression tests
node conformance/run.mjs --filter 3.7             # conformance cases for one spec section
```

| Suite | Location | What it checks |
|---|---|---|
| Regression tests | `tests/run/*.pud` with `.out` (and optional `.code`) files | programs run and print the expected output |
| Rejection tests | `tests/fail/*.pud` with `.err` files | programs are rejected with the expected diagnostics |
| Plugin test | `tests/run.rs` | the `examples/plugin-host` plugin and its host capabilities |
| Conformance suite | `conformance/` | the toolchain against SPEC.md, section by section ([README](conformance/README.md)) |

CI runs all of them on Linux, macOS and Windows for every pull request, plus a build without the embedded runtime and checks of the install scripts.

### Which test to add

- A change in **language behavior** gets a conformance case that names the spec sections it covers.
- A **bug fix** gets a regression test in `tests/run` or `tests/fail`, or a conformance case when the bug was a spec violation.
- A **known gap** fixed by your change: remove the `xfail` line from its case. The suite fails while an `xfail` case passes, so you will notice.

## Pull requests

- Keep each pull request to one change, and explain the reason for it in the description.
- Write commit subjects as a short sentence in the imperative ("Run wasi tasks on component threads"), with details in the body when they help.
- Update [CHANGELOG.md](CHANGELOG.md) under `Unreleased` for user-visible changes.
- Update [README.md](README.md) or the [documentation](https://github.com/puddi-lang/puddi-docs) when you change commands, flags or behavior that they describe.
- Follow the style of the surrounding code. `pud fmt` shows the official style for Puddi sources.

## Changing the specification

[SPEC.md](SPEC.md) defines the language. Chapters 2 to 10 are normative; the grammar summary, implementation notes and example are not.

- **Editorial changes** (typos, clearer wording with the same meaning, better examples) can go straight to a pull request.
- **Normative changes** change what programs mean or which programs are valid. Open an issue first that describes the problem, the proposed text, its effect on existing programs and alternatives you considered. Once there is agreement, the pull request changes SPEC.md together with conformance cases, and with the implementation or an `xfail` marker on the new cases.

[GOVERNANCE.md](GOVERNANCE.md) drafts a formal RFC process for when the community grows. It is not in effect yet.

## License

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Puddi, as defined in the Apache-2.0 license, shall be dual licensed under the MIT and Apache 2.0 licenses, without any additional terms or conditions.
