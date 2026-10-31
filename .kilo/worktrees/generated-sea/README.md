<div align="center">

# ESLint Formatter Crisp

[![npm version](https://img.shields.io/npm/v/eslint-formatter-crisp.svg?style=for-the-badge&color=blue)](https://www.npmjs.com/package/eslint-formatter-crisp)
[![CI](https://img.shields.io/github/actions/workflow/status/jubayerhossaindev/eslint-formatter-crisp/ci.yml?branch=main&style=for-the-badge&label=CI)](https://github.com/jubayerhossaindev/eslint-formatter-crisp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](LICENSE)
[![Node.js](https://img.shields.io/badge/Node.js-%3E%3D18-339933?style=for-the-badge&logo=node.js&logoColor=white)](https://nodejs.org/)
[![Rust](https://img.shields.io/badge/Rust-Powered-orange?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)

**Good-lookin' ESLint reports, made crisp.**

A fast, minimal, and beautiful ESLint formatter powered by a Rust core via N-API. Drop-in replacement for any ESLint workflow with blazing fast performance and crystal-clear output.

[Installation](#installation) • [Usage](#usage) • [Features](#features) • [API](#api) • [Contributing](#contributing)

</div>

<a id="features"></a>

## ✨ Features

- **⚡ Blazing Fast** — Rust-powered rendering with zero overhead, delivering results instantly even on large codebases
- **🎨 Beautiful Output** — Clean, readable terminal output with syntax highlighting, clear error indicators, and perfect alignment
- **🔌 Drop-in Replacement** — Works seamlessly with any ESLint configuration. Just use `--format crisp`
- **📦 Multiple Formats** — Terminal, JSON, and SARIF outputs for different use cases and CI integrations
- **🌍 Cross-Platform** — Prebuilt native binaries for Linux (GNU & musl), macOS (Intel & Apple Silicon), and Windows (x64)
- **📘 TypeScript First** — Full TypeScript support with comprehensive type definitions
- **🎯 Zero Dependencies** — Lightweight and dependency-free for maximum reliability
- **🧠 Smart Truncation** — Intelligent line truncation with perfect caret alignment

<a id="installation"></a>

## 📦 Installation

```bash
npm install --save-dev eslint-formatter-crisp
```

```bash
yarn add --dev eslint-formatter-crisp
```

```bash
pnpm add -D eslint-formatter-crisp
```

<a id="usage"></a>

## 🚀 Usage

### CLI

```bash
npx eslint src --format crisp
```

```bash
eslint . --format crisp --max-warnings 0
```

### Programmatic Usage

```js
const crisp = require('eslint-formatter-crisp');

const results = await eslint.lintFiles(['src/**/*.js']);
const output = crisp(results);
console.log(output);
```

### ESM

```js
import crisp from 'eslint-formatter-crisp';

const output = crisp(results, data);
console.log(output);
```

### JSON Output

```js
const crispJson = require('eslint-formatter-crisp/json');
console.log(crispJson(results));
```

### SARIF Output

```js
const crispSarif = require('eslint-formatter-crisp/sarif');
console.log(crispSarif(results));
```

Perfect for GitHub Code Scanning and other SARIF-compatible tools.

## 🎛️ Configuration

Control Crisp's behavior with environment variables:

| Variable | Description | Default |
| --- | --- | --- |
| `FORCE_COLOR` | Force color output (`0`, `1`, `2`, or `3`) | Auto-detect |
| `NO_COLOR` | Disable all colors when set | - |
| `CRISP_ASCII` | Use ASCII characters instead of Unicode | `0` |
| `CRISP_FULL_LINES` | Show full lines without truncation | `0` |
| `CRISP_MAX_MESSAGES` | Limit number of messages displayed | Unlimited |
| `CRISP_SUGGESTIONS` | Show fix suggestions (experimental) | `0` |
| `CRISP_THEME` | Custom theme configuration (JSON) | Default |

<a id="api"></a>

## 🛠️ API

### Exports

| Export | Description |
| --- | --- |
| `default` / `crisp` | Main formatter function following ESLint's formatter signature |
| `format` | Alias for the main formatter |
| `nativeLoaded()` | Check if the native Rust module loaded successfully |
| `detectColorLevel()` | Detect terminal color support level |
| `detectLinks()` | Detect terminal hyperlink support |

### Subpath Exports

```js
import crisp from 'eslint-formatter-crisp';           // Terminal formatter
import crispJson from 'eslint-formatter-crisp/json'; // JSON formatter
import crispSarif from 'eslint-formatter-crisp/sarif'; // SARIF 2.1.0 formatter
```

## 🏗️ Architecture

Crisp is built with performance in mind:

- **Rust Core** — All formatting logic is implemented in Rust for maximum speed and safety
- **N-API Bindings** — Seamless integration with Node.js via `napi-rs`
- **Prebuilt Binaries** — Zero compilation required on install with native binaries for all major platforms
- **Fallback Safe** — Gracefully handles missing native modules with clear error messages

## 🧪 Testing

```bash
# Run all tests
npm test

# Run Node.js tests only
npm run test:node

# Run Rust tests only
cargo test --locked
```

<a id="contributing"></a>

## 🤝 Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details on our code of conduct and development process.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the [MIT License](LICENSE).

## 🙏 Acknowledgments

Built with ❤️ using [Rust](https://www.rust-lang.org/), [napi-rs](https://napi.rs/), and [Node.js](https://nodejs.org/).

<div align="center">

**[Report Bug](https://github.com/jubayerhossaindev/eslint-formatter-crisp/issues) • [Request Feature](https://github.com/jubayerhossaindev/eslint-formatter-crisp/issues)**

</div>
