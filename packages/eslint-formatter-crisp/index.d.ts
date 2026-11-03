/**
 * eslint-formatter-crisp — public type declarations.
 *
 * The package exports a single ESLint custom formatter function plus a few
 * introspection helpers. Types mirror ESLint's LintResult/LintMessage shapes
 * (https://eslint.org/docs/latest/extend/custom-formatters) with the small
 * adaptations documented in the README:
 *  - `msg.fix` is normalized to a boolean at the JS/native boundary,
 *  - `null` is accepted wherever ESLint may omit a field,
 *  - malformed numeric fields (negative/NaN/out-of-range) are coerced to
 *    "absent" instead of wrapping around (v0.2.2+),
 *  - per-file error/warning counts are always recomputed from `messages`
 *    (v0.2.2+), so the summary can never disagree with the rendered list,
 *  - control characters (C0/C1, DEL; except \t and \n) coming from source
 *    text or messages are rendered as visible width-1 symbols (v0.2.2+).
 *
 * NOTE for maintainers: `napi build` REGENERATES this file by default and
 * would clobber the hand-written declarations. The package build scripts
 * therefore pass `--dts .napi-generated.d.ts` so the auto-generated bindings
 * land in a scratch file instead. Never run plain `napi build` here.
 */

/** ESLint severity: 1 = warning, 2 = error. */
export type Severity = 1 | 2;

/** A single lint message, as produced by ESLint (subset used by crisp). */
export interface CrispLintMessage {
  /** Raw message text; quoted identifiers get bolded in the output. */
  message: string;
  severity?: Severity | null;
  /** 1-based. 0 / null / undefined = "no location" (fatal parse errors). */
  line?: number | null;
  column?: number | null;
  ruleId?: string | null;
  fatal?: boolean | null;
  /** ESLint's fix range object; normalized to `true` internally. */
  fix?: unknown;
  /** Any additional ESLint fields are passed through untouched. */
  [key: string]: unknown;
}

/** Per-file lint result, as produced by ESLint (subset used by crisp). */
export interface CrispLintResult {
  filePath: string;
  /** Original source; absent/null when ESLint suppresses it. */
  source?: string | null;
  /** Post-fix source from --fix-dry-run runs (preferred over source). */
  output?: string | null;
  /** Accepted but ignored: counts are recomputed from `messages` (v0.2.2+). */
  errorCount?: number | null;
  warningCount?: number | null;
  messages: CrispLintMessage[];
  /** Any additional ESLint fields are passed through untouched. */
  [key: string]: unknown;
}

/** ESLint's second formatter argument (only rulesMeta is consumed). */
export interface CrispFormatterData {
  rulesMeta?: Record<
    string,
    | {
        docs?: {
          url?: string | null;
          /** Marked rules get the experimental "( deprecated )" suffix
           *  when CRISP_SUGGESTIONS=1 (v0.3.0+). */
          deprecated?: boolean | null;
        } | null;
      }
    | undefined
  >;
  [key: string]: unknown;
}

/** The ESLint custom-formatter signature. */
declare function crisp(results: CrispLintResult[], data?: CrispFormatterData): string;

declare namespace crisp {
  /**
   * Color-level detection with chalk-compatible semantics:
   * FORCE_COLOR > NO_COLOR > TERM=dumb > CI > stream.isTTY.
   * Returns 0 (plain) / 1 (16-color) / 2 (256) / 3 (24-bit).
   */
  function detectColorLevel(stream?: { isTTY?: boolean } | null): 0 | 1 | 2 | 3;

  /** OSC-8 hyperlink decision (isTTY-gated, disabled in CI / TERM=dumb). */
  function detectLinks(stream?: { isTTY?: boolean } | null, colorLevel?: number): boolean;

  /** True when the native binary loaded successfully. */
  function nativeLoaded(): boolean;

  /** Alias of the default export (CJS/ESM interop convenience). */
  const format: typeof crisp;
}

export default crisp;
export { crisp };
/** Alias of the formatter itself (mirrors `module.exports.format` at runtime). */
export const format: typeof crisp;
export const detectColorLevel: typeof crisp.detectColorLevel;
export const detectLinks: typeof crisp.detectLinks;
export const nativeLoaded: typeof crisp.nativeLoaded;
