'use strict';

/**
 * eslint-formatter-crisp — JavaScript adapter.
 *
 * Responsibilities (kept deliberately small; the formatting engine is Rust):
 *  1. Load the native binary for the current platform with clear errors.
 *  2. Detect color level and hyperlink support (chalk/terminal-link semantics).
 *  3. Expose the ESLint custom-formatter interface (CJS + ESM interop).
 *
 * Input contract: the ESLint formatter signature is
 *   formatter(results: LintResult[], data?: LintResultData) => string
 * The adapter forwards `results`/`data` to the native `format()` after
 * normalizing `msg.fix` to a boolean (the only shape adaptation).
 */

const os = require('node:os');
const fs = require('node:fs');
const path = require('node:path');

// ---------------------------------------------------------------------------
// 1. Native module loading
// ---------------------------------------------------------------------------

// All platforms shipped inside this fat package as of v0.2.0:
//   linux  x64  gnu   (verified host)
//   linux  x64  musl  (Alpine, verified via cross-build)
//   linux  arm64 gnu  (cross-built)
//   linux  arm64 musl (Alpine, cross-built)
//   darwin x64        (Intel macOS)
//   darwin arm64      (Apple Silicon)
//   win32  x64  msvc  (the .dll is built with the mingw toolchain but the
//                      runtime loads N-API symbols via libloading, so the
//                      MSVC runtime ABI on the host is fine)
const SUPPORTED_TRIPLES = [
  'linux-x64-gnu',
  'linux-x64-musl',
  'linux-arm64-gnu',
  'linux-arm64-musl',
  'darwin-x64',
  'darwin-arm64',
  'win32-x64-msvc',
];

function currentTriples() {
  const arch = os.arch();
  const platform = process.platform;
  const triples = [];
  if (platform === 'linux') {
    // Try glibc first, then musl (Alpine). Cheap and deterministic.
    triples.push(`linux-${arch}-gnu`);
    triples.push(`linux-${arch}-musl`);
  } else if (platform === 'darwin') {
    triples.push(`darwin-${arch}`);
  } else if (platform === 'win32') {
    triples.push(`win32-${arch}-msvc`);
  }
  return triples;
}

let native = null;
let loadError = null;

// Try each candidate triple in order. Existence alone isn't enough on Linux:
// a glibc-built binary may exist on disk but fail to dlopen on Alpine (where
// the glibc compat layer is missing). Fall through to the next candidate on
// actual require() failure, not just on file-not-found.
//
// Extracted as a pure function so the fallback order is unit-testable with
// injected existsFn/requireFn mocks (Phase 2: loader-fallback regression
// test in test/node.test.mjs).
function resolveNative(candidates, existsFn, requireFn) {
  const result = { native: null, loadError: null, tried: [], failed: [] };
  for (const candidate of candidates) {
    if (!existsFn(candidate)) continue;
    result.tried.push(candidate);
    try {
      result.native = requireFn(candidate);
      break;
    } catch (err) {
      result.failed.push(candidate);
      result.loadError = err;
      // Continue to next candidate (e.g. gnu failed on Alpine -> try musl).
    }
  }
  if (!result.native && !result.loadError && candidates.length > 0) {
    result.loadError = new Error(
      `no prebuilt native binary for ${process.platform}-${os.arch()}. ` +
        `Tried: ${candidates.join(', ')}. ` +
        `Supported platforms: ${SUPPORTED_TRIPLES.join(', ')}.`,
    );
  }
  return result;
}

const candidates = currentTriples().map((triple) => path.join(__dirname, `crisp.${triple}.node`));
const resolved = resolveNative(candidates, fs.existsSync, require);
native = resolved.native;
loadError = resolved.loadError;

function assertLoaded() {
  if (!native) {
    const err = new Error(
      `eslint-formatter-crisp: failed to load the native module. ${loadError && loadError.message ? loadError.message : ''}`,
    );
    err.cause = loadError;
    err.code = 'ERR_CRISP_NATIVE_LOAD';
    throw err;
  }
}

// ---------------------------------------------------------------------------
// 2. Color / hyperlink detection (chalk- and terminal-link-compatible subset)
// ---------------------------------------------------------------------------

