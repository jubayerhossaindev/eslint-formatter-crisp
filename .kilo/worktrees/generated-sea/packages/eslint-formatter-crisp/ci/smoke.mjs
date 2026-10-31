/**
 * CI smoke runner — executed on GitHub Actions inside a sandbox that has the
 * packed eslint-formatter-crisp tgz + eslint@9 installed, on every supported
 * OS and Node version (see .github/workflows/ci.yml).
 *
 * Asserts, without any shell-specific tooling:
 *  1. the native module loaded on this host (the real win32/darwin/musl test),
 *  2. `eslint -f crisp` output contains the expected rule ids and icons,
 *  3. CRISP_ASCII=1 output is pure ASCII,
 *  4. the ESM entry formats too,
 *  5. zero raw control bytes in output (Phase 1 sanitizer holds everywhere).
 */
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(process.cwd() + '/package.json');
const crisp = require('eslint-formatter-crisp');
const eslintPkgPath = require.resolve('eslint/package.json');
const eslintBin = require('node:path').join(
  require('node:path').dirname(eslintPkgPath),
  'bin',
  'eslint.js',
);

let failures = 0;
const check = (name, cond, detail) => {
  if (cond) {
    console.log(`  PASS  ${name}`);
  } else {
    failures++;
    console.error(`  FAIL  ${name}${detail ? ' :: ' + detail : ''}`);
  }
};

function runEslint(env) {
  try {
    return execFileSync(process.execPath, [eslintBin, 'bad.js', '-f', 'crisp'], {
      cwd: process.cwd(),
      env: { ...process.env, ...env },
      encoding: 'utf8',
    });
  } catch (err) {
    // ESLint exits 1 when the fixture has lint errors — that is the EXPECTED
    // path here (we lint a deliberately broken file). Anything else is fatal.
    if (err && err.status === 1 && typeof err.stdout === 'string') return err.stdout;
    throw err;
  }
}

const reportPath = `smoke-${process.platform}-${process.arch}-node${process.versions.node}.txt`;

try {
  check('native module loads on this host', crisp.nativeLoaded() === true);

  const out = runEslint({});
  fs.writeFileSync(reportPath, out);

  check('no-undef rule id rendered', out.includes('no-undef'), out.slice(0, 200));
  check('no-unused-vars rule id rendered', out.includes('no-unused-vars'));
  check('error icon present', out.includes('✖') || out.includes('x'));
  check('file name rendered', out.includes('bad.js'));
  // eslint-disable-next-line no-control-regex -- intentional: asserting absence of control bytes
  check(
    'no raw C0/C1 control bytes in output',
    !/[\u0000-\u0008\u000B-\u001F\u007F-\u009F]/.test(out),
  );

  const ascii = runEslint({ CRISP_ASCII: '1' });
  check(
    'CRISP_ASCII=1 renders pure ASCII',
    [...ascii].every((c) => c.charCodeAt(0) < 128),
  );

  const esm = await import('eslint-formatter-crisp');
  check('ESM entry formats', typeof esm.default === 'function' && esm.default.length >= 1);
  check('ESM nativeLoaded true', esm.nativeLoaded() === true);
} catch (err) {
  failures++;
  console.error('FATAL:', err && err.stack ? err.stack : err);
  try {
    fs.writeFileSync(reportPath, String(err && err.stack ? err.stack : err));
  } catch {
    // report write failed too; nothing left to do
  }
}

console.log(
  `\nsmoke result (${process.platform}/${process.arch}/node ${process.versions.node}): ${failures === 0 ? 'ALL GREEN' : failures + ' failure(s)'}`,
);
process.exit(failures ? 1 : 0);
