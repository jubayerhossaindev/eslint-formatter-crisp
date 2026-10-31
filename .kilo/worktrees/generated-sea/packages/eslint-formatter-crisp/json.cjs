'use strict';

/**
 * eslint-formatter-crisp/json — canonical ESLint JSON output.
 *
 * Zero-dependency drop-in for ESLint's built-in `json` formatter: stable,
 * machine-consumable `JSON.stringify(results, null, 2)`. The output is the
 * raw ESLint `results` array verbatim (every field preserved, key order
 * stable) so downstream tools can parse it byte-compatibly with the core
 * formatter.
 *
 * Defensive contract: `results` that is null/undefined (or otherwise not an
 * array) is treated as an empty list, so the output always parses. The
 * second `data` argument (rulesMeta) is accepted — ESLint always passes it —
 * and deliberately ignored; rule metadata must never leak into machine
 * output.
 *
 * Usage: eslint --format node_modules/eslint-formatter-crisp/json.cjs
 * (or `--format node_modules/eslint-formatter-crisp/json` once published).
 */

module.exports = function jsonFormatter(results) {
  return JSON.stringify(Array.isArray(results) ? results : [], null, 2);
};