// Precedence (chalk / supports-color compatible; v0.4.0 documents the full
// chain and unit-freezes it in test/node.test.mjs):
//   1. FORCE_COLOR  - standard override. '0'/'false' force color OFF,
//                     '1'/'true'/'' force 16-color, '2' 256-color,
//                     '3' 24-bit. Beats EVERY other signal, including
//                     NO_COLOR and TTY sniffing; anything unparsable -> off.
//   2. NO_COLOR     - https://no-color.org: any non-empty value -> off.
//   3. TERM=dumb    - off.
//   4. CI           - 16-color (CI logs keep their decoration).
//   5. stream.isTTY - 16-color on a terminal; piped output stays plain.
function detectColorLevel(stream) {
  const force = process.env.FORCE_COLOR;
  if (force !== undefined) {
    // An explicit FORCE_COLOR short-circuits all remaining detection.
    if (force === '' || force === 'true' || force === '1') return 1;
    if (force === '2') return 2;
    if (force === '3') return 3;
    // '0', 'false', and anything unparsable disable color outright.
    return 0;
  }
  if (process.env.NO_COLOR) return 0;
  if (process.env.TERM === 'dumb') return 0;
  if (process.env.CI) return 1;
  if (stream && stream.isTTY) return 1;
  return 0;
}

function detectLinks(stream, colorLevel) {
  // Parity verified in Phase 1: the reference emits no OSC-8 when piped
  // (experiments s1/s10). Approximation of supports-hyperlinks.
  if (colorLevel === 0) return false;
  if (process.env.CI) return false;
  if (process.env.TERM === 'dumb') return false;
  if (!stream || !stream.isTTY) return false;
  return true;
}

