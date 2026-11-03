import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';

import crisp, { format, detectColorLevel, detectLinks, nativeLoaded } from '../index.mjs';

const __dirname = fileURLToPath(new URL('.', import.meta.url));

const { _internal } = crisp;
const {
  sanitizeControl,
  toU32,
  toSeverity,
  normalizeData,
  normalizeMessage,
  resolveNative,
  currentTriples,
  detectMaxWidth,
  parseThemeJson,
  detectTheme,
  detectMaxMessages,
} = _internal;

// ---------------------------------------------------------------------------
// detectColorLevel — full precedence chain
// ---------------------------------------------------------------------------

describe('detectColorLevel', () => {
  it('FORCE_COLOR=1 -> 1', () => {
    const prev = process.env.FORCE_COLOR;
    process.env.FORCE_COLOR = '1';
    try {
      assert.strictEqual(detectColorLevel(process.stdout), 1);
    } finally {
      process.env.FORCE_COLOR = prev;
    }
  });

  it('FORCE_COLOR=true -> 1', () => {
    const prev = process.env.FORCE_COLOR;
    process.env.FORCE_COLOR = 'true';
    try {
      assert.strictEqual(detectColorLevel(process.stdout), 1);
    } finally {
      process.env.FORCE_COLOR = prev;
    }
  });

  it('FORCE_COLOR="" -> 1', () => {
    const prev = process.env.FORCE_COLOR;
    process.env.FORCE_COLOR = '';
    try {
      assert.strictEqual(detectColorLevel(process.stdout), 1);
    } finally {
      process.env.FORCE_COLOR = prev;
    }
  });

  it('FORCE_COLOR=2 -> 2', () => {
    const prev = process.env.FORCE_COLOR;
    process.env.FORCE_COLOR = '2';
    try {
      assert.strictEqual(detectColorLevel(process.stdout), 2);
    } finally {
      process.env.FORCE_COLOR = prev;
    }
  });

  it('FORCE_COLOR=3 -> 3', () => {
    const prev = process.env.FORCE_COLOR;
    process.env.FORCE_COLOR = '3';
    try {
      assert.strictEqual(detectColorLevel(process.stdout), 3);
    } finally {
      process.env.FORCE_COLOR = prev;
    }
  });

  it('FORCE_COLOR=0 -> 0', () => {
    const prev = process.env.FORCE_COLOR;
    process.env.FORCE_COLOR = '0';
    try {
      assert.strictEqual(detectColorLevel(process.stdout), 0);
    } finally {
      process.env.FORCE_COLOR = prev;
    }
  });

  it('FORCE_COLOR=false -> 0', () => {
    const prev = process.env.FORCE_COLOR;
    process.env.FORCE_COLOR = 'false';
    try {
      assert.strictEqual(detectColorLevel(process.stdout), 0);
    } finally {
      process.env.FORCE_COLOR = prev;
    }
  });

  it('FORCE_COLOR=garbage -> 0', () => {
    const prev = process.env.FORCE_COLOR;
    process.env.FORCE_COLOR = 'garbage';
    try {
      assert.strictEqual(detectColorLevel(process.stdout), 0);
    } finally {
      process.env.FORCE_COLOR = prev;
    }
  });

  it('NO_COLOR wins over CI', () => {
    const prevForce = process.env.FORCE_COLOR;
    const prevNo = process.env.NO_COLOR;
    const prevCi = process.env.CI;
    delete process.env.FORCE_COLOR;
    process.env.NO_COLOR = '1';
    process.env.CI = '1';
    try {
      assert.strictEqual(detectColorLevel(process.stdout), 0);
    } finally {
      if (prevForce === undefined) delete process.env.FORCE_COLOR;
      else process.env.FORCE_COLOR = prevForce;
      if (prevNo === undefined) delete process.env.NO_COLOR;
      else process.env.NO_COLOR = prevNo;
      if (prevCi === undefined) delete process.env.CI;
      else process.env.CI = prevCi;
    }
  });

  it('TERM=dumb -> 0', () => {
    const prevForce = process.env.FORCE_COLOR;
    const prevTerm = process.env.TERM;
    delete process.env.FORCE_COLOR;
    process.env.TERM = 'dumb';
    try {
      assert.strictEqual(detectColorLevel(process.stdout), 0);
    } finally {
      if (prevForce === undefined) delete process.env.FORCE_COLOR;
      else process.env.FORCE_COLOR = prevForce;
      if (prevTerm === undefined) delete process.env.TERM;
      else process.env.TERM = prevTerm;
    }
  });

  it('CI=1 -> 1', () => {
    const prevForce = process.env.FORCE_COLOR;
    const prevNo = process.env.NO_COLOR;
    const prevCi = process.env.CI;
    const prevTerm = process.env.TERM;
    delete process.env.FORCE_COLOR;
    delete process.env.NO_COLOR;
    delete process.env.TERM;
    process.env.CI = '1';
    try {
      assert.strictEqual(detectColorLevel(process.stdout), 1);
    } finally {
      if (prevForce === undefined) delete process.env.FORCE_COLOR;
      else process.env.FORCE_COLOR = prevForce;
      if (prevNo === undefined) delete process.env.NO_COLOR;
      else process.env.NO_COLOR = prevNo;
      if (prevCi === undefined) delete process.env.CI;
      else process.env.CI = prevCi;
      if (prevTerm === undefined) delete process.env.TERM;
      else process.env.TERM = prevTerm;
    }
  });

  it('stream.isTTY true -> 1', () => {
    const prevForce = process.env.FORCE_COLOR;
    const prevNo = process.env.NO_COLOR;
    const prevCi = process.env.CI;
    const prevTerm = process.env.TERM;
    delete process.env.FORCE_COLOR;
    delete process.env.NO_COLOR;
    delete process.env.CI;
    delete process.env.TERM;
    try {
      assert.strictEqual(detectColorLevel({ isTTY: true }), 1);
    } finally {
      if (prevForce === undefined) delete process.env.FORCE_COLOR;
      else process.env.FORCE_COLOR = prevForce;
      if (prevNo === undefined) delete process.env.NO_COLOR;
      else process.env.NO_COLOR = prevNo;
      if (prevCi === undefined) delete process.env.CI;
      else process.env.CI = prevCi;
      if (prevTerm === undefined) delete process.env.TERM;
      else process.env.TERM = prevTerm;
    }
  });

  it('stream.isTTY false -> 0', () => {
    const prevForce = process.env.FORCE_COLOR;
    const prevNo = process.env.NO_COLOR;
    const prevCi = process.env.CI;
    const prevTerm = process.env.TERM;
    delete process.env.FORCE_COLOR;
    delete process.env.NO_COLOR;
    delete process.env.CI;
    delete process.env.TERM;
    try {
      assert.strictEqual(detectColorLevel({ isTTY: false }), 0);
    } finally {
      if (prevForce === undefined) delete process.env.FORCE_COLOR;
      else process.env.FORCE_COLOR = prevForce;
      if (prevNo === undefined) delete process.env.NO_COLOR;
      else process.env.NO_COLOR = prevNo;
      if (prevCi === undefined) delete process.env.CI;
      else process.env.CI = prevCi;
      if (prevTerm === undefined) delete process.env.TERM;
      else process.env.TERM = prevTerm;
    }
  });

  it('no stream -> 0', () => {
    const prevForce = process.env.FORCE_COLOR;
    const prevNo = process.env.NO_COLOR;
    const prevCi = process.env.CI;
    const prevTerm = process.env.TERM;
    delete process.env.FORCE_COLOR;
    delete process.env.NO_COLOR;
    delete process.env.CI;
    delete process.env.TERM;
    try {
      assert.strictEqual(detectColorLevel(), 0);
    } finally {
      if (prevForce === undefined) delete process.env.FORCE_COLOR;
      else process.env.FORCE_COLOR = prevForce;
      if (prevNo === undefined) delete process.env.NO_COLOR;
      else process.env.NO_COLOR = prevNo;
      if (prevCi === undefined) delete process.env.CI;
      else process.env.CI = prevCi;
      if (prevTerm === undefined) delete process.env.TERM;
      else process.env.TERM = prevTerm;
    }
  });
});

