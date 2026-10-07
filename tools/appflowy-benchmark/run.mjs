import { appendFile, mkdir, open, rename, unlink, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

import { prepareVariants, buildFixture, directoryHash } from './prepare.mjs';
import { deadline } from './deadline.mjs';
import { renderReport, samplesCsv, summarize } from './report.mjs';

const directory = path.dirname(fileURLToPath(import.meta.url));
const families = ['arithmetic', 'text-list', 'dependency', 'business-long'];
const operations = ['read', 'edit-cell', 'edit-all', 'edit-formula'];

// Keep measurements serial and paired. Failure modes include building while
// timing, timing Playwright polling, reusing the old revision, silently dropping
// failures, changing work between A/B, and losing evidence on a later failure.
function optionsFromArguments() {
  const { values } = parseArgs({ options: {
    help: { type: 'boolean' }, smoke: { type: 'boolean' }, prepare: { type: 'boolean' }, screenshots: { type: 'boolean' },
    layer: { type: 'string', default: 'chain' }, output: { type: 'string' }, cache: { type: 'string' },
    rows: { type: 'string' }, families: { type: 'string' }, sessions: { type: 'string' },
    samples: { type: 'string' }, warmups: { type: 'string' }, seed: { type: 'string', default: '47183' },
    'browser-executable': { type: 'string' }, 'backend-url': { type: 'string' },
    'gotrue-url': { type: 'string' }, credentials: { type: 'string' }, 'auth-state': { type: 'string' },
    'full-app-rows': { type: 'string', default: '100' }, 'timeout-ms': { type: 'string', default: '120000' },
  } });
  if (values.help) {
    console.log(`Usage: node tools/appflowy-benchmark/run.mjs [options]

  --layer chain|full-app|all   Default: chain (no Cloud required)
  --output DIR               New output directory; existing evidence is never overwritten
  --cache DIR                Pinned checkouts/builds (default: OS cache directory)
  --prepare                  Prepare pinned dependencies only
  --smoke                    100 rows, 1 session, 2 samples, 1 warmup
  --rows 100,1000,10000       Computed-chain row counts
  --families NAME,...        arithmetic,text-list,dependency,business-long
  --sessions N               Independent browser processes per variant/case (default: 5)
  --samples N                Hot samples per operation/session (default: 20)
  --warmups N                Unreported warmups per hot operation (default: 5)
  --seed N                   Deterministic dataset seed (default: 47183)
  --browser-executable FILE  Use the same Chromium binary for both variants
  --backend-url URL          Full-app Cloud endpoint
  --gotrue-url URL           Full-app auth endpoint (default: backend /gotrue)
  --credentials FILE         Private JSON {email,password}; never copied into evidence
  --auth-state FILE          Alternatively, private Playwright storage state
  --full-app-rows 100         Full-app database sizes (independent from chain sizes)
  --screenshots              Capture dummy Grid/editor images after UI measurements
  --timeout-ms N             Failure deadline, never a reported measurement

First install Node, pnpm, Rust/wasm-pack and Chromium as documented. The runner
fetches immutable source pins, prepares the SDK and builds production assets.
Failed or unmatched measurements fail the run; they do not count as speedups.`);
    return null;
  }
  const integer = (name, fallback, minimum = 1) => {
    const value = Number(values[name] ?? fallback);
    if (!Number.isSafeInteger(value) || value < minimum) throw new Error(`Invalid --${name}`);
    return value;
  };
  const rowList = (name, fallback) => {
    const rows = (values[name] ?? fallback).split(',').map(Number);
    if (!rows.length || rows.some((value) => !Number.isSafeInteger(value) || value < 1 || value > 100_000) || new Set(rows).size !== rows.length)
      throw new Error(`Invalid --${name}`);
    return rows;
  };
  if (!['chain', 'full-app', 'all'].includes(values.layer)) throw new Error('Invalid --layer');
  const selectedFamilies = (values.families ?? families.join(',')).split(',');
  if (selectedFamilies.some((family) => !families.includes(family)) || new Set(selectedFamilies).size !== selectedFamilies.length)
    throw new Error('Invalid --families');
  const seed = integer('seed', 47183, 0);
  if (seed > 0xffffffff) throw new Error('Seed must fit an unsigned 32-bit integer');
  const options = {
    layer: values.layer,
    outputDir: path.resolve(values.output ?? path.join(os.tmpdir(), `formula-benchmark-${Date.now()}`)),
    cacheDir: path.resolve(values.cache ?? path.join(os.homedir(), '.cache', 'notion-formula-benchmark')),
    prepareOnly: Boolean(values.prepare),
    rowCounts: rowList('rows', values.smoke ? '100' : '100,1000,10000'),
    fullAppRows: rowList('full-app-rows', '100'), families: selectedFamilies,
    sessions: integer('sessions', values.smoke ? 1 : 5),
    samples: integer('samples', values.smoke ? 2 : 20),
    warmups: integer('warmups', values.smoke ? 1 : 5, 0), seed,
    timeoutMs: integer('timeout-ms', 120000), browserExecutablePath: values['browser-executable'],
    captureScreenshots: Boolean(values.screenshots),
    backendUrl: values['backend-url'], gotrueUrl: values['gotrue-url'],
    credentialsPath: values.credentials && path.resolve(values.credentials),
    authStatePath: values['auth-state'] && path.resolve(values['auth-state']),
  };
  if (!options.prepareOnly && options.layer !== 'chain' &&
      (!options.backendUrl || (!options.credentialsPath && !options.authStatePath)))
    throw new Error('Full-app runs require --backend-url and --credentials or --auth-state');
  for (const endpoint of [options.backendUrl, options.gotrueUrl].filter(Boolean)) {
    const url = new URL(endpoint);
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password)
      throw new Error('Backend endpoints must be HTTP(S) URLs without embedded credentials');
  }
  return options;
}

