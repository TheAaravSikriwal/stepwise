# Stepwise

Write code in a small teaching language, run it in your browser, and **step backward** through
every line to see exactly what happened.

> Status: the MVP works end to end: write a program, run it, and step through it in both directions,
> with every step explained in plain English. Ask any variable "where did this value come from?",
> or open the call galaxy to see the whole run as a 3D map. See [STATUS.md](STATUS.md).

## How it works

A compiler written in Rust turns Stepwise source into WebAssembly. The compiler inserts calls that
record every step, assignment, function call and print into a trace. The debugger then replays
that recording in either direction. There's no server: everything runs in the browser.

- [docs/product-plan.md](docs/product-plan.md): what we're building and why
- [DEVPLAN.md](DEVPLAN.md): how we're building it, who owns what, and the problem playbook
- [docs/language.md](docs/language.md): the whole language on one page
- [docs/how-it-works.md](docs/how-it-works.md): record and replay, in plain English
- [docs/specs/](docs/specs/): the spec for each core piece (lexer, parser, checker, codegen, replay)

## Layout

| Path | What |
|---|---|
| `compiler/` | The compiler crate (`stepwise-compiler`) |
| `compiler-wasm/` | wasm-bindgen wrapper that exposes `compile()` to JavaScript |
| `web/` | The site: Vite + TypeScript, the worker, runtime, and UI |
| `docs/` | Plans, specs, and the language reference |

## Setup

You need Rust (stable, with the `wasm32-unknown-unknown` target), `wasm-pack`, and Node 24+.

```bash
cargo test --workspace          # compiler tests (all of them)
cd web
npm install
npm run wasm                    # build the compiler to web/pkg
npm test                        # runtime + end-to-end tests
npm run dev                     # http://localhost:5173
```

Rebuild with `npm run wasm` after changing any Rust code.

### Windows: Smart App Control

If builds randomly fail with `can't find crate for ...` or "An Application Control policy has
blocked this file", Windows Smart App Control is blocking DLLs the Rust compiler just built. See
STATUS.md for options. To format code without `cargo fmt`, run `rustfmt` on the files directly;
`rustfmt.toml` sets the edition.
