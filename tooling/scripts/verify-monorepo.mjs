import { access, readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const required = [
  '.moon/workspace.yml',
  '.moon/toolchains.yml',
  '.moon/tasks/node.yml',
  '.moon/tasks/rust.yml',
  'crates/crisp-core/Cargo.toml',
  'crates/crisp-native/Cargo.toml',
  'packages/eslint-formatter-crisp/package.json',
  'packages/eslint-formatter-crisp/package-lock.json',
];
for (const relativePath of required) await access(path.join(repoRoot, relativePath));

// Root package manager files are part of the target layout (workspace root).
const rootPkg = path.join(repoRoot, 'package.json');
const rootLock = path.join(repoRoot, 'package-lock.json');
await access(rootPkg);
await access(rootLock);

const workspace = await readFile(path.join(repoRoot, '.moon/workspace.yml'), 'utf8');
for (const expected of [
  'crisp-core:',
  'crates/crisp-core',
  'crisp-native:',
  'crates/crisp-native',
  'eslint-formatter-crisp:',
  'packages/eslint-formatter-crisp',
]) {
  if (!workspace.includes(expected)) throw new Error(`Moon workspace map is missing ${expected}`);
}
console.log('Monorepo layout and independent lockfiles verified.');
