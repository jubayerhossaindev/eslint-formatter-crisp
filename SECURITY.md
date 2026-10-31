# Security Policy

## Supported versions

| Version | Supported          |
| ------- | ------------------ |
| 0.4.x   | :white_check_mark: |
| < 0.4   | :x:                |

## Reporting a vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Open a private security advisory at
<https://github.com/ReactJSLover/eslint-formatter-crisp/security/advisories/new>,
or contact the maintainers directly (see the npm package `author` field).
Include:

- the package version and platform triple (`linux-x64-gnu`, `darwin-arm64`, …),
- a minimal reproduction (fixture + ESLint invocation),
- the impact you observed (terminal rendering, log integrity, crash).

You will receive an acknowledgment within 7 days, a fix or a mitigation plan
within 30 days for confirmed issues, and a public disclosure coordinated with
you once a patched release is out.

## Threat model notes

- The formatter renders data that comes from **linted source code** and from
  **ESLint rule messages**. Source text is treated as untrusted input: control
  characters (C0/C1/DEL, except `\t` and `\n`) are rendered as visible
  width-1 symbols so a hostile repository cannot emit terminal escape
  sequences (cursor movement, fake prompts, log forging) through formatter
  output (shipped in the Phase 1 hardening, see CHANGELOG [0.3.0]).
- Extremely large result sets are bounded: rendered messages are capped by
  `CRISP_MAX_MESSAGES` (default 5000, `0` = unlimited; the summary totals
  stay complete), so a hostile repository cannot balloon formatter time and
  output size without limit.
- Rule documentation URLs from `data.rulesMeta` are embedded into OSC-8
  hyperlink sequences; they are not validated as absolute HTTPS URLs because
  ESLint itself controls that metadata. Callers that forward untrusted
  `rulesMeta` should validate URLs themselves.
- The native binaries are prebuilt and shipped inside the tarball; the npm
  tarball integrity is enforced by the registry (`sha512` in the lockfile).
  Release assets additionally carry a `SHA256SUMS` file starting with v0.3.0.

## Scope

- In scope: `index.cjs`, `index.mjs`, `index.d.ts`, the `./json` and
  `./sarif` subpath formatters, the Rust engine and its N-API boundary,
  packaging metadata, CI.
- Out of scope: ESLint core, terminal emulator vulnerabilities, and any host
  that does not ship a prebuilt binary.