// ---------------------------------------------------------------------------
// detectLinks
// ---------------------------------------------------------------------------

describe('detectLinks', () => {
  it('colorLevel 0 -> false', () => {
    assert.strictEqual(detectLinks({ isTTY: true }, 0), false);
  });

  it('CI true -> false', () => {
    const prevCi = process.env.CI;
    process.env.CI = '1';
    try {
      assert.strictEqual(detectLinks({ isTTY: true }, 1), false);
    } finally {
      if (prevCi === undefined) delete process.env.CI;
      else process.env.CI = prevCi;
    }
  });

  it('TERM=dumb -> false', () => {
    const prevTerm = process.env.TERM;
    process.env.TERM = 'dumb';
    try {
      assert.strictEqual(detectLinks({ isTTY: true }, 1), false);
    } finally {
      if (prevTerm === undefined) delete process.env.TERM;
      else process.env.TERM = prevTerm;
    }
  });

  it('no stream -> false', () => {
    assert.strictEqual(detectLinks(undefined, 1), false);
  });

  it('stream not TTY -> false', () => {
    assert.strictEqual(detectLinks({ isTTY: false }, 1), false);
  });

  it('stream TTY + colorLevel > 0 -> true', () => {
    const prevCI = process.env.CI;
    delete process.env.CI;
    try {
      assert.strictEqual(detectLinks({ isTTY: true }, 1), true);
    } finally {
      if (prevCI === undefined) delete process.env.CI;
      else process.env.CI = prevCI;
    }
  });
});

