import assert from 'node:assert/strict';
import test from 'node:test';

import { summarize, samplesCsv } from './report.mjs';

// Reporting failures: mismatched work is compared, missing/failed samples look
// fast, warmups enter the report, zero-clock samples yield infinite speedups,
// session pairing is lost, or spreadsheet/CSV formatting corrupts raw evidence.
function sample(variant, elapsedMs, session = 0, iteration = 0, extra = {}) {
  return { layer: 'chain', variant, scenario: 'arithmetic', operation: 'read',
    rows: 100, session, iteration, elapsedMs, verified: true, checksum: 'same', ...extra };
}

test('reports matched measurements and keeps independent scenarios separate', () => {
  const samples = [];
  for (let session = 0; session < 5; session++) {
    for (let iteration = 0; iteration < 20; iteration++) {
      const work = { rowCount: 100, formulaCount: 3 };
      samples.push(sample('legacy', 10, session, iteration, work), sample('native', 5, session, iteration, work));
    }
  }
  samples.push(sample('legacy', 2, 0, 0, { operation: 'initial' }), sample('native', 4, 0, 0, { operation: 'initial' }));
  const report = summarize(samples);
  const read = report.find((row) => row.operation === 'read');
  assert.equal(read.legacy.count, 100);
  assert.equal(read.native.p50Ms, 5);
  assert.equal(read.native.p95Ms, 5);
  assert.equal(read.native.projectedCellsPerSecond, 60000);
  assert.equal(read.nativeOverLegacy, 0.5);
  assert.deepEqual(read.ratio95, [0.5, 0.5]);
  assert.equal(read.conclusion, 'native-faster');
  assert.equal(report.find((row) => row.operation === 'initial').conclusion, 'insufficient-sessions');
});

test('rejects incorrect, unpaired, duplicate, non-finite, and unequal work', () => {
  const pair = [sample('legacy', 10), sample('native', 5)];
  for (const change of [
    { verified: false }, { elapsedMs: -1 }, { elapsedMs: NaN },
    { checksum: 'different' }, { rowCount: 99 }, { formulaCount: 2 },
  ]) {
    const baseline = { ...pair[0], rowCount: 100, formulaCount: 1 };
    const candidate = { ...pair[1], rowCount: 100, formulaCount: 1, ...change };
    assert.throws(() => summarize([baseline, candidate]));
  }
  assert.throws(() => summarize([pair[0]]));
  assert.throws(() => summarize([...pair, pair[0]]));
});

test('zero-duration samples do not claim an unbounded speedup', () => {
  const [row] = summarize([sample('legacy', 1), sample('native', 0)]);
  assert.equal(row.nativeOverLegacy, null);
  assert.equal(row.conclusion, 'below-clock-resolution');
});

test('CSV retains metadata and escapes separators without changing numerical values', () => {
  const csv = samplesCsv([sample('legacy', 1.25, 0, 0, {
    scenario: 'a,"b', inputChecksum: 'abc', visibleRows: 8, domReadyMs: 0.5, stableFrameMs: 0.75,
    diagnosticCategory: 'syntax', diagnosticText: 'expected expression',
  })]);
  assert.match(csv, /elapsedMs/);
  assert.match(csv, /"a,""b"/);
  assert.match(csv, /1\.25/);
  assert.match(csv, /inputChecksum,visibleRows,visibleFormulaCells,domReadyMs,stableFrameMs/);
  assert.match(csv, /abc,8,,0\.5,0\.75/);
  assert.match(csv, /diagnosticCategory,diagnosticText/);
  assert.match(csv, /syntax,expected expression/);
});
