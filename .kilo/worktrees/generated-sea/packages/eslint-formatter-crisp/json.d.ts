/**
 * eslint-formatter-crisp/json — canonical ESLint JSON formatter types.
 *
 * Zero-dependency emitter producing ESLint's canonical machine-readable
 * output: `JSON.stringify(results, null, 2)`. Accepts the same arguments as
 * any ESLint custom formatter; `data` (rulesMeta) is accepted and ignored.
 * `results` of null/undefined (or any non-array) serializes as `[]`.
 */
declare const jsonFormatter: (results: unknown, data?: unknown) => string;

export = jsonFormatter;