async function runChain(variants, chromium, options, record, metadata) {
  const hosts = new Map();
  try {
    // Both versions finish compilation before any timed browser processes start.
    for (const variant of variants) {
      console.log(`Build computed-chain fixture: ${variant.id}`);
      const host = await buildFixture(variant, { browserDir: path.join(directory, 'browser') });
      hosts.set(variant.id, host);
      metadata.builds.push({ variant: variant.id, layer: 'chain', ...host.metadata });
    }
    let caseIndex = 0;
    for (const family of options.families) {
      for (const rows of options.rowCounts) {
        for (let session = 0; session < options.sessions; session++) {
          const order = (caseIndex + session) % 2 ? [...variants].reverse() : variants;
          for (const variant of order) {
            const browser = await chromium.launch({ headless: true, executablePath: options.browserExecutablePath });
            metadata.browser = browser.version();
            const page = await browser.newPage({ timezoneId: 'Asia/Singapore', viewport: { width: 1600, height: 1050 } });
            const pageErrors = [];
            page.on('pageerror', (error) => pageErrors.push(error.message));
            try {
              page.setDefaultTimeout(options.timeoutMs);
              await page.goto(hosts.get(variant.id).url, { waitUntil: 'load', timeout: options.timeoutMs });
              await page.waitForFunction(() => Boolean(window.formulaBenchmark));
              const setup = await deadline(page.evaluate((config) => window.formulaBenchmark.setup(config),
                { family, rows, seed: options.seed }), options.timeoutMs, 'Dataset setup');
              if (setup.variant !== variant.id || setup.rowCount !== rows) throw new Error('Wrong benchmark implementation or dataset');
              if (session === 0) metadata.cases.push(setup);
              const measure = (operation, iteration) => deadline(page.evaluate(
                ({ operation, iteration }) => window.formulaBenchmark.measure(operation, iteration),
                { operation, iteration }), options.timeoutMs, `${variant.id}/${family}/${rows}/${operation}`);
              const common = { layer: 'chain', variant: variant.id, scenario: family, rows, session };
              await record({ ...common, ...await measure('initial', 0) });
              for (const operation of operations) {
                for (let index = 0; index < options.warmups; index++) await measure(operation, index);
                for (let index = 0; index < options.samples; index++) {
                  const sample = await measure(operation, options.warmups + index);
                  await record({ ...common, ...sample, iteration: index });
                }
              }
              await page.evaluate(() => window.formulaBenchmark.dispose());
              if (pageErrors.length) throw new Error(`Browser errors: ${pageErrors.join('; ')}`);
              console.log(`Measured ${variant.id} ${family} ${rows} rows session ${session + 1}/${options.sessions}`);
            } finally { await browser.close(); }
          }
        }
        caseIndex++;
      }
    }
  } finally {
    for (const host of hosts.values()) await host.close();
  }
}

