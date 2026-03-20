#!/usr/bin/env node

import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';

const DEFAULT_STYLELINT_VERSION = '16.15.0';

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const corpora = await discoverCorpora(args.corpusRoot);
  if (corpora.length === 0) {
    throw new Error(`no corpora found under '${args.corpusRoot}'`);
  }

  const configPath = await writeTempConfig();
  try {
    const results = [];
    for (const corpus of corpora) {
      results.push(await runCorpus(corpus, args, configPath));
    }

    const payload = {
      schemaVersion: 1,
      tool: 'stylelint',
      executionModel: 'single-process-per-corpus-iteration',
      stylelintVersion: process.env.STYLELINT_VERSION || DEFAULT_STYLELINT_VERSION,
      protocol: {
        coldIterations: args.coldIterations,
        warmIterations: args.warmIterations,
      },
      corpora: results,
    };

    await fs.mkdir(path.dirname(args.output), { recursive: true });
    await fs.writeFile(args.output, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
    process.stdout.write(`stylelint benchmark summary written to ${args.output}\n`);
  } finally {
    await fs.rm(configPath, { force: true });
  }
}

async function runCorpus(corpus, args, configPath) {
  const files = await discoverLintFiles(corpus.root);
  if (files.length === 0) {
    throw new Error(`corpus '${corpus.id}' has no lintable files`);
  }

  const sources = [];
  for (const filePath of files) {
    const source = await fs.readFile(filePath, 'utf8');
    sources.push({ filePath, source });
  }

  const totalBytes = sources.reduce((sum, entry) => sum + Buffer.byteLength(entry.source), 0);
  const digest = digestSources(sources);

  const coldRuns = [];
  for (let i = 0; i < args.coldIterations; i += 1) {
    coldRuns.push(runIteration(sources, configPath));
  }

  const warmRuns = [];
  for (let i = 0; i < args.warmIterations; i += 1) {
    warmRuns.push(runIteration(sources, configPath));
  }

  const sortedWarm = [...warmRuns].sort((left, right) => left.totalMs - right.totalMs);
  const warmMedian = sortedWarm[Math.floor(sortedWarm.length / 2)] || zeroSummary();

  return {
    corpusId: corpus.id,
    files: files.length,
    totalBytes,
    corpusDigest: digest,
    coldRuns,
    warmRuns,
    warmMedian,
  };
}

function runIteration(sources, configPath) {
  const filePaths = sources.map((entry) => entry.filePath);
  const startedAt = process.hrtime.bigint();
  let peakRss = process.memoryUsage().rss;
  lintBatch(filePaths, configPath);
  peakRss = Math.max(peakRss, process.memoryUsage().rss);

  const endedAt = process.hrtime.bigint();
  const totalMs = nanosToMillis(endedAt - startedAt);
  const totalSeconds = totalMs / 1000;
  const totalBytes = sources.reduce((sum, entry) => sum + Buffer.byteLength(entry.source), 0);
  const averageFileMs = sources.length > 0 ? totalMs / sources.length : 0;

  return {
    totalMs,
    filesPerSecond: totalSeconds > 0 ? sources.length / totalSeconds : 0,
    mbPerSecond: totalSeconds > 0 ? (totalBytes / (1024 * 1024)) / totalSeconds : 0,
    p50FileMs: averageFileMs,
    p95FileMs: averageFileMs,
    peakRssBytes: peakRss,
  };
}

function lintBatch(filePaths, configPath) {
  const result = spawnSync(
    'stylelint',
    [
      ...filePaths,
      '--custom-syntax',
      'postcss-html',
      '--config',
      configPath,
      '--formatter',
      'json',
      '--allow-empty-input',
    ],
    { encoding: 'utf8' }
  );

  if (result.error) {
    throw new Error(`failed to execute stylelint batch run: ${result.error.message}`);
  }

  if (result.status !== 0 && result.status !== 2) {
    throw new Error(`stylelint runtime failure: ${result.stderr || result.stdout}`);
  }
}

async function writeTempConfig() {
  const config = {
    rules: {
      'block-no-empty': true,
      'declaration-block-no-duplicate-properties': true,
      'declaration-property-value-no-unknown': true,
      'no-duplicate-selectors': true,
      'property-no-unknown': true,
      'property-no-vendor-prefix': true,
      'selector-no-qualifying-type': true,
      'value-no-vendor-prefix': true,
    },
  };

  const configPath = path.join(os.tmpdir(), `csslint-stylelint-perf-${Date.now()}.json`);
  await fs.writeFile(configPath, `${JSON.stringify(config)}\n`, 'utf8');
  return configPath;
}

async function discoverCorpora(root) {
  const entries = await fs.readdir(root, { withFileTypes: true });
  entries.sort((left, right) => left.name.localeCompare(right.name));
  return entries.filter((entry) => entry.isDirectory()).map((entry) => ({
    id: entry.name,
    root: path.join(root, entry.name),
  }));
}

async function discoverLintFiles(root) {
  const files = [];
  const pending = [root];

  while (pending.length > 0) {
    const next = pending.pop();
    const entries = await fs.readdir(next, { withFileTypes: true });
    entries.sort((left, right) => left.name.localeCompare(right.name));
    for (const entry of entries) {
      const fullPath = path.join(next, entry.name);
      if (entry.isDirectory()) {
        pending.push(fullPath);
        continue;
      }
      if (entry.isFile() && /\.(css|vue|svelte)$/u.test(entry.name)) {
        files.push(fullPath);
      }
    }
  }

  files.sort((left, right) => left.localeCompare(right));
  return files;
}

function parseArgs(argv) {
  const parsed = {
    corpusRoot: 'tests/perf/corpora',
    output: 'artifacts/perf/stylelint-summary.json',
    warmIterations: 5,
    coldIterations: 1,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--corpus-root') {
      parsed.corpusRoot = argv[++i];
    } else if (arg === '--output') {
      parsed.output = argv[++i];
    } else if (arg === '--warm-iterations') {
      parsed.warmIterations = Number.parseInt(argv[++i], 10);
    } else if (arg === '--cold-iterations') {
      parsed.coldIterations = Number.parseInt(argv[++i], 10);
    } else if (arg === '-h' || arg === '--help') {
      throw new Error('usage: node scripts/stylelint_perf_benchmark.mjs [--corpus-root tests/perf/corpora] [--output artifacts/perf/stylelint-summary.json] [--warm-iterations 5] [--cold-iterations 1]');
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }

  return parsed;
}

function digestSources(sources) {
  let hash = 0xcbf29ce484222325n;
  for (const entry of sources) {
    const bytes = Buffer.from(`${entry.filePath}\n${entry.source}`, 'utf8');
    for (const byte of bytes) {
      hash ^= BigInt(byte);
      hash = (hash * 0x100000001b3n) & 0xffffffffffffffffn;
    }
  }
  return hash.toString(16).padStart(16, '0');
}

function nanosToMillis(value) {
  return Number(value) / 1_000_000;
}

function zeroSummary() {
  return {
    totalMs: 0,
    filesPerSecond: 0,
    mbPerSecond: 0,
    p50FileMs: 0,
    p95FileMs: 0,
    peakRssBytes: 0,
  };
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exit(2);
});
