// Reconstruct this dated run only. Preserve the failed all-layer status and
// distinguish recorded samples from metadata corroborated after its crash.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { samplesCsv, summarize } from '../../../../tools/appflowy-benchmark/report.mjs';

const directory = path.dirname(fileURLToPath(import.meta.url));
const destination = process.argv[2];
if (!destination) throw new Error('Usage: node recover-chain.mjs NEW_OUTPUT_DIRECTORY');
const sha256 = contents => createHash('sha256').update(contents).digest('hex');
const context = JSON.parse(await readFile(path.join(directory, 'chain-recovery-context.json'), 'utf8'));
const raw = await readFile(path.join(directory, 'chain-samples.ndjson'));
assert.equal(sha256(raw), context.attempt.samplesSha256);
assert.equal(raw.length, context.attempt.samplesBytes);
assert.equal(raw.at(-1), 10, 'The original log must end in a complete newline');
const samples = raw.toString('utf8').trimEnd().split('\n').map(line => JSON.parse(line));
const configuration = context.invocation;
const key = sample => JSON.stringify(['scenario', 'rows', 'variant', 'session', 'operation', 'iteration'].map(field => sample[field]));
const expected = new Set();
for (const scenario of configuration.families) {
  for (const rows of configuration.rowCounts) {
    for (const variant of ['legacy', 'native']) {
      for (let session = 0; session < configuration.sessions; session++) {
        for (const operation of ['initial', 'read', 'edit-cell', 'edit-all', 'edit-formula']) {
          for (let iteration = 0; iteration < (operation === 'initial' ? 1 : configuration.samples); iteration++) {
            expected.add(key({ scenario, rows, variant, session, operation, iteration }));
          }
        }
      }
    }
  }
}
assert.equal(expected.size, 2520);
assert.equal(samples.length, expected.size);
for (const sample of samples) {
  assert.equal(sample.layer, 'chain');
  assert.equal(sample.verified, true);
  assert.equal(sample.rowCount, sample.rows);
  assert.equal(sample.formulaCount, sample.scenario === 'dependency' ? 7 : 3);
  assert.ok(expected.delete(key(sample)), 'Every expected sample must occur exactly once');
}
assert.equal(expected.size, 0);
const summary = summarize(samples);
assert.equal(summary.length, 60);
for (const variant of ['legacy', 'native']) {
  const observed = context.corroboration.metadata.builds.find(build => build.variant === variant);
  const cached = context.corroboration.cacheMarkers[variant].chainBuild;
  assert.equal(observed.artifactHash, cached.artifactHash);
  for (const [field, value] of Object.entries(cached.inputs)) assert.equal(observed[field], value);
}
assert.equal(context.attempt.originalRevision, context.corroboration.benchmark.revision);
assert.equal(context.attempt.originalSourceHash, context.corroboration.benchmark.sourceHash);
const report = {
  schemaVersion: 1,
  createdAt: context.attempt.lock.startedAt,
  status: 'recovered-chain-complete',
  originalAttemptStatus: 'failed',
  originalFailurePhase: context.attempt.phase,
  completedLayers: ['chain'],
  recovery: {
    recoveredAt: new Date().toISOString(),
    scriptSha256: sha256(await readFile(fileURLToPath(import.meta.url))),
    reporterSha256: sha256(await readFile(new URL('../../../../tools/appflowy-benchmark/report.mjs', import.meta.url))),
    contextSha256: sha256(await readFile(path.join(directory, 'chain-recovery-context.json'))),
    samplesSha256: sha256(raw),
    recorded: 'Unmodified NDJSON samples, cache lock start time, last append mtime and observed failure.',
    reconstructed: context.corroboration.source,
    unavailable: context.unavailable,
  },
  benchmark: context.corroboration.benchmark,
  configuration,
  environment: { ...context.corroboration.environment, startLoadAverage: null, endLoadAverage: null },
  variants: context.corroboration.variants,
  metadata: { ...context.corroboration.metadata, limits: [...context.corroboration.metadata.limits, ...context.limits] },
  samples,
  summary,
};
await mkdir(destination); // A rerun must not replace existing experimental evidence.
await writeFile(path.join(destination, 'chain-recovered.json'), JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
await writeFile(path.join(destination, 'chain-samples.csv'), samplesCsv(samples), { flag: 'wx' });
console.log(`Recovered ${samples.length} unchanged chain samples in ${summary.length} complete paired groups. Original all-layer attempt remains failed.`);
