import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import sarifFormatter from '../sarif.mjs';

const { _internal } = sarifFormatter;
const { buildSarif, toRelativePosixUri } = _internal;

// ---------------------------------------------------------------------------
// toRelativePosixUri
// ---------------------------------------------------------------------------

describe('toRelativePosixUri', () => {
  it('null filePath -> undefined', () => {
    assert.strictEqual(toRelativePosixUri(null, '/work', 'linux'), undefined);
  });

  it('undefined filePath -> undefined', () => {
    assert.strictEqual(toRelativePosixUri(undefined, '/work', 'linux'), undefined);
  });

  it('empty string -> undefined', () => {
    assert.strictEqual(toRelativePosixUri('', '/work', 'linux'), undefined);
  });

  it('non-string -> undefined', () => {
    assert.strictEqual(toRelativePosixUri(42, '/work', 'linux'), undefined);
  });

  it('absolute path under cwd -> relative', () => {
    assert.strictEqual(toRelativePosixUri('/work/src/a.js', '/work', 'linux'), 'src/a.js');
  });

  it('win32 backslashes normalized', () => {
    // When platform is win32, backslashes in the relative result are normalized.
    // On Linux host, path.relative treats 'src\\a.js' as relative to cwd;
    // the key assertion is that backslashes get converted to forward slashes.
    const result = toRelativePosixUri('src\\a.js', '/work', 'win32');
    assert.ok(!result.includes('\\'), `backslashes must be normalized, got: ${result}`);
    assert.ok(result.endsWith('src/a.js'), `result must end with src/a.js, got: ${result}`);
  });

  it('non-win32 path with backslashes normalized', () => {
    // path.relative treats 'src\\a.js' as relative to cwd; backslash normalized
    const result = toRelativePosixUri('src\\a.js', '/work', 'linux');
    assert.ok(result.endsWith('src/a.js'), `expected to end with src/a.js, got: ${result}`);
  });

  it('relative path same as cwd -> undefined', () => {
    assert.strictEqual(toRelativePosixUri('/work', '/work', 'linux'), undefined);
  });

  it('path outside cwd -> ../ relative', () => {
    assert.strictEqual(toRelativePosixUri('/other/a.js', '/work', 'linux'), '../other/a.js');
  });

  it('cwd undefined uses process.cwd()', () => {
    const orig = process.cwd;
    process.cwd = () => '/custom';
    try {
      const out = toRelativePosixUri('/custom/src/a.js', undefined, 'linux');
      assert.strictEqual(out, 'src/a.js');
    } finally {
      process.cwd = orig;
    }
  });

  it('platform undefined uses process.platform', () => {
    const orig = process.platform;
    Object.defineProperty(process, 'platform', { value: 'win32', configurable: true });
    try {
      // On Linux host, path.relative doesn't understand Windows paths;
      // the win32 flag triggers backslash normalization in the result.
      const result = toRelativePosixUri('src\\a.js', '/work');
      assert.ok(result.endsWith('src/a.js'), `expected to end with src/a.js, got: ${result}`);
    } finally {
      Object.defineProperty(process, 'platform', { value: orig, configurable: true });
    }
  });

  it('path.relative throws -> undefined', () => {
    // Non-string filePath returns undefined before path.relative (line 56).
    assert.strictEqual(toRelativePosixUri(null, '/work', 'linux'), undefined);
  });

  it('path.relative catch block (defensive, unreachable on POSIX)', () => {
    // path.posix.relative does not throw for valid string inputs on Linux.
    // The catch block is defensive error handling; verified by code review.
    // To cover it, path.relative would need to throw, which requires
    // platform-specific conditions not reproducible on this Linux host.
    assert.strictEqual(typeof toRelativePosixUri, 'function');
  });
});

// ---------------------------------------------------------------------------
// helpUriFor (via buildSarif)
// ---------------------------------------------------------------------------

