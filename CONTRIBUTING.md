# Contributing to eslint-formatter-crisp

Thanks for your interest in improving `eslint-formatter-crisp`! This document
covers the project layout, the local toolchain, and the conventions CI enforces.
Reading it before opening a PR saves everyone a round-trip.

## Project overview

`eslint-formatter-crisp` is an ESLint formatter whose rendering core is written
in Rust and exposed to Node.js through [N-API-RS](https://napi.rs/). It ships
with **zero runtime dependencies** — the entire formatter (tokenizer, layout
engine, color and width handling, OSC-8 hyperlinks) lives behind a single
native binding. The npm package bundles **seven prebuilt native binaries**
(linux-x64-gnu, linux-x64-musl, linux-arm64-gnu, linux-arm64-musl, darwin-x64,
darwin-arm64, win32-x64-msvc), so installing it never compiles anything and
never asks the user for a Rust toolchain.

## Development prerequisites

- **Rust stable** via [rustup](https://rustup.rs/). The MSRV is whatever the
  CI matrix currently runs; check `Cargo.toml` if in doubt.
- **Node.js >= 18** (matching the `engines.node` field in `package.json`).
- **npm** (a `package-lock.json` is checked in — please do not switch to a
  different package manager in PRs).
- Optional: [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) and
  [zig](https://ziglang.org/) for cross-building non-host binaries locally.

> **Important:** a local `npm run build` produces only the **host platform**
> binary. Building all seven prebuilt binaries happens in GitHub Actions via
> the `release.yml` workflow — there is no supported way to reproduce the full
> matrix on a single developer machine. If you need a binary for a non-host
> triple locally, follow the cross-build instructions in the README's
> "Rebuilding from source" section.

## Getting started

```shell
git clone https://github.com/ReactJSLover/eslint-formatter-crisp.git
cd eslint-formatter-crisp
npm install                  # installs @napi-rs/cli
cargo test && node --test    # Rust unit tests + Node adapter tests
npx napi build --platform --release   # host binary only → crisp.<host-triple>.node
```

After `npm run build` you can run ESLint against the freshly built binary:

```shell
npx eslint . -f crisp
```

## Code structure

| Path                 | Purpose                                                                                                                                                                 |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/engine/`        | Framework-agnostic Rust rendering core (formatter, highlight tokenizer, width/CJK handling, locator, model, plural, rules, style). No N-API types leak into this layer. |
| `src/lib.rs`         | Thin N-API-RS boundary. Typed Rust ↔ JS glue generated via `#[napi]` macros.                                                                                            |
| `index.cjs`          | JavaScript adapter (CommonJS entry). Loads the right prebuilt binary, normalizes ESLint results, calls the native `format` function.                                    |
| `index.mjs`          | ESM entry.                                                                                                                                                              |
| `index.d.ts`         | Hand-written TypeScript declarations (authoritative — the auto-generated `.napi-generated.d.ts` is never published).                                                    |
| `json.cjs`           | `./json` subpath formatter — canonical ESLint JSON.                                                                                                                     |
| `sarif.cjs`          | `./sarif` subpath formatter — SARIF 2.1.0 for GitHub Code Scanning.                                                                                                     |
| `test/`              | Node `--test` suites (integration + fuzz-regression).                                                                                                                   |
| `ci/`                | Smoke-test harness: `smoke.mjs` (live ESLint run against a fixture tree) and `types-consumer.ts` (downstream TypeScript consumer check).                                |
| `.github/workflows/` | CI (`ci.yml`) and release (`release.yml`) pipelines.                                                                                                                    |
| `docs/`              | `PUBLISHING.md`, `PRIVATE-REPO-PUBLISH.md`.                                                                                                                             |

## Testing

Three layers — all three must pass before a PR is mergeable:

```shell
cargo test        # 76 Rust unit tests (engine + boundary)
node --test       # 65 Node adapter tests (integration + malformed-input fuzz matrix)
ci/smoke.mjs      # ESLint integration: runs ESLint against a fixture tree with -f crisp
```

Property-based tests live alongside the unit tests via the
[`proptest`](https://docs.rs/proptest) crate (exhaustive input generation for
edge-case widths, control-character positions, count coercion). Micro-benchmarks
for the engine hot paths use [`criterion`](https://docs.rs/criterion) and run
under `cargo bench`. Even if you only touch JS, please skim the relevant
`proptest`/`criterion` modules — they document expected behavior better than
prose.

If you add a feature, add tests at the layer where the behavior lives:

- Pure layout/width/color logic → `cargo test` (prefer `proptest` for
  invariants over examples).
- JS adapter / loader / subpath formatters → `node --test`.
- End-to-end rendering against a real ESLint result → `ci/smoke.mjs`.

## Code style

- **`cargo fmt` is enforced.** CI runs `cargo fmt --check`; an unformatted PR
  fails before the test matrix even starts. Run `cargo fmt` locally before
  pushing.
- **Clippy with `-D warnings`.** Any clippy warning fails CI. Do not silence a
  warning with `#[allow(...)]` without explaining why in the PR description.
- **`unsafe` is forbidden.** `Cargo.toml` sets `#![deny(unsafe_code)]`. There
  is no justification for introducing `unsafe` in this codebase — the
  formatter reads JS-provided strings and writes JS-bound strings, nothing
  more.
- **Comments in English.** Doc-comments on every public item (crate, module,
  `pub fn`, `pub struct`, `pub enum`, `pub const`). Private helpers may use
  inline `//` comments where intent is not obvious.
- **JS adapter conventions.** The `.cjs` files target Node ≥ 18 with no
  transpiler; do not introduce TypeScript or ESM-only syntax in them. The
  `.mjs` files mirror the `.cjs` files API-for-API — keep them in sync.

## Commit message conventions

This project follows [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add CRISP_NO_EMOJI toggle for header markers
fix: correct column drift on CJK identifiers in narrow windows
docs: document CRISP_THEME JSON shape in README
chore: bump @napi-rs/cli to 3.10.x
refactor: extract caret-merge logic into its own module
test: add proptest for hostile line/column inputs
```

- One logical change per commit. If a commit needs "and also…", split it.
- The commit body should explain **why** the change is needed, not restate the
  diff.
- **No AI-generated commit messages.** If you used an LLM to draft the code,
  re-author the commit message yourself so it reflects your actual intent.
  Commit messages are part of the project's permanent record — they should
  read like a human wrote them.

## Pull request process

1. Open PRs against the `main` branch.
2. The PR description should answer: *what* changes, *why*, and *how to verify*.
3. CI must pass on the PR before review starts. The matrix is:
   - `cargo fmt --check`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
   - `node --test`
   - `tsc` strict typecheck on the types consumer (`ci/types-consumer.ts`)
   - smoke matrix: ubuntu/windows/macos × node 18/20/22/24
   - a dedicated **ESLint 8.57** job (dual 8/9 support is formally tested, not
     assumed)
   - a stress job that runs ESLint against the 60-file / 240-error fixture
4. Add tests for every behavior change. A PR that only adds code without tests
   will be sent back.
5. Update `CHANGELOG.md` under an `[Unreleased]` section — a one-line entry per
   user-visible change. Note breaking changes explicitly with `BREAKING:`.
6. Do not touch `package.json`'s `version` field. Version bumps are done by
   maintainers at release time (see below).
7. Keep PRs focused. A PR that touches both the Rust engine and the JS adapter
   is fine if the change is inherently cross-cutting; a PR that bundles an
   unrelated refactor with a feature is not.

## Reporting bugs and security issues

- **Bugs** (rendering glitches, broken platforms, performance regressions):
  open a [GitHub Issue](https://github.com/ReactJSLover/eslint-formatter-crisp/issues).
  Use the bug-report issue template and fill in every field — the OS / Node /
  ESLint / crisp version quartet is the difference between a one-shot fix and
  a back-and-forth.
- **Security** (control-char injection, hostile-input crashes, native-load
  abuse): **do not open a public issue.** Follow
  [SECURITY.md](./SECURITY.md) — open a private security advisory at
  <https://github.com/ReactJSLover/eslint-formatter-crisp/security/advisories/new>.
  The supported-versions table and SLA commitments live in that file.

## Releasing

Releases are **maintainer-only**. The full procedure — including the
private-repo bootstrap, cross-build matrix, signature file generation, and npm
publication — is documented in:

- [`docs/PUBLISHING.md`](./docs/PUBLISHING.md) — canonical release runbook.
- [`docs/PRIVATE-REPO-PUBLISH.md`](./docs/PRIVATE-REPO-PUBLISH.md) — bootstrap
  from a private mirror.

The short version: tagging `vX.Y.Z` triggers `.github/workflows/release.yml`,
which cross-builds all seven binaries via `cargo-zigbuild`, emits the
`SHA256SUMS` file, and publishes a fat npm tarball with every binary bundled.
Do not run `npm publish` locally — the published artifacts must come from the
release workflow so the binary matrix and signatures stay reproducible.

## License

By contributing, you agree that your contributions are licensed under the
project's [MIT license](./LICENSE).