// ---------------------------------------------------------------------------
// sanitizeControl
// ---------------------------------------------------------------------------

describe('sanitizeControl', () => {
  it('non-string passes through', () => {
    assert.strictEqual(sanitizeControl(null), null);
    assert.strictEqual(sanitizeControl(), undefined);
    assert.strictEqual(sanitizeControl(42), 42);
  });

  it('clean string unchanged', () => {
    assert.strictEqual(sanitizeControl('hello world'), 'hello world');
  });

  it('C0 control chars replaced', () => {
    const out = sanitizeControl('a\x01b\x02c');
    // eslint-disable-next-line no-control-regex -- test asserts control chars are replaced
    assert.ok(!/[\u0001\u0002]/.test(out));
    assert.ok(out.includes('\u2401')); // ␁
    assert.ok(out.includes('\u2402')); // ␂
  });

  it('tab and newline preserved', () => {
    assert.strictEqual(sanitizeControl('a\tb\nc'), 'a\tb\nc');
  });

  it('DEL replaced with ␡', () => {
    assert.strictEqual(sanitizeControl('a\x7f b'), 'a\u2421 b');
  });

  it('C1 range replaced with replacement char', () => {
    const out = sanitizeControl('a\x80b\x9fc');
    assert.ok(out.includes('\uFFFD'));
    assert.ok(!/[\u0080-\u009F]/.test(out));
  });

  it('ESC replaced', () => {
    const out = sanitizeControl('\x1b[31mred');
    assert.ok(!out.includes('\x1b'));
    assert.ok(out.includes('\u241b')); // ESC -> ␛ (U+241B)
  });
});

// ---------------------------------------------------------------------------
// toU32
// ---------------------------------------------------------------------------

describe('toU32', () => {
  it('valid number', () => {
    assert.strictEqual(toU32(42), 42);
    assert.strictEqual(toU32(0), 0);
    assert.strictEqual(toU32(0xffffffff), 0xffffffff);
  });

  it('float floored', () => {
    assert.strictEqual(toU32(3.9), 3);
  });

  it('negative -> undefined', () => {
    assert.strictEqual(toU32(-1), undefined);
  });

  it('out of range -> undefined', () => {
    assert.strictEqual(toU32(0x100000000), undefined);
  });

  it('NaN -> undefined', () => {
    assert.strictEqual(toU32(NaN), undefined);
  });

  it('Infinity -> undefined', () => {
    assert.strictEqual(toU32(Infinity), undefined);
  });

  it('non-number -> undefined', () => {
    assert.strictEqual(toU32('42'), undefined);
    assert.strictEqual(toU32(null), undefined);
    assert.strictEqual(toU32(), undefined);
  });
});

// ---------------------------------------------------------------------------
// toSeverity
// ---------------------------------------------------------------------------

describe('toSeverity', () => {
  it('1 -> 1', () => assert.strictEqual(toSeverity(1), 1));
  it('2 -> 2', () => assert.strictEqual(toSeverity(2), 2));
  it('0 -> 0', () => assert.strictEqual(toSeverity(0), 0));
  it('negative -> undefined', () => assert.strictEqual(toSeverity(-1), undefined));
  it('above i32 max -> undefined', () => assert.strictEqual(toSeverity(0x80000000), undefined));
  it('NaN -> undefined', () => assert.strictEqual(toSeverity(NaN), undefined));
  it('non-number -> undefined', () => assert.strictEqual(toSeverity('2'), undefined));
});

// ---------------------------------------------------------------------------
// normalizeMessage
// ---------------------------------------------------------------------------

