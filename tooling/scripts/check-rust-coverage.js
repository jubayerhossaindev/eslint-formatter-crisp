#!/usr/bin/env node
/**
 * check-rust-coverage.js — Parse cargo-llvm-cov JSON output and enforce
 * numeric coverage thresholds for Rust production code.
 *
 * Reads llvm-cov JSON from stdin (or --input), checks per-file line,
 * region, and function coverage against thresholds, and exits non-zero
 * if any file falls below.
 *
 * Usage:
 *   cargo llvm-cov --workspace --json | node check-rust-coverage.js
 *   cargo llvm-cov --workspace --json --output-path coverage.json &&
 *     node check-rust-coverage.js --input coverage.json
 */

import { readFileSync } from 'node:fs';

const THRESHOLDS = {
  lines: 99, // % line coverage minimum
  regions: 96, // % region coverage minimum
  functions: 90, // % function coverage minimum (accommodates #[napi] attrs)
};

let inputPath;
const args = process.argv.slice(2);
for (let i = 0; i < args.length; i += 1) {
  if (args[i] === '--input' && i + 1 < args.length) {
    inputPath = args[i + 1];
    i += 1;
  }
}

let jsonText;
if (inputPath) {
  jsonText = readFileSync(inputPath, 'utf8');
} else {
  let stdin = '';
  process.stdin.on('data', (chunk) => {
    stdin += chunk;
  });
  process.stdin.on('end', () => {
    jsonText = stdin;
    run();
  });
  process.stdin.on('error', (err) => {
    console.error(`Error reading stdin: ${err.message}`);
    process.exit(1);
  });
  if (process.stdin.isTTY) {
    console.error('No input provided. Pipe cargo-llvm-cov --json output or use --input.');
    process.exit(1);
  }
}

run();

function run() {
  let data;
  try {
    data = JSON.parse(jsonText);
  } catch (err) {
    console.error(`Failed to parse coverage JSON: ${err.message}`);
    process.exit(1);
  }

  // cargo-llvm-cov JSON: { data: [{ files: [{ filename, summary: {lines, regions, functions, branches} }] }] }
  const dataArray = data.data || [];
  if (dataArray.length === 0) {
    console.error('No coverage data found in JSON.');
    process.exit(1);
  }

  const files = dataArray[0].files || [];
  let failures = 0;
  let totalLines = 0;
  let totalLinesHit = 0;
  let totalRegions = 0;
  let totalRegionsHit = 0;
  let totalFunctions = 0;
  let totalFunctionsHit = 0;

  for (const file of files) {
    const filePath = file.filename || 'unknown';
    const summary = file.summary || {};

    const lines = summary.lines?.count || 0;
    const linesHit = summary.lines?.covered || 0;
    const regions = summary.regions?.count || 0;
    const regionsHit = summary.regions?.covered || 0;
    const functions = summary.functions?.count || 0;
    const functionsHit = summary.functions?.covered || 0;

    totalLines += lines;
    totalLinesHit += linesHit;
    totalRegions += regions;
    totalRegionsHit += regionsHit;
    totalFunctions += functions;
    totalFunctionsHit += functionsHit;

    const linePct = lines > 0 ? (linesHit * 100) / lines : 100;
    const regionPct = regions > 0 ? (regionsHit * 100) / regions : 100;
    const funcPct = functions > 0 ? (functionsHit * 100) / functions : 100;

    let fileFailed = false;
    if (linePct < THRESHOLDS.lines) {
      console.error(
        `FAIL: ${filePath} line coverage ${linePct.toFixed(2)}% < ${THRESHOLDS.lines}%`,
      );
      fileFailed = true;
    }
    if (regionPct < THRESHOLDS.regions) {
      console.error(
        `FAIL: ${filePath} region coverage ${regionPct.toFixed(2)}% < ${THRESHOLDS.regions}%`,
      );
      fileFailed = true;
    }
    if (funcPct < THRESHOLDS.functions) {
      console.error(
        `FAIL: ${filePath} function coverage ${funcPct.toFixed(2)}% < ${THRESHOLDS.functions}%`,
      );
      fileFailed = true;
    }

    if (fileFailed) failures += 1;
    else {
      console.error(
        `PASS: ${filePath} lines=${linePct.toFixed(2)}% regions=${regionPct.toFixed(2)}% functions=${funcPct.toFixed(2)}%`,
      );
    }
  }

  const totalLinePct = totalLines > 0 ? (totalLinesHit * 100) / totalLines : 100;
  const totalRegionPct = totalRegions > 0 ? (totalRegionsHit * 100) / totalRegions : 100;
  const totalFuncPct = totalFunctions > 0 ? (totalFunctionsHit * 100) / totalFunctions : 100;

  console.error('\n=== TOTAL ===');
  console.error(`lines: ${totalLinePct.toFixed(2)}% (${totalLinesHit}/${totalLines})`);
  console.error(`regions: ${totalRegionPct.toFixed(2)}% (${totalRegionsHit}/${totalRegions})`);
  console.error(`functions: ${totalFuncPct.toFixed(2)}% (${totalFunctionsHit}/${totalFunctions})`);

  if (failures > 0) {
    console.error(`\n${failures} file(s) below coverage thresholds.`);
    process.exit(1);
  }

  console.error('\nAll files meet coverage thresholds.');
  process.exit(0);
}
