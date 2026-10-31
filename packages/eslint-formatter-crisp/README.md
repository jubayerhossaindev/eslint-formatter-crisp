# eslint-formatter-crisp

<div align="center">

[![npm version](https://img.shields.io/npm/v/eslint-formatter-crisp.svg?style=for-the-badge&color=blue)](https://www.npmjs.com/package/eslint-formatter-crisp)
[![npm downloads](https://img.shields.io/npm/dm/eslint-formatter-crisp.svg?style=for-the-badge&color=green)](https://www.npmjs.com/package/eslint-formatter-crisp)
[![CI](https://img.shields.io/github/actions/workflow/status/jubayerhossaindev/eslint-formatter-crisp/ci.yml?branch=main&style=for-the-badge&label=CI)](https://github.com/jubayerhossaindev/eslint-formatter-crisp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](LICENSE)
[![Node.js >=18](https://img.shields.io/badge/Node.js-%3E=18-339933?style=for-the-badge&logo=node.js&logoColor=white)](https://nodejs.org/)
[![Rust Powered](https://img.shields.io/badge/Rust-Powered-orange?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)

**Good-lookin' ESLint reports, made crisp.**

A fast, minimal, and beautiful ESLint formatter powered by a Rust core via N-API.
Drop-in replacement for any ESLint workflow — blazing fast performance and crystal-clear output.

</div>

---

## ✨ Features

| Feature | Description |
| --------- | ----------- |
| ⚡ **Blazing Fast** | Rust core rendering with zero JavaScript runtime overhead |
| 🎨 **Beautiful Output** | Clean terminal reports with syntax highlighting, error indicators, and perfect alignment |
| 🔌 **Drop-in Replacement** | Works with any ESLint config — just use `--format crisp` |
| 📦 **Multiple Formats** | Terminal, JSON (`./json`), and SARIF 2.1.0 (`./sarif`) output |
| 🌍 **Cross-Platform** | Prebuilt native binaries for Linux, macOS, and Windows |
| 📘 **TypeScript First** | Full TypeScript declarations (`index.d.ts`, `json.d.ts`, `sarif.d.ts`) |
| 🧠 **Smart Truncation** | Intelligent line truncation with perfect caret alignment |
| 🛡️ **Zero Dependencies** | Lightweight and dependency-free for maximum reliability |

---

## 📦 Installation

Install in the project where ESLint runs:

```sh
npm install --save-dev eslint-formatter-crisp
```

```sh
yarn add --dev eslint-formatter-crisp
```

```sh
pnpm add -D eslint-formatter-crisp
```

**Requirements:** Node.js `>=18`. No Rust toolchain needed — the package bundles prebuilt `.node` binaries.

---

## 🚀 Usage

### CLI

```sh
npx eslint src --format crisp
```

```sh
eslint . --format crisp --max-warnings 0
```

### Programmatic Usage

### CommonJS

```js
const crisp = require('eslint-formatter-crisp');
const report = crisp(results);
console.log(report);
```

### ES Modules

```js
import crisp, { nativeLoaded } from 'eslint-formatter-crisp';

const report = crisp(results);
console.log(nativeLoaded());
```

### JSON & SARIF Output

```js
import 'eslint-formatter-crisp/json';
import 'eslint-formatter-crisp/sarif';
```

The TypeScript declarations live in `index.d.ts`, `json.d.ts`, and `sarif.d.ts`.

---

## ⚙️ Configuration

Control Crisp with environment variables:

| Variable | Purpose | Example |
| --------- | ------- | ------- |
| `FORCE_COLOR` | Force color on/off | `1` / `0` |
| `NO_COLOR` | Disable color (no-color.org) | any value |
| `COLUMNS` | Source excerpt width | `80` |
| `CRISP_FULL_LINES` | Disable truncation | `1` |
| `CRISP_ASCII` | ASCII-only decorations | `1` |
| `CRISP_MAX_MESSAGES` | Max rendered messages (`0` = unlimited) | `3` |
| `CRISP_SUGGESTIONS` | Render fix suggestions (experimental) | `1` |
| `CRISP_THEME` | Theme overrides (JSON) | `{"fruitA":"#001122"}` |
| `MO_THEME` | MO-compatible theme name or JSON | `one-dark-pro` |

---

## 🛠️ API

```ts
interface CrispLintResult {
  filePath: string;
  source?: string | null;
  output?: string | null;
  errorCount: number;
  warningCount: number;
  messages: CrispLintMessage[];
}

interface CrispLintMessage {
  ruleId?: string | null;
  severity: 1 | 2;
  line: number | null;
  column: number | null;
  message: string;
  fatal?: boolean | null;
  fix?: { range: [number, number]; text: string } | null;
  suggestions?: Array<{ desc: string }> | null;
}

declare function crisp(results: CrispLintResult[], data?: CrispFormatterData): string;
declare function crisp(results: CrispLintResult[]): string;

declare const nativeLoaded: () => boolean;
```

---

## 🧪 Development

```sh
# Install dependencies
npm ci

# Run full test suite (Rust + Node)
npm test

# Typecheck
npm run typecheck

# Smoke test native binary
npm run smoke
```

Native source and build configuration are included for maintainers; the published package allowlist excludes Rust development files.

---

## 📄 License

MIT. See [LICENSE](../LICENSE).