describe('normalizeMessage', () => {
  it('valid message normalized', () => {
    const out = normalizeMessage(
      {
        message: 'hi',
        severity: 2,
        line: 1,
        column: 1,
        ruleId: 'semi',
        fatal: false,
        fix: { range: [0, 0] },
      },
      0,
      0,
      false,
    );
    assert.strictEqual(out.message, 'hi');
    assert.strictEqual(out.severity, 2);
    assert.strictEqual(out.line, 1);
    assert.strictEqual(out.column, 1);
    assert.strictEqual(out.ruleId, 'semi');
    assert.strictEqual(out.fatal, false);
    assert.strictEqual(out.fix, true);
  });

  it('null message throws TypeError', () => {
    assert.throws(() => normalizeMessage(null, 0, 0, false), TypeError);
  });

  it('array message throws TypeError', () => {
    assert.throws(() => normalizeMessage([], 0, 0, false), TypeError);
  });

  it('non-string message throws TypeError', () => {
    assert.throws(() => normalizeMessage({ message: 42 }, 0, 0, false), TypeError);
  });

  it('suggestion extracted when wantSuggestions', () => {
    const out = normalizeMessage({ message: 'hi', suggestions: [{ desc: 'fix it' }] }, 0, 0, true);
    assert.strictEqual(out.suggestion, 'fix it');
  });

  it('suggestion not extracted when wantSuggestions false', () => {
    const out = normalizeMessage({ message: 'hi', suggestions: [{ desc: 'fix it' }] }, 0, 0, false);
    assert.strictEqual(out.suggestion, undefined);
  });

  it('null fields become undefined', () => {
    const out = normalizeMessage(
      { message: 'hi', line: null, column: null, ruleId: null, fatal: null },
      0,
      0,
      false,
    );
    assert.strictEqual(out.line, undefined);
    assert.strictEqual(out.column, undefined);
    assert.strictEqual(out.ruleId, undefined);
    assert.strictEqual(out.fatal, undefined);
  });

  it('message sanitized', () => {
    const out = normalizeMessage({ message: 'a\x01b' }, 0, 0, false);
    assert.ok(out.message.includes('\u2401'));
  });
});

// ---------------------------------------------------------------------------
// normalizeData
// ---------------------------------------------------------------------------

describe('normalizeData', () => {
  it('null data -> empty rulesMeta', () => {
    assert.deepStrictEqual(normalizeData(null), { rulesMeta: {} });
    assert.deepStrictEqual(normalizeData(), { rulesMeta: {} });
  });

  it('non-object data -> empty rulesMeta', () => {
    assert.deepStrictEqual(normalizeData('string'), { rulesMeta: {} });
    assert.deepStrictEqual(normalizeData(42), { rulesMeta: {} });
  });

  it('null rulesMeta -> empty', () => {
    assert.deepStrictEqual(normalizeData({ rulesMeta: null }), { rulesMeta: {} });
  });

  it('non-object rulesMeta -> empty', () => {
    assert.deepStrictEqual(normalizeData({ rulesMeta: 'x' }), { rulesMeta: {} });
  });

  it('well-shaped meta preserved', () => {
    const data = {
      rulesMeta: {
        semi: { docs: { url: 'https://eslint.org/rules/semi', deprecated: true } },
      },
    };
    const out = normalizeData(data);
    assert.strictEqual(out.rulesMeta.semi.docs.url, 'https://eslint.org/rules/semi');
    assert.strictEqual(out.rulesMeta.semi.docs.deprecated, true);
  });

  it('malformed nodes skipped', () => {
    const data = {
      rulesMeta: {
        good: { docs: { url: 'https://x' } },
        bad: null,
        noDocs: {},
        badDocs: { docs: 'string' },
        noUrl: { docs: { deprecated: true } },
      },
    };
    const out = normalizeData(data);
    assert.strictEqual(out.rulesMeta.good.docs.url, 'https://x');
    assert.strictEqual(out.rulesMeta.bad, undefined);
    assert.strictEqual(out.rulesMeta.noDocs, undefined);
    assert.strictEqual(out.rulesMeta.badDocs, undefined);
    assert.strictEqual(out.rulesMeta.noUrl.docs.deprecated, true);
    assert.strictEqual(out.rulesMeta.noUrl.docs.url, undefined);
  });
});

// ---------------------------------------------------------------------------
// parseThemeJson
// ---------------------------------------------------------------------------

