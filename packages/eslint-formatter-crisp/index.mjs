/**
 * eslint-formatter-crisp — ESM entry.
 * ESLint (v9) loads formatters with dynamic import; the CJS default export
 * satisfies its formatter resolution. Named re-exports expose the raw API.
 */
import crisp from './index.cjs';

export default crisp;
export const format = crisp.format ?? crisp;
export const detectColorLevel = crisp.detectColorLevel;
export const detectLinks = crisp.detectLinks;
export const nativeLoaded = crisp.nativeLoaded;
