# debug-rsc-extension

Chrome DevTools panel for inspecting React Server Component (Flight) payloads.

A Rust crate (`flight/`) parses Flight streams and runs as WASM in the panel. Every type crossing the Rust/TS seam is a Rust type exported by ts-rs into `src/bindings/` (committed, never edited by hand).

## Setup

Needs Rust with the `wasm32-unknown-unknown` target, `wasm-pack`, and `fnm`.

```sh
fnm use
npm install
```

## Commands

| Command | What it does |
|---|---|
| `npm run wasm` | Build the Rust crate to `src/wasm/` (gitignored) |
| `npm run bindings` | `cargo test`, which also regenerates `src/bindings/` |
| `npm run check:bindings` | Regenerate from scratch and fail on any diff or `bigint` |
| `npm run dev` / `npm run build` | Build the WASM, then the extension with Vite (`dev` keeps rebuilding the panel) |
| `npm run typecheck` | `tsc --noEmit` (needs `npm run wasm` first) |
| `npm test` | vitest (needs `npm run wasm` first) |

Load `.output/chrome-mv3` as an unpacked extension. See [docs/dev-extension.md](docs/dev-extension.md) for the dev loop, builds from CI, and a smoke-test checklist.

CI (`.github/workflows/ci.yml`) runs the Rust and TS tests, the bindings drift check, the typecheck and the build on every pull request and every push to `main`. Every merge to `main` also publishes a GitHub release with the built extension attached as a zip.

## Fixtures

`fixtures/next-16-*.json` are `Chunk[]` recorded from a throwaway Next 16.4.3 app (production and dev builds, the HTML document and the `RSC: 1` fetch). Rust and TS tests both read them, so the parser is tested against real Flight output, not only hand-written rows.
