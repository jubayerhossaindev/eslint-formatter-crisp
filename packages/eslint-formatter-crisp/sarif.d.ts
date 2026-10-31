/**
 * eslint-formatter-crisp/sarif — SARIF 2.1.0 emitter types.
 *
 * Zero-dependency emitter producing a single-run SARIF 2.1.0 document
 * (`$schema: https://json.schemastore.org/sarif-2.1.0.json`, 2-space
 * pretty-printed JSON) consumable by GitHub code scanning and SARIF viewers.
 * Accepts the same arguments as any ESLint custom formatter; `data.rulesMeta`
 * feeds rule `helpUri` entries when present.
 */
declare const sarifFormatter: (results: unknown, data?: unknown) => string;

export = sarifFormatter;
