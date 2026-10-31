/**
 * Fuzz-case regression freeze (Phase 2, audit roadmap).
 *
 * The 26-case malformed-input matrix from the deep audit (Task 9,
 * scripts/fuzz_formatter.cjs cases A01-A22 + W1-W4) is frozen here as the
 * permanent behavior contract, plus the Phase 1 hardening assertions
 * (P1 sanitizer / P2 null-tolerance / P3 clamp / P4 friendly errors /
 * P6 count recompute) and the loader-fallback unit test.
 *
 * Run: node --test (auto-discovered together with test/node.test.mjs)
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const require = createRequire(import.meta.url);
const pkgRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const crisp = require(path.join(pkgRoot, 'index.cjs'));
const { sanitizeControl, toU32, toSeverity, normalizeData, resolveNative, currentTriples } =
  crisp._internal;

const NO_C0 = /[\u0000-\u0008\u000B-\u001F\u007F-\u009F]/; // eslint-disable-line no-control-regex

function errMsg(fn) {
  try {
    fn();
    return null;
  } catch (e) {
    return e.message;
  }
}

// ---------------------------------------------------------------------------
// A-series: ESLint never sends these, but a hand-rolled caller might.
// Contract: no raw crashes; either friendly TypeError or sensible output.
// ---------------------------------------------------------------------------

test('A01 empty array -> empty string', () => {
  assert.equal(crisp([], undefined), '');
});

test('A02/A03 non-array results -> friendly TypeError (designed error)', () => {
  for (const bad of [null, 'nope', 42, undefined]) {
    assert.match(
      errMsg(() => crisp(bad, undefined)),
      /expected an array of ESLint LintResult/,
    );
  }
});

test('A04 results=[null] -> friendly TypeError with index (P4)', () => {
  assert.match(
    errMsg(() => crisp([null], undefined)),
    /results\[0\] must be a LintResult object, got null/,
  );
  assert.match(
    errMsg(() => crisp([{ filePath: '/a.js' }, undefined], undefined)),
    /results\[1\] must be a LintResult object/,
  );
});

test('A04b/c null message element / missing message text -> friendly TypeError (P4)', () => {
  assert.match(
    errMsg(() => crisp([{ filePath: '/a.js', messages: [null] }], undefined)),
    /messages\[0\] must be a LintMessage object/,
  );
  assert.match(
    errMsg(() =>
      crisp(
        [{ filePath: '/a.js', messages: [{ ruleId: 'r', severity: 2, line: 1, column: 1 }] }],
        undefined,
      ),
    ),
    /\.message must be a string/,
  );
});

test('A05/A06 missing or null messages -> treated as clean/empty, no throw (P2)', () => {
  assert.equal(crisp([{ filePath: '/a.js' }], undefined), '');
  assert.equal(crisp([{ filePath: '/a.js', messages: null }], undefined), '');
});

test('A07/A08 weird severities (3, 99) -> render as warning, never crash', () => {
  for (const severity of [3, 99]) {
    const out = crisp(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', severity, line: 1, column: 1, message: 'x' }],
        },
      ],
      undefined,
    );
    assert.ok(out.includes('⚠'), `severity ${severity} rendered as warning`);
  }
});

test('A09/A10 negative / huge line -> no location fallback row (P2/P3)', () => {
  for (const line of [-5, 1e21, null, NaN]) {
    const out = crisp(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', severity: 2, line, column: 1, message: 'loc-less' }],
        },
      ],
      undefined,
    );
    assert.ok(out.includes('loc-less'));
    assert.ok(!out.includes(' 1: '), 'no code frame for a coerced-away location');
  }
});

test('A11 fatal parse error (line 0) -> red parse-error row', () => {
  const out = crisp(
    [
      {
        filePath: '/a.js',
        messages: [
          {
            ruleId: null,
            severity: 2,
            line: 0,
            column: 0,
            fatal: true,
            message: 'Parsing error: boom',
          },
        ],
      },
    ],
    undefined,
  );
  // v0.5.3: parse error renders as `└─ ✖` + `  Parsing error: boom ...`
  assert.ok(out.includes('✖'), `✖ icon present: ${out}`);
  assert.ok(out.includes('Parsing error: boom'), `message present: ${out}`);
});

test('A12 line/column null -> no throw (P2)', () => {
  const out = crisp(
    [
      {
        filePath: '/a.js',
        messages: [
          { ruleId: 'r', severity: 2, line: null, column: null, message: 'null location' },
        ],
      },
    ],
    undefined,
  );
  assert.ok(out.includes('null location'));
});

test('A13 multiline message -> no throw, all lines present', () => {
  const out = crisp(
    [
      {
        filePath: '/a.js',
        source: 'x = 1;',
        messages: [
          { ruleId: 'r', severity: 2, line: 1, column: 1, message: 'line1\nline2\nline3' },
        ],
      },
    ],
    undefined,
  );
  assert.ok(out.includes('line1') && out.includes('line2') && out.includes('line3'));
});

test('A14 empty message -> no throw', () => {
  const out = crisp(
    [
      {
        filePath: '/a.js',
        messages: [{ ruleId: 'r', severity: 2, line: 1, column: 1, message: '' }],
      },
    ],
    undefined,
  );
  assert.ok(typeof out === 'string');
});

test('A15 fix as full range object -> auto-fixable (normalized to boolean)', () => {
  const out = crisp(
    [
      {
        filePath: '/a.js',
        messages: [
          {
            ruleId: 'r',
            severity: 2,
            line: 1,
            column: 1,
            message: 'fix me',
            fix: { range: [0, 1], text: 'y' },
          },
        ],
      },
    ],
    undefined,
  );
  assert.ok(out.includes('auto-fixable'));
});

test('A16/A17/A18 hostile counts (float, negative, 1e21) -> sane output (P3)', () => {
  // Floats floor toward the message truth; empty messages always win, so the
  // file is skipped entirely instead of printing fabricated large numbers.
  assert.equal(
    crisp([{ filePath: '/a.js', errorCount: 1.5, warningCount: 2.7, messages: [] }], undefined),
    '',
  );
  const out17 = crisp(
    [{ filePath: '/a.js', errorCount: -3, warningCount: -1, messages: [] }],
    undefined,
  );
  assert.equal(out17, '');
  assert.ok(!out17.includes('4294967293'), 'no u32 wrap');
  const out18 = crisp(
    [{ filePath: '/a.js', errorCount: 1e21, warningCount: 0, messages: [] }],
    undefined,
  );
  assert.equal(out18, '');
  assert.ok(!out18.includes('3735027712'), 'no silent truncation');
});

test('A19/A20/A21 null data / null docs.url / null rulesMeta -> no throw (P2)', () => {
  const results = [
    {
      filePath: '/a.js',
      messages: [{ ruleId: 'my-rule', severity: 2, line: 1, column: 1, message: 'x' }],
    },
  ];
  assert.ok(typeof crisp(results, null) === 'string');
  assert.ok(
    typeof crisp(results, { rulesMeta: { 'my-rule': { docs: { url: null } } } }) === 'string',
  );
  assert.ok(typeof crisp(results, { rulesMeta: null }) === 'string');
});

test('A22 header counts recomputed from messages, never trusted (P6)', () => {
  const out = crisp(
    [
      {
        filePath: '/a.js',
        errorCount: 1,
        warningCount: 0,
        messages: [
          { ruleId: 'r1', severity: 2, line: 1, column: 1, message: 'a' },
          { ruleId: 'r2', severity: 2, line: 2, column: 1, message: 'b' },
          { ruleId: 'r3', severity: 1, line: 3, column: 1, message: 'c' },
        ],
      },
    ],
    undefined,
  );
  assert.ok(out.includes('2 errors'), 'header shows recomputed error count');
  assert.ok(out.includes('1 warning'), 'header shows recomputed warning count');
});

// ---------------------------------------------------------------------------
// P1: control-character sanitizer (injection defense)
// ---------------------------------------------------------------------------

test('P1 no raw C0/C1/DEL byte ever reaches the output', () => {
  // Force plain output so ANSI decoration (which emits raw ESC) cannot
  // mask or trigger this content-hygiene assertion, regardless of ambient
  // CI / TTY / FORCE_COLOR. NO_COLOR is checked before CI in
  // detectColorLevel, but FORCE_COLOR short-circuits first, so neutralize both.
  const prevNoColor = process.env.NO_COLOR;
  const prevForce = process.env.FORCE_COLOR;
  process.env.NO_COLOR = '1';
  delete process.env.FORCE_COLOR;
  try {
    const out = crisp(
      [
        {
          filePath: '/a\x1b[31m.js',
          source: 'const evil = "x\x1b[31mRED\x1b[0m\x1b BEL\rX";',
          messages: [
            {
              ruleId: 'r\x1b',
              severity: 2,
              line: 1,
              column: 12,
              message: 'injected\x07BEL\x1b[2Jclear',
            },
          ],
          errorCount: 1,
          warningCount: 0,
        },
      ],
      undefined,
    );
    assert.ok(!NO_C0.test(out), 'found raw control bytes in output');
    assert.ok(out.includes('\u241b'), 'ESC shown as visible symbol');
    assert.ok(out.includes('\u2407'), 'BEL shown as visible symbol');
    assert.ok(out.includes('\u240d'), 'CR shown as visible symbol');
  } finally {
    if (prevNoColor === undefined) delete process.env.NO_COLOR;
    else process.env.NO_COLOR = prevNoColor;
    if (prevForce === undefined) delete process.env.FORCE_COLOR;
    else process.env.FORCE_COLOR = prevForce;
  }
});

test('P1 sanitizer preserves caret alignment (1-char, width-1 replacements)', () => {
  const mk = (src) => [
    {
      filePath: '/x.js',
      source: src,
      messages: [
        {
          ruleId: 'quotes',
          severity: 2,
          line: 1,
          column: 11,
          message: 'Strings must use singlequote.',
        },
      ],
      errorCount: 1,
      warningCount: 0,
    },
  ];
  const clean = crisp(mk('const v = "ABCDEF";'), undefined);
  const dirty = crisp(mk('const v = "ABCD\x1bEF";'), undefined);
  const arrowCol = (s) => {
    const row = s.split('\n').find((l) => l.includes('↑'));
    return row ? row.indexOf('↑') : -1;
  };
  assert.equal(arrowCol(dirty), arrowCol(clean));
  assert.ok(arrowCol(clean) >= 0);
});

test('P1 sanitizeControl unit behavior (\\t and \\n untouched)', () => {
  assert.equal(sanitizeControl('plain text'), 'plain text');
  assert.equal(sanitizeControl('tab\tnewline\nkept'), 'tab\tnewline\nkept');
  assert.equal(sanitizeControl('esc\x1b'), 'esc\u241b');
  assert.equal(sanitizeControl('del\x7f'), 'del\u2421');
  assert.equal(sanitizeControl('c1\x85'), 'c1\uFFFD');
  assert.equal(sanitizeControl(null), null);
  assert.equal(sanitizeControl(undefined), undefined);
});

// ---------------------------------------------------------------------------
// P3 helpers: numeric domain coercion
// ---------------------------------------------------------------------------

test('P3 toU32 / toSeverity clamp hostile numerics', () => {
  assert.equal(toU32(-3), undefined);
  assert.equal(toU32(1e21), undefined);
  assert.equal(toU32(NaN), undefined);
  assert.equal(toU32(Infinity), undefined);
  assert.equal(toU32(1.9), 1);
  assert.equal(toU32(7), 7);
  assert.equal(toU32('5'), undefined, 'strings rejected');
  assert.equal(toSeverity(3), 3);
  assert.equal(toSeverity(-1), undefined);
  assert.equal(toSeverity('2'), undefined);
});

// ---------------------------------------------------------------------------
// P2: rulesMeta normalization
// ---------------------------------------------------------------------------

test('P2 normalizeData keeps only well-shaped metadata', () => {
  const clean = normalizeData({
    rulesMeta: {
      good: { docs: { url: 'https://x' } },
      nullMeta: null,
      noDocs: 42,
      nullDocs: { docs: null },
      nullUrl: { docs: { url: null } },
      deprecated: { docs: { url: 'https://y', deprecated: true } },
    },
  });
  assert.deepEqual(clean.rulesMeta.good, { docs: { url: 'https://x' } });
  assert.equal(clean.rulesMeta.nullMeta, undefined);
  assert.equal(clean.rulesMeta.noDocs, undefined);
  assert.equal(clean.rulesMeta.nullDocs, undefined);
  assert.deepEqual(clean.rulesMeta.nullUrl, { docs: {} });
  assert.deepEqual(clean.rulesMeta.deprecated, { docs: { url: 'https://y', deprecated: true } });
  assert.deepEqual(normalizeData(null), { rulesMeta: {} });
  assert.deepEqual(normalizeData({ rulesMeta: null }), { rulesMeta: {} });
});

// ---------------------------------------------------------------------------
// Loader fallback (Phase 2): gnu -> musl order with mock fs/require
// ---------------------------------------------------------------------------

test('loader falls through to the second candidate on require() failure', () => {
  const calls = { exists: [], loaded: [], failed: [] };
  const existsFn = (p) => {
    calls.exists.push(p);
    return true; // both files exist on disk
  };
  const requireFn = (p) => {
    if (p.includes('gnu')) {
      calls.failed.push(p);
      throw new Error('libstdc++.so.6: cannot open shared object (simulated Alpine)');
    }
    calls.loaded.push(p);
    return { marker: 'musl-loaded' };
  };
  const res = resolveNative(
    ['/fake/crisp.linux-x64-gnu.node', '/fake/crisp.linux-x64-musl.node'],
    existsFn,
    requireFn,
  );
  assert.equal(res.native && res.native.marker, 'musl-loaded');
  // loadError intentionally RETAINS the last failure for diagnostics; the
  // module only surfaces it when no native module loaded at all.
  assert.match(res.loadError.message, /simulated Alpine/);
  assert.equal(res.failed.length, 1);
  assert.ok(res.failed[0].includes('gnu'));
  assert.ok(calls.exists.length === 2, 'both candidates probed');
});

test('loader reports descriptive error when no candidate exists', () => {
  const res = resolveNative(
    ['/fake/never.node'],
    () => false,
    () => {
      throw new Error('should not be called');
    },
  );
  assert.equal(res.native, null);
  assert.ok(res.loadError.message.includes('no prebuilt native binary'));
  assert.ok(res.loadError.message.includes('Supported platforms'));
  assert.ok(res.loadError.message.includes('win32-x64-msvc'));
});

test('loader skips non-existent first candidate and loads the next', () => {
  const loaded = [];
  const res = resolveNative(
    ['/fake/missing.node', '/fake/present.node'],
    (p) => p.includes('present'),
    (p) => {
      loaded.push(p);
      return { ok: true };
    },
  );
  assert.equal(res.native && res.native.ok, true);
  assert.equal(loaded.length, 1);
  assert.ok(loaded[0].includes('present'));
});

test('currentTriples is platform-appropriate and non-empty', () => {
  const triples = currentTriples();
  assert.ok(Array.isArray(triples) && triples.length >= 1);
  if (process.platform === 'linux') {
    assert.ok(triples.some((t) => t.endsWith('-gnu')));
    assert.ok(triples.some((t) => t.endsWith('-musl')));
  } else if (process.platform === 'darwin') {
    assert.deepEqual(triples, [`darwin-${process.arch}`]);
  } else if (process.platform === 'win32') {
    assert.deepEqual(triples, [`win32-${process.arch}-msvc`]);
  }
});