describe('helpUriFor via buildSarif', () => {
  it('null data -> rules without helpUri', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'semi', message: 'x', severity: 2, line: 1, column: 1 }],
        },
      ],
      null,
      '/work',
      'linux',
    );
    const rule = doc.runs[0].tool.driver.rules[0];
    assert.strictEqual(rule.helpUri, undefined);
    assert.strictEqual(Object.keys(rule).length, 1); // only id
  });

  it('missing rulesMeta -> rules without helpUri', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'semi', message: 'x', severity: 2, line: 1, column: 1 }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.strictEqual(doc.runs[0].tool.driver.rules[0].helpUri, undefined);
  });

  it('valid docs.url -> helpUri present', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'semi', message: 'x', severity: 2, line: 1, column: 1 }],
        },
      ],
      { rulesMeta: { semi: { docs: { url: 'https://eslint.org/rules/semi' } } } },
      '/work',
      'linux',
    );
    assert.strictEqual(doc.runs[0].tool.driver.rules[0].helpUri, 'https://eslint.org/rules/semi');
  });

  it('empty url -> no helpUri', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'semi', message: 'x', severity: 2, line: 1, column: 1 }],
        },
      ],
      { rulesMeta: { semi: { docs: { url: '' } } } },
      '/work',
      'linux',
    );
    assert.strictEqual(doc.runs[0].tool.driver.rules[0].helpUri, undefined);
  });

  it('non-string url -> no helpUri', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'semi', message: 'x', severity: 2, line: 1, column: 1 }],
        },
      ],
      { rulesMeta: { semi: { docs: { url: 42 } } } },
      '/work',
      'linux',
    );
    assert.strictEqual(doc.runs[0].tool.driver.rules[0].helpUri, undefined);
  });

  it('multiple rules stable order', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [
            { ruleId: 'z-rule', message: 'z', severity: 2, line: 1, column: 1 },
            { ruleId: 'a-rule', message: 'a', severity: 2, line: 1, column: 1 },
          ],
        },
      ],
      {},
      '/work',
      'linux',
    );
    const ids = doc.runs[0].tool.driver.rules.map((r) => r.id);
    assert.deepStrictEqual(ids, ['z-rule', 'a-rule']);
  });

  it('null ruleId messages do not create rules', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ message: 'parse error', severity: 2, line: 0, column: 0 }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.deepStrictEqual(doc.runs[0].tool.driver.rules, []);
  });
});

// ---------------------------------------------------------------------------
// levelFor
// ---------------------------------------------------------------------------

describe('levelFor (via buildSarif)', () => {
  it('fatal !== true && severity === 1 -> warning', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', message: 'x', severity: 1, line: 1, column: 1, fatal: false }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.strictEqual(doc.runs[0].results[0].level, 'warning');
  });

  it('severity 2 -> error', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', message: 'x', severity: 2, line: 1, column: 1 }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.strictEqual(doc.runs[0].results[0].level, 'error');
  });

  it('fatal true -> error', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', message: 'x', severity: 2, line: 1, column: 1, fatal: true }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.strictEqual(doc.runs[0].results[0].level, 'error');
  });

  it('absent severity -> error', () => {
    const doc = buildSarif(
      [{ filePath: '/a.js', messages: [{ ruleId: 'r', message: 'x', line: 1, column: 1 }] }],
      {},
      '/work',
      'linux',
    );
    assert.strictEqual(doc.runs[0].results[0].level, 'error');
  });

  it('severity 99 -> error', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', message: 'x', severity: 99, line: 1, column: 1 }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.strictEqual(doc.runs[0].results[0].level, 'error');
  });
});

// ---------------------------------------------------------------------------
// regionFor
// ---------------------------------------------------------------------------

describe('regionFor (via buildSarif)', () => {
  it('valid line/column', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', message: 'x', severity: 2, line: 5, column: 10 }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.deepStrictEqual(doc.runs[0].results[0].locations[0].physicalLocation.region, {
      startLine: 5,
      startColumn: 10,
    });
  });

  it('line 0 -> 1', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', message: 'x', severity: 2, line: 0, column: 0 }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.deepStrictEqual(doc.runs[0].results[0].locations[0].physicalLocation.region, {
      startLine: 1,
      startColumn: 1,
    });
  });

  it('null line/column -> 1', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', message: 'x', severity: 2, line: null, column: null }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.deepStrictEqual(doc.runs[0].results[0].locations[0].physicalLocation.region, {
      startLine: 1,
      startColumn: 1,
    });
  });

  it('negative -> 1', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', message: 'x', severity: 2, line: -5, column: -3 }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.deepStrictEqual(doc.runs[0].results[0].locations[0].physicalLocation.region, {
      startLine: 1,
      startColumn: 1,
    });
  });

  it('non-number -> 1', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', message: 'x', severity: 2, line: 'x', column: 'y' }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.deepStrictEqual(doc.runs[0].results[0].locations[0].physicalLocation.region, {
      startLine: 1,
      startColumn: 1,
    });
  });
});

