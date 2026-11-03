import { readdir, readFile, access } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const docsRoot = path.join(repoRoot, 'apps/docs');
const docsContent = path.join(docsRoot, 'content/docs');
const required = [
  'getting-started/installation.mdx',
  'guides/usage.mdx',
  'configuration/options.mdx',
  'api/formatter.mdx',
  'api/json.mdx',
  'api/sarif.mdx',
  'api/types.mdx',
  'platforms/compatibility.mdx',
  'performance/benchmarks.mdx',
  'troubleshooting/common-issues.mdx',
  'project/contributing.mdx',
  'project/security.mdx',
  'project/license.mdx',
];
for (const file of required) await access(path.join(docsContent, file));

async function walk(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const fullPath = path.join(directory, entry.name);
    files.push(...(entry.isDirectory() ? await walk(fullPath) : [fullPath]));
  }
  return files;
}

const mdxFiles = (await walk(docsContent)).filter((file) => file.endsWith('.mdx'));
for (const file of mdxFiles) {
  const text = await readFile(file, 'utf8');
  if (!text.startsWith('---\n') || !/^title:\s*.+$/m.test(text.split('---', 3)[1] ?? '')) {
    throw new Error(`Missing MDX title frontmatter: ${path.relative(docsRoot, file)}`);
  }
}

const manifest = JSON.parse(await readFile(path.join(docsRoot, 'package.json'), 'utf8'));
for (const script of ['dev', 'build', 'build:pages', 'lint', 'typecheck', 'verify']) {
  if (!manifest.scripts?.[script]) throw new Error(`Missing docs script: ${script}`);
}
for (const file of [
  'source.config.ts',
  'next.config.mjs',
  'app/api/search/route.ts',
  'app/llms.txt/route.ts',
  'app/llms-full.txt/route.ts',
]) {
  await access(path.join(docsRoot, file));
}
console.log(`Documentation source verified (${mdxFiles.length} MDX pages).`);
