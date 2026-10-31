import { access, readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const required = [
  '.moon/workspace.yml',
  '.moon/toolchains.yml',
  '.moon/tasks/common.yml',
  'apps/docs/package.json',
  'apps/docs/package-lock.json',
  'packages/eslint-formatter-crisp/package.json',
  'packages/eslint-formatter-crisp/package-lock.json',
  'packages/eslint-formatter-crisp/Cargo.toml',
  'packages/eslint-formatter-crisp/Cargo.lock',
];
for (const relativePath of required) await access(path.join(repoRoot, relativePath));

for (const forbidden of ['package.json', 'package-lock.json']) {
  try {
    await access(path.join(repoRoot, forbidden));
    throw new Error(`Unexpected root package manager file: ${forbidden}`);
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
}

const workspace = await readFile(path.join(repoRoot, '.moon/workspace.yml'), 'utf8');
for (const expected of [
  'eslint-formatter-crisp:',
  'packages/eslint-formatter-crisp',
  'docs:',
  'apps/docs',
]) {
  if (!workspace.includes(expected)) throw new Error(`Moon workspace map is missing ${expected}`);
}
console.log('Monorepo layout and independent lockfiles verified.');
