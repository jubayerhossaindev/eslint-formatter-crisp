/**
 * TypeScript consumer fixture — compiled by CI (test/types-consumer.ts job)
 * under node10 / node16 / bundler moduleResolution with --strict.
 *
 * Exercises the public surface of index.d.ts: the default export shape,
 * named exports, and the null-tolerant LintResult/LintMessage fields.
 */
import crisp, { format, detectColorLevel, detectLinks, nativeLoaded } from 'eslint-formatter-crisp';
import type { CrispLintResult, CrispFormatterData } from 'eslint-formatter-crisp';

const results: CrispLintResult[] = [
  {
    filePath: '/work/broken.js',
    source: 'const x = 1\n',
    output: null,
    errorCount: 1,
    warningCount: null,
    messages: [
      {
        message: 'Missing semicolon.',
        severity: 2,
        line: 1,
        column: 11,
        ruleId: 'semi',
        fatal: null,
        fix: { range: [10, 10], text: ';' },
      },
      {
        message: 'Parsing error: boom',
        line: null,
        column: undefined,
        ruleId: null,
      },
    ],
  },
];

const data: CrispFormatterData = {
  rulesMeta: {
    semi: { docs: { url: 'https://eslint.org/docs/latest/rules/semi', deprecated: null } },
    broken: { docs: null },
  },
};

const out: string = crisp(results, data);
const out2: string = format(results);
const level: 0 | 1 | 2 | 3 = detectColorLevel(process.stdout);
const linked: boolean = detectLinks(process.stdout, level);
const ok: boolean = nativeLoaded();

console.log(out.length, out2.length, level, linked, ok);