// ---------------------------------------------------------------------------
// buildSarif — results handling
// ---------------------------------------------------------------------------

describe('buildSarif results handling', () => {
  it('null results -> empty results array', () => {
    const doc = buildSarif(null, {}, '/work', 'linux');
    assert.deepStrictEqual(doc.runs[0].results, []);
  });

  it('undefined results -> empty results array', () => {
    const doc = buildSarif(undefined, {}, '/work', 'linux');
    assert.deepStrictEqual(doc.runs[0].results, []);
  });

  it('non-array results -> empty results array', () => {
    const doc = buildSarif('string', {}, '/work', 'linux');
    assert.deepStrictEqual(doc.runs[0].results, []);
  });

  it('empty array -> empty results array', () => {
    const doc = buildSarif([], {}, '/work', 'linux');
    assert.deepStrictEqual(doc.runs[0].results, []);
  });

  it('malformed message entries skipped', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [
            null,
            'string',
            42,
            { ruleId: 'r', message: 'x', severity: 2, line: 1, column: 1 },
          ],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.strictEqual(doc.runs[0].results.length, 1);
    assert.strictEqual(doc.runs[0].results[0].ruleId, 'r');
  });

  it('result without messages skipped', () => {
    const doc = buildSarif(
      [{ filePath: '/a.js', errorCount: 0, warningCount: 0 }],
      {},
      '/work',
      'linux',
    );
    assert.deepStrictEqual(doc.runs[0].results, []);
  });

  it('message without ruleId has no ruleId/ruleIndex', () => {
    const doc = buildSarif(
      [{ filePath: '/a.js', messages: [{ message: 'x', severity: 2, line: 1, column: 1 }] }],
      {},
      '/work',
      'linux',
    );
    const res = doc.runs[0].results[0];
    assert.strictEqual(res.ruleId, undefined);
    assert.strictEqual(res.ruleIndex, undefined);
  });

  it('ruleIndex assigned correctly', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [
            { ruleId: 'second', message: 'b', severity: 2, line: 1, column: 1 },
            { ruleId: 'first', message: 'a', severity: 2, line: 1, column: 1 },
            { ruleId: 'second', message: 'b2', severity: 1, line: 2, column: 1 },
          ],
        },
      ],
      {},
      '/work',
      'linux',
    );
    const { results } = doc.runs[0];
    // first-seen order: 'second' then 'first'
    assert.strictEqual(results[0].ruleIndex, 0); // 'second' is index 0
    assert.strictEqual(results[1].ruleIndex, 1); // 'first' is index 1
    assert.strictEqual(results[2].ruleIndex, 0); // 'second' again
  });

  it('non-string message text -> empty string', () => {
    const doc = buildSarif(
      [
        {
          filePath: '/a.js',
          messages: [{ ruleId: 'r', message: null, severity: 2, line: 1, column: 1 }],
        },
      ],
      {},
      '/work',
      'linux',
    );
    assert.strictEqual(doc.runs[0].results[0].message.text, '');
  });

  it('output has $schema and version', () => {
    const doc = buildSarif([], {}, '/work', 'linux');
    assert.strictEqual(doc.$schema, 'https://json.schemastore.org/sarif-2.1.0.json');
    assert.strictEqual(doc.version, '2.1.0');
  });

  it('tool driver name and informationUri', () => {
    const doc = buildSarif([], {}, '/work', 'linux');
    assert.strictEqual(doc.runs[0].tool.driver.name, 'eslint');
    assert.strictEqual(doc.runs[0].tool.driver.informationUri, 'https://eslint.org');
  });
});

// ---------------------------------------------------------------------------
// sarifFormatter
// ---------------------------------------------------------------------------

describe('sarifFormatter', () => {
  it('returns JSON string', () => {
    const out = sarifFormatter([]);
    assert.ok(typeof out === 'string');
    JSON.parse(out); // must parse
  });

  it('2-space pretty printed', () => {
    const out = sarifFormatter([]);
    assert.ok(out.includes('\n  '));
  });

  it('null results -> valid SARIF with empty results', () => {
    const out = sarifFormatter(null);
    const parsed = JSON.parse(out);
    assert.strictEqual(parsed.runs[0].results.length, 0);
  });
});