describe('parseThemeJson', () => {
  it('valid JSON object with hex colors', () => {
    const out = parseThemeJson('{"fruitA":"#ff0000","fruitB":"#00ff00"}');
    assert.deepStrictEqual(out, { fruitA: 'ff0000', fruitB: '00ff00' });
  });

  it('hex without # accepted', () => {
    const out = parseThemeJson('{"cristalA":"abcdef"}');
    assert.deepStrictEqual(out, { cristalA: 'abcdef' });
  });

  it('mono true', () => {
    const out = parseThemeJson('{"mono":true}');
    assert.deepStrictEqual(out, { mono: true });
  });

  it('invalid JSON -> undefined', () => {
    assert.strictEqual(parseThemeJson('not json'), undefined);
    assert.strictEqual(parseThemeJson('{'), undefined);
  });

  it('null -> undefined', () => {
    assert.strictEqual(parseThemeJson('null'), undefined);
  });

  it('array -> undefined', () => {
    assert.strictEqual(parseThemeJson('[]'), undefined);
  });

  it('empty object -> undefined', () => {
    assert.strictEqual(parseThemeJson('{}'), undefined);
  });

  it('invalid hex ignored', () => {
    const out = parseThemeJson('{"fruitA":"#zzz","fruitB":"#123456"}');
    assert.deepStrictEqual(out, { fruitB: '123456' });
  });
});

// ---------------------------------------------------------------------------
// detectTheme
// ---------------------------------------------------------------------------

