'use strict';

/**
 * eslint-formatter-crisp/sarif — zero-dependency SARIF 2.1.0 emitter.
 *
 * Produces a single-run SARIF 2.1.0 document (2-space pretty-printed JSON)
 * consumable by GitHub code scanning, SARIF viewers, and CI dashboards:
 *
 * {
 *   "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
 *   "version": "2.1.0",
 *   "runs": [{
 *     "tool": { "driver": {
 *       "name": "eslint",
 *       "informationUri": "https://eslint.org",
 *       "rules": [ { "id": "<ruleId>", "helpUri": "<docs url>" } ]
 *     }},
 *     "results": [{
 *       "ruleId": "<ruleId>",         // omitted for null-ruleId parse errors
 *       "ruleIndex": 0,               // omitted together with ruleId
 *       "level": "error" | "warning", // severity 2 / 1; fatal -> "error"
 *       "message": { "text": "..." },
 *       "locations": [{
 *         "physicalLocation": {
 *           "artifactLocation": { "uri": "src/a.js" },  // cwd-relative POSIX
 *           "region": { "startLine": 1, "startColumn": 1 }
 *         }
 *       }]
 *     }]                              // present even when empty
 *   }]
 * }
 *
 * Defensive contract (ESLint custom formatters must never throw on hand-rolled
 * input): `results` that is null/undefined/non-array is treated as an empty
 * list; malformed message entries are skipped; missing line/column fall back
 * to 1; non-string message text renders as ''. `data.rulesMeta[r].docs.url`
 * feeds the rule `helpUri` when present.
 */

const path = require('node:path');

const SARIF_SCHEMA = 'https://json.schemastore.org/sarif-2.1.0.json';
const ESLINT_INFORMATION_URI = 'https://eslint.org';

// Convert an ESLint filePath into a cwd-relative POSIX-style URI suitable for
// artifactLocation.uri (SARIF resolves it against the run's originalUriBase).
// Pure function (cwd/platform injected) so the win32 backslash normalization
// is unit-testable on every host:
//   - path.relative() against the ESLint cwd (ESLint reports absolute paths);
//   - backslashes -> '/' when the host is win32, and also on other platforms
//     when the relative result still contains backslashes (a hand-rolled
//     windows-style path), because backslashes are invalid in URI references;
//   - non-string/empty/unrelativizable inputs -> undefined (caller omits the
//     artifactLocation instead of emitting a broken URI).
function toRelativePosixUri(filePath, cwd, platform) {
  if (typeof filePath !== 'string' || filePath === '') return undefined;
  const isWin = (platform === undefined ? process.platform : platform) === 'win32';
  let rel;
  try {
    rel = path.relative(typeof cwd === 'string' && cwd !== '' ? cwd : process.cwd(), filePath);
  } catch {
    return undefined;
  }
  if (rel === '') return undefined;
  if (isWin || rel.includes('\\')) {
    rel = rel.split('\\').join('/');
  }
  return rel;
}

// data.rulesMeta[ruleId].docs.url, defensively; undefined when absent.
function helpUriFor(data, ruleId) {
  if (!data || typeof data !== 'object') return undefined;
  const rulesMeta = data.rulesMeta;
  if (!rulesMeta || typeof rulesMeta !== 'object') return undefined;
  const meta = rulesMeta[ruleId];
  if (!meta || typeof meta !== 'object') return undefined;
  const docs = meta.docs;
  if (!docs || typeof docs !== 'object') return undefined;
  return typeof docs.url === 'string' && docs.url !== '' ? docs.url : undefined;
}

// Only ESLint severity 1 maps to "warning"; everything else — severity 2,
// fatal parse errors (severity 2 with `fatal: true`), absent/odd severities —
// is reported as "error" so nothing under-reports.
function levelFor(message) {
  if (message && typeof message === 'object' && message.fatal !== true && message.severity === 1) {
    return 'warning';
  }
  return 'error';
}

// SARIF regions are 1-based; ESLint's 0 / null / undefined (parse errors)
// clamp to the first cell.
function regionFor(message) {
  const num = (v) => (typeof v === 'number' && Number.isFinite(v) && v > 0 ? Math.floor(v) : 1);
  return { startLine: num(message.line), startColumn: num(message.column) };
}

// Build the full SARIF document object. `cwd`/`platform` are injectable for
// tests; production callers use process.cwd()/process.platform.
function buildSarif(results, data, cwd, platform) {
  const list = Array.isArray(results) ? results : [];

  // Pass 1: collect unique ruleIds in first-seen order (stable output).
  // Messages with a null/non-string ruleId (parsing errors) never create
  // rule entries.
  const ruleIds = [];
  const seen = new Set();
  for (const result of list) {
    if (!result || typeof result !== 'object' || !Array.isArray(result.messages)) {
      continue;
    }
    for (const message of result.messages) {
      const id =
        message &&
        typeof message === 'object' &&
        typeof message.ruleId === 'string' &&
        message.ruleId !== ''
          ? message.ruleId
          : null;
      if (id !== null && !seen.has(id)) {
        seen.add(id);
        ruleIds.push(id);
      }
    }
  }
  const rules = ruleIds.map((id) => {
    const helpUri = helpUriFor(data, id);
    return helpUri === undefined ? { id } : { id, helpUri };
  });
  const ruleIndex = new Map(ruleIds.map((id, i) => [id, i]));

  // Pass 2: map messages to SARIF results.
  const sarifResults = [];
  for (const result of list) {
    if (!result || typeof result !== 'object' || !Array.isArray(result.messages)) {
      continue;
    }
    const uri = toRelativePosixUri(result.filePath, cwd, platform);
    for (const message of result.messages) {
      if (!message || typeof message !== 'object') continue;
      const entry = {};
      const id =
        typeof message.ruleId === 'string' && message.ruleId !== '' ? message.ruleId : null;
      if (id !== null) {
        entry.ruleId = id;
        const idx = ruleIndex.get(id);
        if (idx !== undefined) entry.ruleIndex = idx;
      }
      entry.level = levelFor(message);
      entry.message = {
        text: typeof message.message === 'string' ? message.message : '',
      };
      const physicalLocation = {};
      if (uri !== undefined) {
        physicalLocation.artifactLocation = { uri };
      }
      physicalLocation.region = regionFor(message);
      entry.locations = [{ physicalLocation }];
      sarifResults.push(entry);
    }
  }

  return {
    $schema: SARIF_SCHEMA,
    version: '2.1.0',
    runs: [
      {
        tool: {
          driver: {
            name: 'eslint',
            informationUri: ESLINT_INFORMATION_URI,
            rules,
          },
        },
        results: sarifResults,
      },
    ],
  };
}

function sarifFormatter(results, data) {
  return JSON.stringify(buildSarif(results, data), null, 2);
}

module.exports = sarifFormatter;

// Exported for unit tests (mirrors index.cjs's _internal convention). Not
// part of the public contract; may change in minor versions.
Object.defineProperty(module.exports, '_internal', {
  value: Object.freeze({ buildSarif, toRelativePosixUri }),
  enumerable: false,
});
