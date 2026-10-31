/**
 * Node.js integration tests for eslint-formatter-crisp.
 * Run: node --test (auto-discovers test/*.test.mjs)
 * Verifies: native loading, input conversion, output types, error
 * propagation, documented exports, and the s1 golden shape at level 0.
 *
 * Tests run against a REAL temporary cwd with real fixture files so the
 * adapter's process.cwd() path relativization is exercised end to end
 * (mirroring how ESLint invokes a formatter).
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import fs from 'node:fs';
import os from 'node:os';

const require = createRequire(import.meta.url);
const pkgRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const crisp = require(path.join(pkgRoot, 'index.cjs'));
// v0.4.0: the zero-dependency machine-readable formatters.
const jsonFormatter = require(path.join(pkgRoot, 'json.cjs'));
const sarifFormatter = require(path.join(pkgRoot, 'sarif.cjs'));

function stripAnsi(s) {
  // eslint-disable-next-line no-control-regex
  return s.replace(/\x1b(?:\[[0-9;]*m|\]8;;[^\x1b]*\x1b\\)/g, '');
}

/** Run `fn` inside a fresh temp dir containing the given files. */
function withTempCwd(files, fn) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'crisp-test-'));
  const written = {};
  for (const [name, content] of Object.entries(files)) {
    const p = path.join(dir, name);
    fs.writeFileSync(p, content);
    written[name] = p;
  }
  const prevCwd = process.cwd();
  process.chdir(dir);
  try {
    return fn(written, dir);
  } finally {
    process.chdir(prevCwd);
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

/** Force the no-color path for deterministic assertions. */
function withoutColor(fn) {
  const prevNoColor = process.env.NO_COLOR;
  const prevForce = process.env.FORCE_COLOR;
  process.env.NO_COLOR = '1';
  delete process.env.FORCE_COLOR;
  try {
    return fn();
  } finally {
    if (prevNoColor === undefined) delete process.env.NO_COLOR;
    else process.env.NO_COLOR = prevNoColor;
    if (prevForce === undefined) delete process.env.FORCE_COLOR;
    else process.env.FORCE_COLOR = prevForce;
  }
}

const BROKEN_JS =
  'const unusedVar = 42;\nfunction greet(name) {\n    console.log("hello " + name)\n    return name\n}\ngreet(undefinedThing)\n';

// Exact ESLint data captured with `eslint -f json` (Phase 2 M1).
function s1Results(brokenPath) {
  return [
    {
      filePath: brokenPath,
      source: BROKEN_JS,
      output: undefined,
      errorCount: 5,
      warningCount: 2,
      messages: [
        {
          ruleId: 'no-unused-vars',
          severity: 1,
          message: "'unusedVar' is assigned a value but never used.",
          line: 1,
          column: 7,
        },
        {
          ruleId: 'no-console',
          severity: 1,
          message: 'Unexpected console statement.',
          line: 3,
          column: 5,
        },
        {
          ruleId: 'quotes',
          severity: 2,
          message: 'Strings must use singlequote.',
          line: 3,
          column: 17,
          fix: { range: [17, 24], text: "'hello '" },
        },
        {
          ruleId: 'semi',
          severity: 2,
          message: 'Missing semicolon.',
          line: 3,
          column: 33,
          fix: { range: [32, 32], text: ';' },
        },
        {
          ruleId: 'semi',
          severity: 2,
          message: 'Missing semicolon.',
          line: 4,
          column: 16,
          fix: { range: [15, 15], text: ';' },
        },
        {
          ruleId: 'no-undef',
          severity: 2,
          message: "'undefinedThing' is not defined.",
          line: 6,
          column: 7,
        },
        {
          ruleId: 'semi',
          severity: 2,
          message: 'Missing semicolon.',
          line: 6,
          column: 22,
          fix: { range: [21, 21], text: ';' },
        },
      ],
    },
  ];
}

const S1_GOLDEN_PLAIN = `\n${'─'.repeat(37)}\n ./broken.js 🔥 5 errors 2 warnings🔥\n${'─'.repeat(37)}\n\n 1: const unusedVar = 42;\n          ↑\n          └─ ⚠ unusedVar is assigned a value but never used. [ no-unused-vars ]\n\n\n 3:     console.log("hello " + name)\n        ↑           ↑               ↑\n        │           │               └─ ✖ Missing semicolon. [ semi ] ( auto-fixable )\n        │           │\n        │           └─ ✖ Strings must use singlequote. [ quotes ] ( auto-fixable )\n        │\n        └─ ⚠ Unexpected console statement. [ no-console ]\n\n 4:     return name\n                   ↑\n                   └─ ✖ Missing semicolon. [ semi ] ( auto-fixable )\n\n\n 6: greet(undefinedThing)\n          ↑              ↑\n          │              └─ ✖ Missing semicolon. [ semi ] ( auto-fixable )\n          │\n          └─ ✖ undefinedThing is not defined. [ no-undef ]\n\n${'─'.repeat(67)}\n 5 errors   2 warnings   4 auto-fixables ( with the \`--fix\` flag )\n${'─'.repeat(67)}\n 💡 Run \`eslint --fix\` to auto-fix 4 issues.\n\n`;

test('native module loads', () => {
  assert.equal(crisp.nativeLoaded(), true);
});

test('formatter returns a string (the public ESLint interface)', () => {
  withTempCwd({ 'broken.js': BROKEN_JS }, (files) => {
    const out = crisp(s1Results(files['broken.js']), {});
    assert.equal(typeof out, 'string');
    assert.ok(out.length > 0);
  });
});

test('module.exports has default export (CJS/ESM interop)', () => {
  assert.equal(typeof crisp.default, 'function');
  assert.equal(crisp.default, crisp);
});

test('s1 golden output matches reference text at color level 0', () => {
  withTempCwd({ 'broken.js': BROKEN_JS }, (files) => {
    withoutColor(() => {
      const out = crisp(s1Results(files['broken.js']), {});
      assert.equal(out, S1_GOLDEN_PLAIN, 'exact byte match (no color path)');
    });
  });
});

test('FORCE_COLOR=1 adds ANSI codes but same visible text', () => {
  withTempCwd({ 'broken.js': BROKEN_JS }, (files) => {
    const prevNoColor = process.env.NO_COLOR;
    delete process.env.NO_COLOR;
    process.env.FORCE_COLOR = '1';
    try {
      const out = crisp(s1Results(files['broken.js']), {});
      assert.ok(out.includes('\x1b[2m'), 'dim codes present');
      assert.ok(out.includes('\x1b[31m'), 'red present');
      assert.ok(out.includes('\x1b[91m'), 'fruit gradient present');
      assert.equal(stripAnsi(out), stripAnsi(S1_GOLDEN_PLAIN), 'same visible text as plain');
    } finally {
      if (prevNoColor === undefined) delete process.env.NO_COLOR;
      else process.env.NO_COLOR = prevNoColor;
      delete process.env.FORCE_COLOR;
    }
  });
});

test('empty results produce empty string', () => {
  withoutColor(() => {
    assert.equal(crisp([], {}), '');
    // null-safe: ESLint may omit source; hand-rolled callers may send null
    assert.equal(
      crisp(
        [
          {
            filePath: '/ok.js',
            source: null,
            output: undefined,
            errorCount: 0,
            warningCount: 0,
            messages: [],
          },
        ],
        {},
      ),
      '',
    );
  });
});

test('invalid input throws a clear TypeError', () => {
  assert.throws(() => crisp('not an array', {}), TypeError);
  assert.throws(() => crisp(42, {}), TypeError);
});

test('malformed message shape surfaces a boundary error', () => {
  assert.throws(
    () => crisp([{ filePath: '/a.js', messages: [{ message: 42 }] }], {}),
    (err) => err instanceof Error,
  );
});

test('rulesMeta drives rule links data path (no throw without it)', () => {
  withTempCwd({ 'broken.js': BROKEN_JS }, (files) => {
    withoutColor(() => {
      const data = { rulesMeta: { 'no-console': { docs: { url: 'https://example.com' } } } };
      const out = crisp(s1Results(files['broken.js']), data);
      assert.ok(out.includes('[ no-console ]'));
    });
  });
});

test('fix-dry-run: output preferred over source', () => {
  withoutColor(() => {
    const results = [
      {
        filePath: '/work/fixed.js',
        source: 'let a = 1\n',
        output: 'let a = 1;\n',
        errorCount: 1,
        warningCount: 0,
        messages: [
          {
            ruleId: 'no-console',
            severity: 1,
            message: 'Unexpected console statement.',
            line: 1,
            column: 1,
          },
        ],
      },
    ];
    const out = crisp(results, {});
    assert.ok(out.includes('let a = 1;'));
    assert.ok(!out.includes('let a = 1\n'));
  });
});

test('parsing-error short-circuit', () => {
  withoutColor(() => {
    const results = [
      {
        filePath: '/work/bad.js',
        source: 'const broken = {',
        output: undefined,
        errorCount: 1,
        warningCount: 0,
        messages: [
          {
            ruleId: null,
            severity: 2,
            message: 'Parsing error: Unexpected token',
            line: 0,
            column: 0,
            fatal: true,
          },
        ],
      },
    ];
    const out = crisp(results, {});
    // v0.5.3: parse error renders as `└─ ✖` + `  Parsing error: ...` on
    // separate lines (with a dim hint suffix). Both must be present.
    assert.ok(out.includes('✖'), `✖ icon present: ${out}`);
    assert.ok(out.includes('Parsing error: Unexpected token'), `message present: ${out}`);
    assert.ok(out.includes('ESLint could not parse this file'), `v0.5.3 hint present: ${out}`);
    assert.ok(!out.includes(' 1: '), `no code frame: ${out}`);
  });
});

test('unicode CJK alignment (display-width based)', () => {
  withoutColor(() => {
    const results = [
      {
        filePath: '/work/unicode.js',
        source: 'const 你好 = "world";\n',
        output: undefined,
        errorCount: 1,
        warningCount: 0,
        messages: [
          {
            ruleId: 'quotes',
            severity: 2,
            message: 'Strings must use singlequote.',
            line: 1,
            column: 12,
            fix: { range: [11, 18], text: "'world'" },
          },
        ],
      },
    ];
    const out = crisp(results, {});
    // Output rows: 0='', 1=divider, 2=header, 3=divider, 4=blank(file sep), 5=source, 6=arrows
    // v0.5.2: file-block separator adds a blank line after the header divider.
    const arrowRow = stripAnsi(out).split('\n')[6];
    assert.equal(arrowRow.trimEnd(), ' '.repeat(17) + '↑');
  });
});

test('CRISP_ASCII=1 emits pure-ASCII decoration (mojibake-safe mode)', () => {
  withTempCwd({ 'broken.js': BROKEN_JS }, (files) => {
    withoutColor(() => {
      const prevAscii = process.env.CRISP_ASCII;
      process.env.CRISP_ASCII = '1';
      try {
        const out = crisp(s1Results(files['broken.js']), {});
        assert.ok(
          [...out].every((c) => c.charCodeAt(0) < 128),
          `expected pure ASCII, found: ${[...out].filter((c) => c.charCodeAt(0) >= 128)}`,
        );
        const plain = stripAnsi(out);
        assert.ok(plain.includes(' ./broken.js * 5 errors 2 warnings*'));
        // v0.5.3: ASCII mode uses uppercase `X` for ✖ (was `x`)
        assert.ok(plain.includes('\\- X Missing semicolon. [ semi ]'));
        assert.ok(plain.includes('\\- ! unusedVar is assigned'));
        assert.ok(plain.includes('^'));
        assert.ok(plain.includes('--'));
      } finally {
        if (prevAscii === undefined) delete process.env.CRISP_ASCII;
        else process.env.CRISP_ASCII = prevAscii;
      }
    });
  });
});

// ---------------------------------------------------------------------------
// v0.3.0 — width-aware layout (COLUMNS / CRISP_FULL_LINES) + experimental
// suggestions display. These run against the real native binary.
// ---------------------------------------------------------------------------

function withEnv(env, fn) {
  const saved = {};
  for (const k of Object.keys(env)) {
    saved[k] = process.env[k];
    if (env[k] === undefined) delete process.env[k];
    else process.env[k] = env[k];
  }
  try {
    return fn();
  } finally {
    for (const k of Object.keys(env)) {
      if (saved[k] === undefined) delete process.env[k];
      else process.env[k] = saved[k];
    }
  }
}

const LONG_LINE_JS = 'const v = "' + 'A'.repeat(120) + '"\nconst w = 2\n';

test('COLUMNS truncates the source excerpt but keeps the caret aligned', () => {
  withEnv({ COLUMNS: '60', NO_COLOR: '1' }, () => {
    const results = [
      {
        filePath: '/work/long.js',
        source: LONG_LINE_JS,
        errorCount: 2,
        warningCount: 0,
        messages: [
          {
            ruleId: 'quotes',
            severity: 2,
            message: 'Strings must use singlequote.',
            line: 1,
            column: 12,
          },
          { ruleId: 'semi', severity: 2, message: 'Missing semicolon.', line: 2, column: 11 },
        ],
      },
    ];
    const out = crisp(results, {});
    const lines = out.split('\n');
    // every line fits the budget
    for (const l of lines) {
      assert.ok([...l].length <= 60, `line too wide (${[...l].length}): ${l}`);
    }
    // ellipsis markers present on the truncated source row
    assert.ok(
      lines.some((l) => l.includes('\u2026')),
      'excerpt marker',
    );
    // both carets survived (one per error line)
    assert.equal(lines.filter((l) => l.includes('\u2191')).length, 2);
    // rule ids still present after wrapping
    assert.ok(out.includes('quotes') && out.includes('semi'));
  });
});

test('CRISP_FULL_LINES=1 disables truncation even with COLUMNS set', () => {
  withEnv({ COLUMNS: '60', CRISP_FULL_LINES: '1', NO_COLOR: '1' }, () => {
    const results = [
      {
        filePath: '/work/long.js',
        source: LONG_LINE_JS,
        errorCount: 1,
        warningCount: 0,
        messages: [
          {
            ruleId: 'quotes',
            severity: 2,
            message: 'Strings must use singlequote.',
            line: 1,
            column: 12,
          },
        ],
      },
    ];
    const out = crisp(results, {});
    assert.ok(out.includes('A'.repeat(120)), 'full line kept');
    assert.ok(!out.includes('\u2026'), 'no ellipsis marker');
  });
});

test('long messages wrap with continuation indent under COLUMNS', () => {
  withEnv({ COLUMNS: '50', NO_COLOR: '1' }, () => {
    const out = crisp(
      [
        {
          filePath: '/work/w.js',
          source: 'x = 1;',
          errorCount: 1,
          warningCount: 0,
          messages: [
            {
              ruleId: 'long-rule',
              severity: 2,
              message:
                'This is a very long diagnostic message that keeps going and going and going until extremely wide.',
              line: 1,
              column: 1,
            },
          ],
        },
      ],
      {},
    );
    const lines = out.split('\n');
    for (const l of lines) {
      assert.ok([...l].length <= 50, `line too wide: ${l}`);
    }
    // continuation lines are indented to the message start (no connector)
    const cont = lines.filter(
      (l) => l.trim().length > 0 && !l.includes('\u2500') && !l.includes(':'),
    );
    assert.ok(cont.length >= 2, 'message wrapped across lines');
    // the rule suffix rides on the LAST message line
    const last = [...lines].reverse().find((l) => l.includes('long-rule'));
    assert.ok(last.trimEnd().endsWith('[ long-rule ]'));
  });
});

test('detectMaxWidth precedence: FULL_LINES > COLUMNS > TTY > none', () => {
  const { detectMaxWidth } = crisp._internal;
  withEnv({ CRISP_FULL_LINES: '1', COLUMNS: '40' }, () => {
    assert.equal(detectMaxWidth({ isTTY: true, columns: 100 }), undefined);
  });
  withEnv({ CRISP_FULL_LINES: undefined, COLUMNS: '40' }, () => {
    assert.equal(detectMaxWidth({ isTTY: false }), 40);
  });
  withEnv({ CRISP_FULL_LINES: undefined, COLUMNS: 'nope' }, () => {
    assert.equal(detectMaxWidth({ isTTY: false }), undefined, 'invalid COLUMNS ignored');
    assert.equal(detectMaxWidth({ isTTY: true, columns: 80 }), 80, 'TTY size used');
  });
  withEnv({ CRISP_FULL_LINES: undefined, COLUMNS: undefined }, () => {
    assert.equal(detectMaxWidth({ isTTY: false }), undefined);
    assert.equal(detectMaxWidth(undefined), undefined);
  });
});

test('CRISP_SUGGESTIONS=1 renders the first fix suggestion (experimental)', () => {
  withEnv({ NO_COLOR: '1' }, () => {
    const results = [
      {
        filePath: '/work/s.js',
        source: 'const x = foo == bar;',
        errorCount: 1,
        warningCount: 0,
        messages: [
          {
            ruleId: 'eqeqeq',
            severity: 2,
            message: "Expected '===' and instead saw '=='.",
            line: 1,
            column: 15,
            suggestions: [{ desc: 'Use ===', fix: { range: [14, 16], text: '===' } }],
          },
        ],
      },
    ];
    const withoutFlag = crisp(results, {});
    assert.ok(!withoutFlag.includes('? Use ==='), 'off by default');
    process.env.CRISP_SUGGESTIONS = '1';
    try {
      const withFlag = crisp(results, {});
      assert.ok(withFlag.includes('? Use ==='), 'suggestion line rendered');
      assert.ok(withFlag.includes('Use ==='));
    } finally {
      delete process.env.CRISP_SUGGESTIONS;
    }
  });
});

test('deprecated rule marker appears only with CRISP_SUGGESTIONS=1', () => {
  withEnv({ NO_COLOR: '1' }, () => {
    const results = [
      {
        filePath: '/work/d.js',
        source: 'x = 1;',
        errorCount: 1,
        warningCount: 0,
        messages: [{ ruleId: 'old-rule', severity: 2, message: 'boom', line: 1, column: 1 }],
      },
    ];
    const data = { rulesMeta: { 'old-rule': { docs: { url: 'https://x', deprecated: true } } } };
    const plain = crisp(results, data);
    assert.ok(!plain.includes('deprecated'), 'marker off by default');
    process.env.CRISP_SUGGESTIONS = '1';
    try {
      const marked = crisp(results, data);
      assert.ok(marked.includes('( deprecated )'), 'marker rendered when enabled');
    } finally {
      delete process.env.CRISP_SUGGESTIONS;
    }
  });
});

test('piped output without COLUMNS stays byte-identical to the historical layout', () => {
  withEnv({ COLUMNS: undefined, NO_COLOR: '1' }, () => {
    const results = [
      {
        filePath: '/work/long.js',
        source: LONG_LINE_JS,
        errorCount: 1,
        warningCount: 0,
        messages: [
          {
            ruleId: 'quotes',
            severity: 2,
            message: 'Strings must use singlequote.',
            line: 1,
            column: 12,
          },
        ],
      },
    ];
    const out = crisp(results, {});
    assert.ok(out.includes('A'.repeat(120)));
  });
});

test('CRISP_THEME JSON overrides gradient stops (experimental)', () => {
  withEnv({ FORCE_COLOR: '3' }, () => {
    const results = s1Results('/work/broken.js');
    const def = crisp(results, {});
    process.env.CRISP_THEME = '{"fruitA":"#001122"}';
    try {
      const themed = crisp(results, {});
      assert.notEqual(themed, def, 'gradient bytes change');
      assert.ok(themed.includes('\x1b[38;2;0;17;34m'), 'custom stop emitted');
    } finally {
      delete process.env.CRISP_THEME;
    }
  });
});

test('MO_THEME stays mo-compatible: known name is a no-op, JSON works', () => {
  withEnv({ FORCE_COLOR: '3' }, () => {
    const results = s1Results('/work/broken.js');
    const def = crisp(results, {});
    process.env.MO_THEME = 'one-dark-pro';
    const named = crisp(results, {});
    delete process.env.MO_THEME;
    assert.equal(named, def, "known Shiki name 'one-dark-pro' keeps the default palette");
    process.env.MO_THEME = '{"cristalB":"#010203"}';
    const themed = crisp(results, {});
    delete process.env.MO_THEME;
    assert.notEqual(themed, def, 'MO_THEME JSON object applies');
  });
});

test('invalid theme env values fall back to default silently', () => {
  withEnv({ FORCE_COLOR: '3' }, () => {
    const results = s1Results('/work/broken.js');
    const def = crisp(results, {});
    for (const bad of ['not-json', '{"fruitA":"zzz"}', '["array"]', 'theme-name-unknown']) {
      process.env.CRISP_THEME = bad;
      try {
        assert.equal(crisp(results, {}), def, `default output for ${bad}`);
      } finally {
        delete process.env.CRISP_THEME;
      }
    }
  });
});

test('parseThemeJson unit behavior', () => {
  const { parseThemeJson, detectTheme } = crisp._internal;
  assert.deepEqual(parseThemeJson('{"fruitA":"#ff0000"}'), { fruitA: 'ff0000' });
  assert.deepEqual(parseThemeJson('{"mono":true}'), { mono: true });
  assert.equal(parseThemeJson('{"fruitA":"nope"}'), undefined);
  assert.equal(parseThemeJson('null'), undefined);
  assert.equal(parseThemeJson('42'), undefined);
  assert.equal(detectTheme(), undefined);
});

// ---------------------------------------------------------------------------
// v0.4.0 — CRISP_MAX_MESSAGES passthrough, FORCE_COLOR standard semantics,
// and the zero-dependency json.cjs / sarif.cjs formatters.
// ---------------------------------------------------------------------------

// Ten-error fixture: one message per line so the rendered/capped message
// count is countable line-by-line, and the summary always totals 10 problems.
function capResults() {
  return [
    {
      filePath: '/work/capped.js',
      source: Array.from({ length: 10 }, (_, i) => `let v${i} = ${i};`).join('\n'),
      errorCount: 10,
      warningCount: 0,
      messages: Array.from({ length: 10 }, (_, i) => ({
        ruleId: 'cap-rule',
        severity: 2,
        message: `cap problem ${i}`,
        line: i + 1,
        column: 1,
      })),
    },
  ];
}

// Feature probe: does the SHIPPED native binary implement the render cap?
// The JS adapter only forwards `maxMessages` to the napi boundary — the
// truncation notice and the totals-vs-list split live in the Rust core. A
// freshly checked-out tree whose prebuilt binary predates the option must
// skip (not fail): the JS contract is frozen by the detectMaxMessages unit
// test below.
const NATIVE_SUPPORTS_MAX_MESSAGES = (() => {
  for (const triple of crisp._internal.currentTriples()) {
    const p = path.join(pkgRoot, `crisp.${triple}.node`);
    if (!fs.existsSync(p)) continue;
    try {
      const native = require(p);
      const out = native.format(capResults(), {}, { colorLevel: 0, maxMessages: 3 });
      return (
        stripAnsi(out)
          .split('\n')
          .filter((l) => l.includes('cap problem')).length < 10
      );
    } catch {
      return false;
    }
  }
  return false;
})();

test('detectMaxMessages parses CRISP_MAX_MESSAGES strictly (u32-safe)', () => {
  const { detectMaxMessages } = crisp._internal;
  withEnv({ CRISP_MAX_MESSAGES: '3' }, () => assert.equal(detectMaxMessages(), 3));
  withEnv({ CRISP_MAX_MESSAGES: '0' }, () =>
    assert.equal(detectMaxMessages(), 0, '"0" passes through verbatim'),
  );
  withEnv({ CRISP_MAX_MESSAGES: '4294967295' }, () =>
    assert.equal(detectMaxMessages(), 4294967295),
  );
  for (const bad of [
    undefined,
    '',
    'abc',
    '-1',
    '3.5',
    ' 3',
    '+3',
    '3x',
    '1e3',
    '4294967296',
    '99999999999999999999',
  ]) {
    withEnv({ CRISP_MAX_MESSAGES: bad }, () =>
      assert.equal(detectMaxMessages(), undefined, `omitted for ${JSON.stringify(bad)}`),
    );
  }
});

test(
  'CRISP_MAX_MESSAGES=3 caps the rendered list but the summary keeps all 10 problems',
  {
    skip: NATIVE_SUPPORTS_MAX_MESSAGES
      ? false
      : 'shipped native binary predates max_messages support (JS passthrough is covered by the detectMaxMessages unit test)',
  },
  () => {
    withEnv({ CRISP_MAX_MESSAGES: '3', NO_COLOR: '1' }, () => {
      const out = stripAnsi(crisp(capResults(), {}));
      // Exact notice (v0.4.0 Rust core wording): 10 problems, cap 3 -> 7 hidden.
      assert.ok(
        out.includes(
          '… 7 more messages not shown (raised cap: set CRISP_MAX_MESSAGES=0 for unlimited)',
        ),
        'truncation notice present with the exact hidden count',
      );
      const rendered = out.split('\n').filter((l) => l.includes('cap problem')).length;
      assert.ok(rendered < 10 && rendered >= 1, `rendered list capped (showed ${rendered})`);
      assert.ok(out.includes('10 errors'), 'summary still totals all 10 problems');
    });
  },
);

test('CRISP_MAX_MESSAGES=0 means unlimited: no notice, all 10 messages rendered', () => {
  withEnv({ CRISP_MAX_MESSAGES: '0', NO_COLOR: '1' }, () => {
    const out = stripAnsi(crisp(capResults(), {}));
    assert.equal(out.split('\n').filter((l) => l.includes('cap problem')).length, 10);
    assert.ok(!out.includes('not shown'), 'no truncation notice at 0');
  });
});

test('CRISP_MAX_MESSAGES garbage is ignored: byte-identical to unset, no throw', () => {
  let baseline;
  withEnv({ CRISP_MAX_MESSAGES: undefined, NO_COLOR: '1' }, () => {
    baseline = crisp(capResults(), {});
  });
  for (const garbage of ['garbage', '-1', '3.5', ' 3', '', '1e3', '99999999999999999999']) {
    withEnv({ CRISP_MAX_MESSAGES: garbage, NO_COLOR: '1' }, () => {
      assert.equal(crisp(capResults(), {}), baseline, `ignored: ${JSON.stringify(garbage)}`);
    });
  }
});

test('detectColorLevel: FORCE_COLOR beats NO_COLOR, TERM=dumb, CI, and TTY sniffing', () => {
  const { detectColorLevel } = crisp._internal;
  const tty = { isTTY: true };
  const pipe = { isTTY: false };
  withEnv({ FORCE_COLOR: '0', NO_COLOR: '1', CI: undefined, TERM: undefined }, () =>
    assert.equal(detectColorLevel(tty), 0, 'FORCE_COLOR=0 beats NO_COLOR + TTY'),
  );
  withEnv({ FORCE_COLOR: 'false', NO_COLOR: undefined, CI: undefined, TERM: undefined }, () =>
    assert.equal(detectColorLevel(tty), 0, 'FORCE_COLOR=false -> 0'),
  );
  withEnv({ FORCE_COLOR: 'true', NO_COLOR: '1', CI: undefined, TERM: undefined }, () =>
    assert.equal(detectColorLevel(tty), 1, 'FORCE_COLOR=true -> 1 (beats NO_COLOR)'),
  );
  withEnv({ FORCE_COLOR: '1', CI: undefined, TERM: undefined }, () =>
    assert.equal(detectColorLevel(pipe), 1, 'FORCE_COLOR=1 forces color even when piped'),
  );
  withEnv({ FORCE_COLOR: '2', NO_COLOR: '1', CI: undefined, TERM: 'dumb' }, () =>
    assert.equal(detectColorLevel(tty), 2, 'FORCE_COLOR=2 -> 256-color'),
  );
  withEnv({ FORCE_COLOR: '3', NO_COLOR: '1', CI: undefined, TERM: 'dumb' }, () =>
    assert.equal(detectColorLevel(tty), 3, 'FORCE_COLOR=3 -> 24-bit'),
  );
  withEnv({ FORCE_COLOR: '', NO_COLOR: '1' }, () =>
    assert.equal(detectColorLevel(tty), 1, "FORCE_COLOR='' -> 1 (chalk compatibility)"),
  );
});

test('detectColorLevel: unset FORCE_COLOR keeps the legacy detection chain', () => {
  const { detectColorLevel } = crisp._internal;
  withEnv({ FORCE_COLOR: undefined, NO_COLOR: undefined, CI: undefined, TERM: undefined }, () => {
    assert.equal(detectColorLevel({ isTTY: true }), 1, 'TTY -> 16-color');
    assert.equal(detectColorLevel({ isTTY: false }), 0, 'piped -> plain');
    assert.equal(detectColorLevel(undefined), 0, 'no stream -> plain');
  });
  withEnv({ FORCE_COLOR: undefined, NO_COLOR: '1', CI: undefined, TERM: undefined }, () =>
    assert.equal(detectColorLevel({ isTTY: true }), 0, 'NO_COLOR still honored'),
  );
  withEnv({ FORCE_COLOR: undefined, NO_COLOR: undefined, CI: undefined, TERM: 'dumb' }, () =>
    assert.equal(detectColorLevel({ isTTY: true }), 0, 'TERM=dumb still honored'),
  );
  withEnv({ FORCE_COLOR: undefined, NO_COLOR: undefined, CI: '1', TERM: undefined }, () =>
    assert.equal(detectColorLevel({ isTTY: false }), 1, 'CI still honored'),
  );
});

test('FORCE_COLOR=0 forces the byte-identical plain path (zero ANSI sequences)', () => {
  withTempCwd({ 'broken.js': BROKEN_JS }, (files) => {
    withEnv(
      { FORCE_COLOR: '0', NO_COLOR: undefined, CI: undefined, TERM: undefined, COLUMNS: undefined },
      () => {
        const out = crisp(s1Results(files['broken.js']), {});
        assert.equal(out, stripAnsi(out), 'no escape sequences at all');
        assert.equal(out, S1_GOLDEN_PLAIN, 'byte-identical to the level-0 golden');
      },
    );
  });
});

test('FORCE_COLOR=1 forces 16-color SGR and never 24-bit', () => {
  withTempCwd({ 'broken.js': BROKEN_JS }, (files) => {
    withEnv({ FORCE_COLOR: '1', NO_COLOR: undefined }, () => {
      const out = crisp(s1Results(files['broken.js']), {});
      assert.ok(out.includes('\x1b[31m'), '16-color SGR present');
      assert.ok(!out.includes('\x1b[38;2;'), 'no 24-bit SGR at level 1');
      assert.equal(stripAnsi(out), stripAnsi(S1_GOLDEN_PLAIN), 'same visible text as plain');
    });
  });
});

test('FORCE_COLOR=3 forces 24-bit SGR', () => {
  withTempCwd({ 'broken.js': BROKEN_JS }, (files) => {
    withEnv({ FORCE_COLOR: '3', NO_COLOR: undefined }, () => {
      const out = crisp(s1Results(files['broken.js']), {});
      assert.ok(out.includes('\x1b[38;2;'), '24-bit SGR present');
      assert.equal(stripAnsi(out), stripAnsi(S1_GOLDEN_PLAIN), 'same visible text as plain');
    });
  });
});

test('json.cjs round-trips ESLint results (canonical JSON.stringify shape)', () => {
  const results = s1Results('/work/broken.js');
  const data = {
    rulesMeta: { quotes: { docs: { url: 'https://eslint.org/docs/latest/rules/quotes' } } },
  };
  const out = jsonFormatter(results, data);
  assert.equal(typeof out, 'string');
  assert.equal(out, JSON.stringify(results, null, 2), 'canonical 2-space shape');
  assert.deepEqual(
    JSON.parse(out),
    JSON.parse(JSON.stringify(results)),
    'every JSON-representable field survives the round-trip (undefined fields drop, as JSON must)',
  );
  assert.ok(out.includes('"fix"'), 'fix objects preserved verbatim');
  assert.ok(!out.includes('rulesMeta'), 'data argument never leaks into the output');
});

test('json.cjs: empty/null/undefined results serialize as []', () => {
  assert.equal(jsonFormatter([]), '[]');
  assert.equal(jsonFormatter(null), '[]');
  assert.equal(jsonFormatter(undefined), '[]');
  assert.equal(jsonFormatter('nope'), '[]');
});

test('sarif.cjs emits a valid single-run SARIF 2.1.0 document', () => {
  const results = [
    {
      filePath: '/work/a.js',
      messages: [
        {
          ruleId: 'no-console',
          severity: 1,
          message: 'Unexpected console statement.',
          line: 3,
          column: 5,
        },
        { ruleId: 'semi', severity: 2, message: 'Missing semicolon.', line: 3, column: 33 },
        { ruleId: 'semi', severity: 2, message: 'Missing semicolon.', line: 4, column: 16 },
      ],
    },
  ];
  const data = {
    rulesMeta: {
      'no-console': { docs: { url: 'https://eslint.org/docs/latest/rules/no-console/' } },
      semi: {},
    },
  };
  const out = sarifFormatter(results, data);
  assert.equal(out, JSON.stringify(JSON.parse(out), null, 2), '2-space pretty-printed JSON');
  const doc = JSON.parse(out);
  assert.equal(doc.$schema, 'https://json.schemastore.org/sarif-2.1.0.json');
  assert.equal(doc.version, '2.1.0');
  assert.equal(doc.runs.length, 1, 'exactly one run');
  const run = doc.runs[0];
  assert.equal(run.tool.driver.name, 'eslint');
  assert.equal(run.tool.driver.informationUri, 'https://eslint.org');
  assert.deepEqual(
    run.tool.driver.rules,
    [
      { id: 'no-console', helpUri: 'https://eslint.org/docs/latest/rules/no-console/' },
      { id: 'semi' },
    ],
    'one deduped entry per unique ruleId; helpUri from rulesMeta; none without docs.url',
  );
  assert.equal(run.results.length, 3);
  assert.equal(run.results[0].level, 'warning', 'severity 1 -> warning');
  assert.equal(run.results[1].level, 'error', 'severity 2 -> error');
  assert.equal(run.results[0].ruleId, 'no-console');
  assert.equal(run.results[0].ruleIndex, 0);
  assert.equal(run.results[2].ruleId, 'semi', 'duplicate ruleIds dedupe in the rules table');
  assert.equal(run.results[0].message.text, 'Unexpected console statement.');
  const loc = run.results[0].locations[0].physicalLocation;
  const uri = loc.artifactLocation.uri;
  assert.ok(typeof uri === 'string' && uri.length > 0 && !uri.includes('\\'), 'relative POSIX uri');
  assert.ok(uri.endsWith('a.js'), `uri points at the linted file (got ${uri})`);
  assert.equal(loc.region.startLine, 3);
  assert.equal(loc.region.startColumn, 5);
});

test('sarif.cjs: artifactLocation.uri is cwd-relative POSIX', () => {
  withTempCwd({ 'a.js': 'const x = 1\n' }, (files) => {
    const doc = JSON.parse(
      sarifFormatter(
        [
          {
            filePath: files['a.js'],
            messages: [{ ruleId: 'r', severity: 2, message: 'boom', line: 1, column: 1 }],
          },
        ],
        {},
      ),
    );
    assert.equal(
      doc.runs[0].results[0].locations[0].physicalLocation.artifactLocation.uri,
      'a.js',
      'absolute filePath inside the cwd relativizes exactly',
    );
  });
});

test('sarif.cjs: windows-style paths normalize to relative POSIX uris', () => {
  const { toRelativePosixUri } = sarifFormatter._internal;
  assert.equal(toRelativePosixUri('/repo/src/a.js', '/repo', 'linux'), 'src/a.js');
  const winUri = toRelativePosixUri('C:\\repo\\src\\a.js', '/anywhere', 'linux');
  assert.ok(!winUri.includes('\\'), 'backslashes converted on the helper level');
  assert.ok(winUri.endsWith('src/a.js'), `win path relativized (got ${winUri})`);
  // End to end: the same normalization applies through the public formatter.
  const doc = JSON.parse(
    sarifFormatter(
      [
        {
          filePath: 'C:\\repo\\src\\b.js',
          messages: [{ ruleId: 'r', severity: 2, message: 'boom', line: 2, column: 4 }],
        },
      ],
      {},
    ),
  );
  const loc = doc.runs[0].results[0].locations[0].physicalLocation;
  assert.ok(!loc.artifactLocation.uri.includes('\\'), 'forward slashes only');
  assert.ok(loc.artifactLocation.uri.endsWith('src/b.js'), `got ${loc.artifactLocation.uri}`);
  assert.equal(loc.region.startLine, 2);
  assert.equal(loc.region.startColumn, 4);
});

test('sarif.cjs: fatal parse errors -> error level, no ruleId, clamped region', () => {
  const doc = JSON.parse(
    sarifFormatter(
      [
        {
          filePath: '/work/bad.js',
          messages: [
            {
              ruleId: null,
              severity: 2,
              message: 'Parsing error: Unexpected token',
              line: 0,
              column: 0,
              fatal: true,
            },
          ],
        },
      ],
      {},
    ),
  );
  assert.deepEqual(doc.runs[0].tool.driver.rules, [], 'null ruleId creates no rule entry');
  const res = doc.runs[0].results[0];
  assert.equal(res.level, 'error', 'fatal parse errors report as error');
  assert.ok(!('ruleId' in res), 'ruleId omitted for null ruleId');
  assert.ok(!('ruleIndex' in res), 'ruleIndex omitted for null ruleId');
  assert.equal(res.message.text, 'Parsing error: Unexpected token');
  const region = res.locations[0].physicalLocation.region;
  assert.equal(region.startLine, 1, 'line 0 clamps to 1');
  assert.equal(region.startColumn, 1, 'column 0 clamps to 1');
});

test('package exports map resolves ./json and ./sarif (self-reference smoke)', () => {
  // Exercises the package.json exports wiring end to end (types entries are
  // compile-time only; this proves the runtime default targets load).
  const byExportsJson = require('eslint-formatter-crisp/json');
  const byExportsSarif = require('eslint-formatter-crisp/sarif');
  assert.equal(byExportsJson, jsonFormatter, './json -> json.cjs');
  assert.equal(byExportsSarif, sarifFormatter, './sarif -> sarif.cjs');
  assert.equal(byExportsJson([{ any: 'field' }]), JSON.stringify([{ any: 'field' }], null, 2));
  assert.equal(JSON.parse(byExportsSarif([], {})).version, '2.1.0');
});

test('sarif.cjs: empty/null results produce a valid empty run', () => {
  for (const input of [[], null, undefined]) {
    const doc = JSON.parse(sarifFormatter(input, {}));
    assert.equal(doc.$schema, 'https://json.schemastore.org/sarif-2.1.0.json');
    assert.equal(doc.version, '2.1.0');
    assert.equal(doc.runs.length, 1);
    assert.deepEqual(doc.runs[0].results, [], 'results present even when empty');
    assert.deepEqual(doc.runs[0].tool.driver.rules, []);
    assert.equal(doc.runs[0].tool.driver.name, 'eslint');
    assert.equal(doc.runs[0].tool.driver.informationUri, 'https://eslint.org');
  }
});