async function main() {
  const options = optionsFromArguments();
  if (!options) return;
  await mkdir(options.outputDir, { recursive: true });
  const evidencePath = path.join(options.outputDir, 'results.json');
  const reservation = await open(evidencePath, 'wx');
  await reservation.close();
  const samples = [];
  const metadata = { builds: [], cases: [], limits: [
    'Only the pinned revisions, selected Chromium binary and recorded machine/workloads are compared.',
    'Computed-chain input data is resident before timing; first evaluation includes Engine/schema initialization, not page asset loading.',
    'Repeated read demand preserves production cache capacities; large working sets can evict legacy entries.',
    'Each initial chain sample uses a new browser process. Warmup samples are verified but not reported.',
    'Memory and instrumented CPU profiles are outside this first latency experiment.',
  ] };
  const report = {
    schemaVersion: 1, createdAt: new Date().toISOString(), status: 'running',
    benchmark: {
      revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: directory, encoding: 'utf8' }).trim(),
      sourceHash: await directoryHash(directory),
      modifiedSources: Boolean(execFileSync('git', ['status', '--porcelain', '--untracked-files=all', '--', '.'],
        { cwd: directory, encoding: 'utf8' }).trim()),
    },
    configuration: Object.fromEntries(Object.entries(options).filter(([key]) =>
      !['credentialsPath', 'authStatePath', 'outputDir', 'cacheDir', 'browserExecutablePath'].includes(key))),
    environment: { platform: os.platform(), release: os.release(), arch: os.arch(), node: process.version,
      cpu: os.cpus()[0]?.model, logicalCpus: os.cpus().length, totalMemoryBytes: os.totalmem(), startLoadAverage: os.loadavg() },
    variants: [], metadata, samples,
  };
  const record = async (sample) => {
    samples.push(sample);
    await appendFile(path.join(options.outputDir, 'samples.ndjson'), JSON.stringify(sample) + '\n');
  };
  const lockPath = path.join(options.cacheDir, 'benchmark.lock');
  let lock;
  try {
    await mkdir(options.cacheDir, { recursive: true });
    lock = await open(lockPath, 'wx').catch(() => { throw new Error(`Cache is already in use; inspect ${lockPath} before removing a stale lock`); });
    await lock.writeFile(JSON.stringify({ pid: process.pid, startedAt: report.createdAt }) + '\n');
    const variants = await prepareVariants({ cacheDir: options.cacheDir });
    report.variants = variants.map(({ root: _root, ...variant }) => variant);
    if (options.prepareOnly) { report.status = 'prepared'; return; }
    const require = createRequire(path.join(variants.find((variant) => variant.id === 'native').root, 'package.json'));
    const { chromium } = require('@playwright/test');
    if (options.layer !== 'full-app') await runChain(variants, chromium, options, record, metadata);
    if (options.layer !== 'chain') {
      const { runFullApp } = await import('./full-app.mjs');
      const fullApp = await runFullApp({ variants, chromium,
        options: { ...options, rowCounts: options.fullAppRows }, onSample: record });
      metadata.fullApp = fullApp.metadata;
      metadata.limits.push(...(fullApp.limits ?? []));
      metadata.artifacts = Object.fromEntries(Object.entries(fullApp.artifacts ?? {})
        .map(([key, artifact]) => [key, Array.isArray(artifact) ? artifact.map(file => path.basename(file)) : path.basename(artifact)]));
    }
    report.summary = summarize(samples);
    report.status = 'passed';
  } catch (error) {
    report.status = 'failed';
    report.failure = error instanceof Error ? error.message : String(error);
    process.exitCode = 1;
    console.error(report.failure);
  } finally {
    if (lock) { await lock.close(); await unlink(lockPath); }
    report.environment.endLoadAverage = os.loadavg();
    report.finishedAt = new Date().toISOString();
    const temporary = `${evidencePath}.tmp`;
    await writeFile(temporary, JSON.stringify(report, null, 2) + '\n');
    await rename(temporary, evidencePath);
    await writeFile(path.join(options.outputDir, 'samples.csv'), samplesCsv(samples));
    await writeFile(path.join(options.outputDir, 'report.md'), renderReport(report));
    console.log(`Evidence: ${options.outputDir} (${report.status}, ${samples.length} samples)`);
  }
}

await main();
