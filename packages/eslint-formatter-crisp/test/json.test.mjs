import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import jsonFormatter from '../json.mjs';

describe('json.mjs', () => {
  it('null results -> []', () => {
    assert.strictEqual(jsonFormatter(null), '[]');
  });

  it('undefined results -> []', () => {
    assert.strictEqual(jsonFormatter(), '[]');
  });

  it('non-array results -> []', () => {
    assert.strictEqual(jsonFormatter('string'), '[]');
    assert.strictEqual(jsonFormatter(42), '[]');
    assert.strictEqual(jsonFormatter({}), '[]');
  });

  it('empty array -> []', () => {
    assert.strictEqual(jsonFormatter([]), '[]');
  });

  it('results passed through verbatim', () => {
    const results = [
      {
        filePath: '/work/a.js',
        messages: [
          { ruleId: 'semi', message: 'Missing semicolon', severity: 2, line: 1, column: 1 },
        ],
        errorCount: 1,
        warningCount: 0,
      },
    ];
    const out = jsonFormatter(results);
    const parsed = JSON.parse(out);
    assert.deepStrictEqual(parsed, results);
  });

  it('data argument accepted and ignored', () => {
    const results = [{ filePath: '/work/a.js', messages: [], errorCount: 0, warningCount: 0 }];
    const data = { rulesMeta: { semi: { docs: { url: 'https://x' } } } };
    const out = jsonFormatter(results, data);
    const parsed = JSON.parse(out);
    assert.deepStrictEqual(parsed, results);
  });

  it('output is 2-space pretty-printed JSON', () => {
    const out = jsonFormatter([
      { filePath: '/work/a.js', messages: [], errorCount: 0, warningCount: 0 },
    ]);
    assert.ok(out.includes('\n  '));
    assert.ok(out.startsWith('[\n'));
  });
});