// Terminal width source (v0.3.0, audit roadmap Phase 3):
//   1. `CRISP_FULL_LINES=1`  -> unlimited (escape hatch, historical layout)
//   2. `COLUMNS` env          -> explicit user intent (honored even when
//                                piped, which also makes it CI/testable)
//   3. TTY size               -> process.stdout.columns on a real terminal
//   4. otherwise              -> unlimited (piped/redirected output keeps
//                                the byte-compatible historical layout)
// Widths below 20 columns are ignored — wrapping below that hurts more
// than it helps.
function detectMaxWidth(stream) {
  if (process.env.CRISP_FULL_LINES === '1') return undefined;
  const raw = process.env.COLUMNS;
  if (raw !== undefined && raw !== '') {
    const n = Number(raw);
    if (Number.isInteger(n) && n >= 20 && n <= 10000) return n;
    // invalid COLUMNS falls through to TTY detection rather than disabling
    // the width feature entirely
  }
  if (
    stream &&
    stream.isTTY &&
    typeof stream.columns === 'number' &&
    stream.columns >= 20 &&
    stream.columns <= 10000
  ) {
    return stream.columns;
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// 3. Input hygiene (Phase 1 hardening: audit findings P1-P4)
// ---------------------------------------------------------------------------

// Regex matching any control character we render as a visible symbol:
//   C0 range minus \t (0x09) and \n (0x0A)  -> U+2400..U+241F (␀..␟)
//   DEL (0x7F)                              -> U+2421 (␡)
//   C1 range (0x80-0x9F)                    -> U+FFFD (replacement)
// Every replacement is exactly ONE character of display width 1, so line/
// column math (caret alignment, indicator positions) is preserved bit-for-bit.
// Rationale (audit finding P1): raw ESC/CSI/BEL/CR bytes coming from linted
// SOURCE text (or hostile rule messages) must never reach the terminal —
// they enable tool-output injection (cursor movement, fake prompts, log
// forging). The reference formatters strip them; crisp renders them as
// visible, width-stable symbols so the code frame still shows that the
// character is THERE — just neutered.
// eslint-disable-next-line no-control-regex -- intentional: this IS the control-char matcher
const CONTROL_RE = /[\u0000-\u0008\u000B-\u001F\u007F-\u009F]/g;

function sanitizeControl(s) {
  if (typeof s !== 'string') return s;
  CONTROL_RE.lastIndex = 0;
  if (!CONTROL_RE.test(s)) return s; // fast path: nothing to do
  return s.replace(CONTROL_RE, (ch) => {
    const code = ch.charCodeAt(0);
    if (code === 0x7f) return '\u2421'; // ␡ SYMBOL FOR DELETE
    if (code <= 0x1f) return String.fromCharCode(0x2400 + code); // ␀..␟
    return '\uFFFD'; // C1 range has no symbol block
  });
}

// Clamp a JS number into the u32 domain napi expects. Non-finite, negative,
// and out-of-range values become `undefined` (Rust Option -> 0), never the
// silent wrap/truncate napi would otherwise perform (audit findings P3:
// -3 -> 4294967293, 1e21 -> 3735027712).
function toU32(v) {
  if (typeof v !== 'number' || !Number.isFinite(v)) return undefined;
  if (v < 0 || v > 0xffffffff) return undefined;
  return Math.floor(v);
}

// Same contract for severity (Rust Option<i32>): keep ESLint's 1/2, coerce
// anything else out of the way (engine treats absent as warning).
function toSeverity(v) {
  if (typeof v !== 'number' || !Number.isFinite(v)) return undefined;
  const s = Math.floor(v);
  return s >= 0 && s <= 0x7fffffff ? s : undefined;
}

const norm = (v) => (v === null || v === undefined ? undefined : v);

// Normalize one LintMessage: null-tolerant field coercion + P1 sanitization.
// When `wantSuggestions` is set (CRISP_SUGGESTIONS=1), the first fix
// suggestion's description is extracted for the experimental display.
function normalizeMessage(m, resultIndex, msgIndex, wantSuggestions) {
  if (m === null || typeof m !== 'object' || Array.isArray(m)) {
    throw new TypeError(
      `eslint-formatter-crisp: results[${resultIndex}].messages[${msgIndex}] must be a LintMessage object, got ${m === null ? 'null' : typeof m}`,
    );
  }
  if (typeof m.message !== 'string') {
    throw new TypeError(
      `eslint-formatter-crisp: results[${resultIndex}].messages[${msgIndex}].message must be a string, got ${typeof m.message}`,
    );
  }
  let suggestion;
  if (wantSuggestions && Array.isArray(m.suggestions)) {
    const first = m.suggestions[0];
    if (first && typeof first === 'object' && typeof first.desc === 'string') {
      suggestion = sanitizeControl(first.desc);
    }
  }
  return {
    ...m,
    message: sanitizeControl(m.message),
    ruleId: sanitizeControl(norm(m.ruleId)),
    severity: toSeverity(m.severity),
    line: toU32(m.line),
    column: toU32(m.column),
    fatal: norm(m.fatal),
    fix: m.fix ? true : undefined,
    suggestion,
  };
}

// Normalize the second formatter argument (rulesMeta). napi-rs rejects JS
// `null` for nested object fields (audit findings A20/A21/P2), so every node
// is rebuilt: only well-shaped metadata survives.
function normalizeData(data) {
  if (!data || typeof data !== 'object') return { rulesMeta: {} };
  const rulesMeta = data.rulesMeta;
  if (rulesMeta === null || rulesMeta === undefined) return { rulesMeta: {} };
  if (typeof rulesMeta !== 'object') return { rulesMeta: {} };
  const clean = {};
  for (const key of Object.keys(rulesMeta)) {
    const meta = rulesMeta[key];
    if (meta === null || typeof meta !== 'object') continue;
    const docs = meta.docs;
    if (docs === null || docs === undefined || typeof docs !== 'object') continue;
    const cleanDocs = {};
    if (typeof docs.url === 'string') cleanDocs.url = docs.url;
    if (docs.deprecated === true) cleanDocs.deprecated = true;
    clean[key] = { docs: cleanDocs };
  }
  return { rulesMeta: clean };
}

// ---------------------------------------------------------------------------
// Experimental themes (`CRISP_THEME` / `MO_THEME`, v0.3.0+)
// ---------------------------------------------------------------------------

// Accepts a JSON object with optional keys:
//   fruitA/fruitB/cristalA/cristalB  -> `#rrggbb` gradient stop overrides
//   mono: true                       -> disable syntax-highlight token colors
// Also accepts mo-compatible theme NAMES for drop-in migration:
//   'one-dark-pro' (the crisp default — no-op) and 'mono'. Unknown names
//   fall back to the default palette (mo shipped full Shiki themes; crisp
//   stays tiny and maps what it can).
function parseThemeJson(raw) {
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return undefined;
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return undefined;
  }
  const hex = (v) =>
    typeof v === 'string' && /^#?[0-9a-fA-F]{6}$/.test(v) ? v.replace('#', '') : undefined;
  const out = {};
  for (const key of ['fruitA', 'fruitB', 'cristalA', 'cristalB']) {
    const v = hex(parsed[key]);
    if (v) out[key] = v;
  }
  if (parsed.mono === true) out.mono = true;
  return Object.keys(out).length > 0 ? out : undefined;
}

const THEME_NAMES = {
  'one-dark-pro': undefined, // crisp's built-in palette is one-dark-pro based
  mono: { mono: true },
};

function detectTheme() {
  const raw = process.env.CRISP_THEME ?? process.env.MO_THEME;
  if (raw === undefined || raw === '') return undefined;
  if (raw.trimStart().startsWith('{')) {
    return parseThemeJson(raw);
  }
  return THEME_NAMES[raw];
}

// v0.4.0: render-cap passthrough (CRISP_MAX_MESSAGES).
//
// The Rust core renders at most N messages per run and appends a truncation
// notice; the summary always totals ALL problems, so counts and the rendered
// list can never disagree. Contract (strict by design):
//   /^\d+$/ -> the value verbatim, INCLUDING "0" (0 = unlimited on the
//              Rust side)
//   anything else (absent, '', 'abc', '-1', '3.5', ' 3', '+3', values above
//              the u32 domain) -> undefined, i.e. the field is OMITTED from
//              the options object and the Rust default applies (5000 total).
// Garbage env input never throws and never reaches the napi boundary.
function detectMaxMessages() {
  const raw = process.env.CRISP_MAX_MESSAGES;
  if (raw === undefined || !/^\d+$/.test(raw)) return undefined;
  const n = Number(raw);
  if (n > 0xffffffff) return undefined; // outside the napi u32 domain
  return n;
}

// ---------------------------------------------------------------------------
// 4. ESLint formatter interface
// ---------------------------------------------------------------------------

function formatResults(results, data) {
  assertLoaded();

  if (!Array.isArray(results)) {
    throw new TypeError(
      `eslint-formatter-crisp: expected an array of ESLint LintResult objects, got ${typeof results}`,
    );
  }

  const colorLevel = detectColorLevel(process.stdout);
  const suggestions = process.env.CRISP_SUGGESTIONS === '1';
  const options = {
    colorLevel,
    baseDir: process.cwd(),
    links: detectLinks(process.stdout, colorLevel),
    // Pure-ASCII decoration mode: for terminals whose encoding cannot
    // render the Unicode glyphs (e.g. GBK consoles where `─` shows up as
    // CJK mojibake). Widths are preserved; code content is untouched.
    ascii: process.env.CRISP_ASCII === '1',
    // v0.3.0: COLUMNS/TTY-aware truncation + message wrap (see above).
    maxWidth: detectMaxWidth(process.stdout),
    // v0.3.0 experimental: fix suggestions + deprecated rule marker.
    suggestions,
    // v0.3.0 experimental color overrides (CRISP_THEME > MO_THEME).
    theme: detectTheme(),
    // v0.4.0: optional message cap (CRISP_MAX_MESSAGES). The Rust core
    // renders at most N messages ("0" = unlimited) and appends a truncation
    // notice while the summary still totals every problem; when omitted the
    // Rust default applies (5000 total). Garbage values are dropped by
    // detectMaxMessages(), never forwarded to the napi boundary.
    maxMessages: detectMaxMessages(),
  };

  // Boundary hygiene (Phase 1, audit P1-P4):
  //  - every rendered string is control-char sanitized (P1)
  //  - line/column/counts/severity coerced into the napi numeric domain (P2/P3)
  //  - null/non-object array elements get the same friendly TypeError class
  //    as a non-array argument (P4)
  //  - fix normalized to boolean; nulls become undefined (napi rejects null)
  const normalized = results.map((r, resultIndex) => {
    if (r === null || typeof r !== 'object' || Array.isArray(r)) {
      throw new TypeError(
        `eslint-formatter-crisp: results[${resultIndex}] must be a LintResult object, got ${r === null ? 'null' : Array.isArray(r) ? 'array' : typeof r}`,
      );
    }
    const messages = Array.isArray(r.messages) ? r.messages : [];
    return {
      filePath: sanitizeControl(r.filePath),
      source: sanitizeControl(norm(r.source)),
      output: sanitizeControl(norm(r.output)),
      errorCount: toU32(r.errorCount),
      warningCount: toU32(r.warningCount),
      messages: messages.map((m, msgIndex) =>
        normalizeMessage(m, resultIndex, msgIndex, suggestions),
      ),
    };
  });

  const dataArg = normalizeData(data);
  return native.format(normalized, dataArg, options);
}

module.exports = formatResults;
module.exports.default = formatResults;
module.exports.format = formatResults;
module.exports.detectColorLevel = detectColorLevel;
module.exports.detectLinks = detectLinks;
module.exports.nativeLoaded = () => native !== null;

// Exported for unit tests (Phase 2: fuzz-case regression freeze). Not part
// of the public contract; may change in minor versions.
Object.defineProperty(module.exports, '_internal', {
  value: Object.freeze({
    sanitizeControl,
    toU32,
    toSeverity,
    normalizeData,
    normalizeMessage,
    resolveNative,
    currentTriples,
    detectMaxWidth,
    parseThemeJson,
    detectTheme,
    // v0.4.0: color precedence + message-cap parsing unit-frozen in tests.
    detectColorLevel,
    detectMaxMessages,
  }),
  enumerable: false,
});