describe('detectTheme', () => {
  it('undefined -> undefined', () => {
    const prev = process.env.CRISP_THEME;
    const prevMo = process.env.MO_THEME;
    delete process.env.CRISP_THEME;
    delete process.env.MO_THEME;
    try {
      assert.strictEqual(detectTheme(), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_THEME;
      else process.env.CRISP_THEME = prev;
      if (prevMo === undefined) delete process.env.MO_THEME;
      else process.env.MO_THEME = prevMo;
    }
  });

  it('empty string -> undefined', () => {
    const prev = process.env.CRISP_THEME;
    process.env.CRISP_THEME = '';
    try {
      assert.strictEqual(detectTheme(), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_THEME;
      else process.env.CRISP_THEME = prev;
    }
  });

  it('one-dark-pro -> undefined (no-op)', () => {
    const prev = process.env.CRISP_THEME;
    process.env.CRISP_THEME = 'one-dark-pro';
    try {
      assert.strictEqual(detectTheme(), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_THEME;
      else process.env.CRISP_THEME = prev;
    }
  });

  it('mono -> { mono: true }', () => {
    const prev = process.env.CRISP_THEME;
    process.env.CRISP_THEME = 'mono';
    try {
      assert.deepStrictEqual(detectTheme(), { mono: true });
    } finally {
      if (prev === undefined) delete process.env.CRISP_THEME;
      else process.env.CRISP_THEME = prev;
    }
  });

  it('unknown name -> undefined', () => {
    const prev = process.env.CRISP_THEME;
    process.env.CRISP_THEME = 'solarized';
    try {
      assert.strictEqual(detectTheme(), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_THEME;
      else process.env.CRISP_THEME = prev;
    }
  });

  it('JSON object -> parsed theme', () => {
    const prev = process.env.CRISP_THEME;
    process.env.CRISP_THEME = '{"fruitA":"#ff0000"}';
    try {
      assert.deepStrictEqual(detectTheme(), { fruitA: 'ff0000' });
    } finally {
      if (prev === undefined) delete process.env.CRISP_THEME;
      else process.env.CRISP_THEME = prev;
    }
  });

  it('MO_THEME fallback', () => {
    const prev = process.env.CRISP_THEME;
    const prevMo = process.env.MO_THEME;
    delete process.env.CRISP_THEME;
    process.env.MO_THEME = 'mono';
    try {
      assert.deepStrictEqual(detectTheme(), { mono: true });
    } finally {
      if (prev === undefined) delete process.env.CRISP_THEME;
      else process.env.CRISP_THEME = prev;
      if (prevMo === undefined) delete process.env.MO_THEME;
      else process.env.MO_THEME = prevMo;
    }
  });
});

// ---------------------------------------------------------------------------
// detectMaxWidth
// ---------------------------------------------------------------------------

describe('detectMaxWidth', () => {
  it('CRISP_FULL_LINES=1 -> undefined', () => {
    const prev = process.env.CRISP_FULL_LINES;
    process.env.CRISP_FULL_LINES = '1';
    try {
      assert.strictEqual(detectMaxWidth({ isTTY: true, columns: 80 }), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_FULL_LINES;
      else process.env.CRISP_FULL_LINES = prev;
    }
  });

  it('COLUMNS=80 valid -> 80', () => {
    const prev = process.env.COLUMNS;
    process.env.COLUMNS = '80';
    try {
      assert.strictEqual(detectMaxWidth({ isTTY: false }), 80);
    } finally {
      if (prev === undefined) delete process.env.COLUMNS;
      else process.env.COLUMNS = prev;
    }
  });

  it('COLUMNS invalid -> falls through to TTY', () => {
    const prev = process.env.COLUMNS;
    process.env.COLUMNS = 'abc';
    try {
      assert.strictEqual(detectMaxWidth({ isTTY: true, columns: 100 }), 100);
    } finally {
      if (prev === undefined) delete process.env.COLUMNS;
      else process.env.COLUMNS = prev;
    }
  });

  it('COLUMNS < 20 -> falls through', () => {
    const prev = process.env.COLUMNS;
    process.env.COLUMNS = '10';
    try {
      assert.strictEqual(detectMaxWidth({ isTTY: true, columns: 100 }), 100);
    } finally {
      if (prev === undefined) delete process.env.COLUMNS;
      else process.env.COLUMNS = prev;
    }
  });

  it('COLUMNS > 10000 -> falls through', () => {
    const prev = process.env.COLUMNS;
    process.env.COLUMNS = '20000';
    try {
      assert.strictEqual(detectMaxWidth({ isTTY: true, columns: 100 }), 100);
    } finally {
      if (prev === undefined) delete process.env.COLUMNS;
      else process.env.COLUMNS = prev;
    }
  });

  it('no COLUMNS, TTY columns 80 -> 80', () => {
    const prev = process.env.COLUMNS;
    delete process.env.COLUMNS;
    try {
      assert.strictEqual(detectMaxWidth({ isTTY: true, columns: 80 }), 80);
    } finally {
      if (prev === undefined) delete process.env.COLUMNS;
      else process.env.COLUMNS = prev;
    }
  });

  it('no stream -> undefined', () => {
    const prev = process.env.COLUMNS;
    delete process.env.COLUMNS;
    try {
      assert.strictEqual(detectMaxWidth(), undefined);
    } finally {
      if (prev === undefined) delete process.env.COLUMNS;
      else process.env.COLUMNS = prev;
    }
  });

  it('stream not TTY -> undefined', () => {
    const prev = process.env.COLUMNS;
    delete process.env.COLUMNS;
    try {
      assert.strictEqual(detectMaxWidth({ isTTY: false }), undefined);
    } finally {
      if (prev === undefined) delete process.env.COLUMNS;
      else process.env.COLUMNS = prev;
    }
  });
});

// ---------------------------------------------------------------------------
// detectMaxMessages
// ---------------------------------------------------------------------------

describe('detectMaxMessages', () => {
  it('undefined -> undefined', () => {
    const prev = process.env.CRISP_MAX_MESSAGES;
    delete process.env.CRISP_MAX_MESSAGES;
    try {
      assert.strictEqual(detectMaxMessages(), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_MAX_MESSAGES;
      else process.env.CRISP_MAX_MESSAGES = prev;
    }
  });

  it('"0" -> 0', () => {
    const prev = process.env.CRISP_MAX_MESSAGES;
    process.env.CRISP_MAX_MESSAGES = '0';
    try {
      assert.strictEqual(detectMaxMessages(), 0);
    } finally {
      if (prev === undefined) delete process.env.CRISP_MAX_MESSAGES;
      else process.env.CRISP_MAX_MESSAGES = prev;
    }
  });

  it('"42" -> 42', () => {
    const prev = process.env.CRISP_MAX_MESSAGES;
    process.env.CRISP_MAX_MESSAGES = '42';
    try {
      assert.strictEqual(detectMaxMessages(), 42);
    } finally {
      if (prev === undefined) delete process.env.CRISP_MAX_MESSAGES;
      else process.env.CRISP_MAX_MESSAGES = prev;
    }
  });

  it('empty string -> undefined', () => {
    const prev = process.env.CRISP_MAX_MESSAGES;
    process.env.CRISP_MAX_MESSAGES = '';
    try {
      assert.strictEqual(detectMaxMessages(), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_MAX_MESSAGES;
      else process.env.CRISP_MAX_MESSAGES = prev;
    }
  });

  it('"abc" -> undefined', () => {
    const prev = process.env.CRISP_MAX_MESSAGES;
    process.env.CRISP_MAX_MESSAGES = 'abc';
    try {
      assert.strictEqual(detectMaxMessages(), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_MAX_MESSAGES;
      else process.env.CRISP_MAX_MESSAGES = prev;
    }
  });

  it('"-1" -> undefined', () => {
    const prev = process.env.CRISP_MAX_MESSAGES;
    process.env.CRISP_MAX_MESSAGES = '-1';
    try {
      assert.strictEqual(detectMaxMessages(), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_MAX_MESSAGES;
      else process.env.CRISP_MAX_MESSAGES = prev;
    }
  });

  it('"3.5" -> undefined', () => {
    const prev = process.env.CRISP_MAX_MESSAGES;
    process.env.CRISP_MAX_MESSAGES = '3.5';
    try {
      assert.strictEqual(detectMaxMessages(), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_MAX_MESSAGES;
      else process.env.CRISP_MAX_MESSAGES = prev;
    }
  });

  it('" 3" -> undefined (leading space)', () => {
    const prev = process.env.CRISP_MAX_MESSAGES;
    process.env.CRISP_MAX_MESSAGES = ' 3';
    try {
      assert.strictEqual(detectMaxMessages(), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_MAX_MESSAGES;
      else process.env.CRISP_MAX_MESSAGES = prev;
    }
  });

  it('above u32 max -> undefined', () => {
    const prev = process.env.CRISP_MAX_MESSAGES;
    process.env.CRISP_MAX_MESSAGES = '4294967296';
    try {
      assert.strictEqual(detectMaxMessages(), undefined);
    } finally {
      if (prev === undefined) delete process.env.CRISP_MAX_MESSAGES;
      else process.env.CRISP_MAX_MESSAGES = prev;
    }
  });
});

// ---------------------------------------------------------------------------
// currentTriples
// ---------------------------------------------------------------------------

describe('currentTriples', () => {
  it('linux returns gnu + musl', () => {
    const prev = process.platform;
    Object.defineProperty(process, 'platform', { value: 'linux', configurable: true });
    Object.defineProperty(process, 'arch', { value: 'x64', configurable: true });
    try {
      const t = currentTriples();
      assert.ok(t.includes('linux-x64-gnu'));
      assert.ok(t.includes('linux-x64-musl'));
      assert.strictEqual(t.length, 2);
    } finally {
      Object.defineProperty(process, 'platform', { value: prev, configurable: true });
    }
  });

  it('darwin returns darwin-arch', () => {
    const prev = process.platform;
    const prevArch = process.arch;
    Object.defineProperty(process, 'platform', { value: 'darwin', configurable: true });
    Object.defineProperty(process, 'arch', { value: 'arm64', configurable: true });
    try {
      const t = currentTriples();
      assert.deepStrictEqual(t, ['darwin-arm64']);
    } finally {
      Object.defineProperty(process, 'platform', { value: prev, configurable: true });
      Object.defineProperty(process, 'arch', { value: prevArch, configurable: true });
    }
  });

  it('win32 returns win32-arch-msvc', () => {
    const prev = process.platform;
    const prevArch = process.arch;
    Object.defineProperty(process, 'platform', { value: 'win32', configurable: true });
    Object.defineProperty(process, 'arch', { value: 'x64', configurable: true });
    try {
      const t = currentTriples();
      assert.deepStrictEqual(t, ['win32-x64-msvc']);
    } finally {
      Object.defineProperty(process, 'platform', { value: prev, configurable: true });
      Object.defineProperty(process, 'arch', { value: prevArch, configurable: true });
    }
  });
});

// ---------------------------------------------------------------------------
// resolveNative
// ---------------------------------------------------------------------------

describe('resolveNative', () => {
  it('first candidate exists -> returns it', () => {
    const exists = (c) => c === 'crisp.linux-x64-gnu.node';
    const requireFn = (c) => ({ loaded: c });
    const result = resolveNative(
      ['crisp.linux-x64-gnu.node', 'crisp.linux-x64-musl.node'],
      exists,
      requireFn,
    );
    assert.strictEqual(result.native.loaded, 'crisp.linux-x64-gnu.node');
    assert.deepStrictEqual(result.tried, ['crisp.linux-x64-gnu.node']);
    assert.deepStrictEqual(result.failed, []);
    assert.strictEqual(result.loadError, null);
  });

  it('first fails, second succeeds -> falls through', () => {
    const exists = () => true;
    const requireFn = (c) => {
      if (c === 'a.node') throw new Error('boom');
      return { loaded: c };
    };
    const result = resolveNative(['a.node', 'b.node'], exists, requireFn);
    assert.strictEqual(result.native.loaded, 'b.node');
    assert.deepStrictEqual(result.tried, ['a.node', 'b.node']);
    assert.deepStrictEqual(result.failed, ['a.node']);
    assert.ok(result.loadError instanceof Error);
  });

  it('none exist -> loadError with message', () => {
    const exists = () => false;
    const requireFn = () => {
      throw new Error('never');
    };
    const result = resolveNative(['a.node', 'b.node'], exists, requireFn);
    assert.strictEqual(result.native, null);
    assert.ok(result.loadError instanceof Error);
    assert.ok(result.loadError.message.includes('no prebuilt native binary'));
    assert.deepStrictEqual(result.tried, []);
    assert.deepStrictEqual(result.failed, []);
  });

  it('empty candidates -> loadError', () => {
    const exists = () => false;
    const requireFn = () => {
      throw new Error('never');
    };
    const result = resolveNative([], exists, requireFn);
    assert.strictEqual(result.native, null);
    assert.strictEqual(result.loadError, null);
    assert.deepStrictEqual(result.tried, []);
  });
});

// ---------------------------------------------------------------------------
// nativeLoaded
// ---------------------------------------------------------------------------

describe('nativeLoaded', () => {
  it('returns boolean', () => {
    assert.strictEqual(typeof nativeLoaded(), 'boolean');
  });
});

// ---------------------------------------------------------------------------
// formatResults — CJS entry
// ---------------------------------------------------------------------------

describe('formatResults (CJS)', () => {
  it('throws on non-array results', () => {
    assert.throws(() => crisp(null), TypeError);
    assert.throws(() => crisp('string'), TypeError);
    assert.throws(() => crisp(42), TypeError);
  });

  it('throws on null element', () => {
    assert.throws(() => crisp([null]), TypeError);
  });

  it('throws on non-object element', () => {
    assert.throws(() => crisp([42]), TypeError);
  });

  it('throws on array element', () => {
    assert.throws(() => crisp([[]]), TypeError);
  });

  it('empty array returns empty string if native loaded', () => {
    if (nativeLoaded()) {
      const out = crisp([]);
      assert.strictEqual(typeof out, 'string');
    } else {
      assert.throws(() => crisp([]), /native/);
    }
  });
});

// ---------------------------------------------------------------------------
// module.exports shape
// ---------------------------------------------------------------------------

describe('module.exports shape', () => {
  it('default export is function', () => {
    assert.strictEqual(typeof crisp, 'function');
    assert.strictEqual(typeof crisp.default, 'function');
    assert.strictEqual(typeof format, 'function');
  });

  it('named exports present', () => {
    assert.strictEqual(typeof detectColorLevel, 'function');
    assert.strictEqual(typeof detectLinks, 'function');
    assert.strictEqual(typeof nativeLoaded, 'function');
  });

  it('_internal is frozen and non-enumerable', () => {
    const desc = Object.getOwnPropertyDescriptor(crisp, '_internal');
    assert.strictEqual(desc.enumerable, false);
    assert.ok(Object.isFrozen(crisp._internal));
  });
});

// ---------------------------------------------------------------------------
// assertLoaded error path — native module missing
// ---------------------------------------------------------------------------

describe('assertLoaded error path', () => {
  it('resolveNative returns null native + loadError when no candidate exists', () => {
    const { resolveNative } = crisp._internal;
    const res = resolveNative(
      ['/fake/never.node'],
      () => false,
      () => {
        throw new Error('should not be called');
      },
    );
    assert.strictEqual(res.native, null);
    assert.ok(res.loadError instanceof Error);
    assert.ok(res.loadError.message.includes('no prebuilt native binary'));
  });

  it('resolveNative falls through to next candidate on require() failure', () => {
    const { resolveNative } = crisp._internal;
    const exists = () => true;
    const requireFn = (p) => {
      if (p.includes('gnu')) throw new Error('simulated load failure');
      return { marker: 'loaded' };
    };
    const res = resolveNative(
      ['/fake/crisp.linux-x64-gnu.node', '/fake/crisp.linux-x64-musl.node'],
      exists,
      requireFn,
    );
    assert.strictEqual(res.native && res.native.marker, 'loaded');
    assert.ok(res.loadError instanceof Error);
    assert.equal(res.failed.length, 1);
  });

  it('resolveNative empty candidates returns null native', () => {
    const { resolveNative } = crisp._internal;
    const res = resolveNative(
      [],
      () => false,
      () => {
        throw new Error('never');
      },
    );
    assert.strictEqual(res.native, null);
    assert.strictEqual(res.loadError, null);
  });
});
