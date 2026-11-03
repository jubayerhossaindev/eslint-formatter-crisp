import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const packageRoot = path.join(repoRoot, 'packages/eslint-formatter-crisp');
const binaries = [
  'crisp.darwin-arm64.node',
  'crisp.darwin-x64.node',
  'crisp.linux-arm64-gnu.node',
  'crisp.linux-arm64-musl.node',
  'crisp.linux-x64-gnu.node',
  'crisp.linux-x64-musl.node',
  'crisp.win32-x64-msvc.node',
];

const lines = await Promise.all(
  binaries.map(async (name) => {
    const bytes = await readFile(path.join(packageRoot, name));
    const digest = createHash('sha256').update(bytes).digest('hex');
    return `${digest}  ${name}`;
  }),
);
const report = `${lines.join('\n')}\n`;
const destination = process.argv[2];
if (destination) {
  const outputPath = path.resolve(repoRoot, destination);
  await writeFile(outputPath, report);
  console.log(`Wrote checksums: ${path.relative(repoRoot, outputPath)}`);
} else {
  process.stdout.write(report);
}
