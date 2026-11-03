import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import crisp, { detectColorLevel, detectLinks, format, nativeLoaded } from '../index.mjs';

describe('index.mjs (ESM entry)', () => {
  it('default export is function', () => {
    assert.strictEqual(typeof crisp, 'function');
    assert.strictEqual(crisp, format);
  });

  it('named exports present', () => {
    assert.strictEqual(typeof detectColorLevel, 'function');
    assert.strictEqual(typeof detectLinks, 'function');
    assert.strictEqual(typeof nativeLoaded, 'function');
  });

  it('nativeLoaded returns boolean', () => {
    assert.strictEqual(typeof nativeLoaded(), 'boolean');
  });

  it('format calls through to CJS', () => {
    if (nativeLoaded()) {
      const out = format([]);
      assert.strictEqual(typeof out, 'string');
    } else {
      assert.throws(() => format([]), /native/);
    }
  });

  it('crisp.format is always defined (no nullish fallback needed)', async () => {
    const cjsModule = (await import('../index.mjs')).default;
    // module.exports.format is always set in index.mjs, so the ESM
    // `crisp.format ?? crisp` fallback is unreachable. Verify the invariant.
    assert.strictEqual(typeof cjsModule.format, 'function');
    // eslint-disable-next-line import/no-named-as-default-member -- testing CJS/ESM interop equivalence
    assert.strictEqual(crisp.format, cjsModule.format);
  });
});
