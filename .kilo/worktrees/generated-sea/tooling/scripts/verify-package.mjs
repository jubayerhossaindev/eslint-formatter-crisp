import { spawnSync } from 'node:child_process';
import { readFile, access } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const packageRoot = path.join(repoRoot, 'packages/eslint-formatter-crisp');
const pkg = JSON.parse(await readFile(path.join(packageRoot, 'package.json'), 'utf8'));
const assert = (condition, message) => {
  if (!condition) throw new Error(message);
};

assert(pkg.name === 'eslint-formatter-crisp', 'Unexpected npm package name');
assert(pkg.engines?.node === '>=18', 'Public Node.js runtime floor changed unexpectedly');
assert(
  pkg.repository?.url === 'git+https://github.com/jubayerhossaindev/eslint-formatter-crisp.git',
  'Repository URL is not the target repository',
);
assert(pkg.exports?.['.']?.require === './index.cjs', 'CommonJS entry is not exported');
assert(pkg.exports?.['.']?.import === './index.mjs', 'ESM entry is not exported');

const required = [
  'index.cjs',
  'index.mjs',
  'index.d.ts',
  'json.cjs',
  'json.d.ts',
  'sarif.cjs',
  'sarif.d.ts',
  'LICENSE',
  'README.md',
  'CHANGELOG.md',
  'SECURITY.md',
  'crisp.darwin-arm64.node',
  'crisp.darwin-x64.node',
  'crisp.linux-arm64-gnu.node',
  'crisp.linux-arm64-musl.node',
  'crisp.linux-x64-gnu.node',
  'crisp.linux-x64-musl.node',
  'crisp.win32-x64-msvc.node',
];
for (const file of required) await access(path.join(packageRoot, file));
assert(
  (await readFile(path.join(repoRoot, 'LICENSE'))).equals(
    await readFile(path.join(packageRoot, 'LICENSE')),
  ),
  'Root and package LICENSE files differ',
);

const packed = spawnSync('npm', ['pack', '--dry-run', '--json'], {
  cwd: packageRoot,
  encoding: 'utf8',
});
assert(packed.status === 0, `npm pack --dry-run failed: ${packed.stderr}`);
const files = new Set(JSON.parse(packed.stdout)[0].files.map(({ path: file }) => file));
for (const file of required)
  assert(files.has(file), `Required file missing from npm tarball: ${file}`);
for (const forbidden of ['src/lib.rs', 'Cargo.toml', 'Cargo.lock', 'node_modules']) {
  assert(!files.has(forbidden), `Development file unexpectedly included in tarball: ${forbidden}`);
}
console.log(`Package metadata and tarball contents verified (${files.size} files).`);
