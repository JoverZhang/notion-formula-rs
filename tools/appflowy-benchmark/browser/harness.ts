/*
 * Failure modes to guard before collecting timings:
 * - A previous native revision can be ready while an edit is still pending.
 * - A ready runtime can still contain pending, NotReady, or failed targets.
 * - Direct SDK/Rust calls omit production Yjs decoding and Worker conversions.
 * - Untracked legacy schema arrays disable its bounded production result cache.
 * - Repeated identical edits accidentally measure cached reads.
 * - Validation, checksums, polling, or Worker logging can contaminate timers.
 * Each adapter must return every requested production-projected result and stop
 * its browser clock before this file validates outputs against the dataset oracle.
 */
import type { FormulaCellResult } from '@/application/database-yjs/fields/formula/formula.type';
import type { FormulaValue } from '@/application/database-yjs/fields/formula/values';

import { BenchmarkDataset } from './dataset';
import {
  AdapterFactory,
  BenchmarkAdapter,
  BenchmarkMetadata,
  FAMILIES,
  OPERATIONS,
  Operation,
  SetupOptions,
  TimedResults,
} from './types';

function canonicalValue(value: FormulaValue): unknown {
  switch (value.type) {
    case 'number':
      if (!Number.isFinite(value.value)) throw new Error('Non-finite benchmark output');
      return ['number', Object.is(value.value, -0) ? '-0' : value.value];
    case 'text': return ['text', value.value];
    case 'boolean': return ['boolean', value.value];
    case 'list': return ['list', value.items.map(canonicalValue)];
    default: throw new Error(`Unexpected ${value.type} benchmark output`);
  }
}

/** Every typed value and display string is checked after the adapter stops time. */
function verifyAndChecksum(dataset: BenchmarkDataset, timed: TimedResults): string {
  if (!Number.isFinite(timed.elapsedMs) || timed.elapsedMs < 0) throw new Error('Invalid browser duration');
  if (timed.results.length !== dataset.rowIds.length) throw new Error('Missing requested output rows');
  let checksum = 0x811c9dc5;
  const append = (text: string) => {
    for (let index = 0; index < text.length; index += 1) {
      checksum = Math.imul(checksum ^ text.charCodeAt(index), 0x01000193) >>> 0;
    }
  };

  timed.results.forEach((results, rowIndex) => {
    if (results.length !== dataset.formulaIds.length) throw new Error('Missing requested formula outputs');
    const expected = dataset.expected(rowIndex);

    results.forEach((result: FormulaCellResult, formulaIndex) => {
      const identity = `${dataset.rowIds[rowIndex]}/${dataset.formulaIds[formulaIndex]}`;
      const oracle = expected[formulaIndex];

      if (result.error !== undefined || result.missingPropertyRef !== undefined)
        throw new Error(`${identity}: ${result.error ?? 'missing property'}`);
      const actualValue = JSON.stringify(canonicalValue(result.value));
      const expectedValue = JSON.stringify(canonicalValue(oracle.value));

      if (actualValue !== expectedValue)
        throw new Error(`${identity}: typed output mismatch: expected ${expectedValue}, received ${actualValue}`);
      if (JSON.stringify(result.resultType) !== JSON.stringify(oracle.resultType))
        throw new Error(`${identity}: result type mismatch`);
      if (result.text !== oracle.text)
        throw new Error(`${identity}: display mismatch: expected ${JSON.stringify(oracle.text)}, received ${JSON.stringify(result.text)}`);
      if (result.value.type === 'number' && !Object.is(result.rawNumeric, result.value.value))
        throw new Error(`${identity}: numeric consumer projection mismatch`);
      if (result.value.type === 'boolean' && result.rawBoolean !== result.value.value)
        throw new Error(`${identity}: boolean consumer projection mismatch`);
      append(JSON.stringify([identity, result.resultType, canonicalValue(result.value), result.text]));
      append('\n');
    });
  });

  return `fnv1a32:${checksum.toString(16).padStart(8, '0')}`;
}

function validateSetup(options: SetupOptions) {
  if (!options || !FAMILIES.includes(options.family)) throw new Error(`Unsupported family ${options?.family}`);
  if (!Number.isSafeInteger(options.rows) || options.rows < 1 || options.rows > 100_000)
    throw new Error('rows must be an integer from 1 to 100000');
  if (!Number.isSafeInteger(options.seed) || options.seed < 0 || options.seed > 0xffff_ffff)
    throw new Error('seed must be an unsigned 32-bit integer');
}

export function installBenchmark(createAdapter: AdapterFactory) {
  let dataset: BenchmarkDataset | undefined;
  let adapter: BenchmarkAdapter | undefined;
  let initialized = false;
  let measuring = false;
  let failed: Error | undefined;

  const dispose = async () => {
    adapter?.dispose();
    dataset?.destroy();
    adapter = undefined;
    dataset = undefined;
    initialized = false;
    failed = undefined;
  };

  window.formulaBenchmark = {
    async setup(options): Promise<BenchmarkMetadata> {
      if (measuring) throw new Error('A benchmark measurement is already running');
      validateSetup(options);
      await dispose();
      dataset = new BenchmarkDataset({ ...options });
      adapter = createAdapter(dataset);
      return {
        mode: 'production-computed-chain', variant: adapter.variant,
        family: options.family, seed: options.seed,
        rowCount: dataset.rowIds.length, formulaCount: dataset.formulaIds.length,
        inputFieldIds: dataset.inputFieldIds,
        formulas: dataset.specs.filter((field) => field.options?.expression !== undefined).map((field) => ({
          id: field.id, expression: String(field.options!.expression), characters: String(field.options!.expression).length,
        })),
        timer: 'browser.performance.now', endpoint: 'all requested production-projected formula results',
      };
    },

    async measure(operation: Operation, iteration) {
      if (!dataset || !adapter) throw new Error('Call setup before measure');
      if (failed) throw new Error(`Benchmark previously failed: ${failed.message}`);
      if (measuring) throw new Error('A benchmark measurement is already running');
      if (!OPERATIONS.includes(operation)) throw new Error(`Unsupported operation ${operation}`);
      if (!Number.isSafeInteger(iteration) || iteration < 0) throw new Error('iteration must be a nonnegative integer');
      if (operation === 'initial' && initialized) throw new Error('initial requires a fresh setup');
      if (operation !== 'initial' && !initialized) throw new Error('Measure initial before reads or edits');
      measuring = true;
      try {
        const timed = operation === 'initial'
          ? await adapter.initial()
          : operation === 'read'
          ? adapter.read()
          : await adapter.edit(dataset.prepareMutation(operation, iteration));
        // No oracle access or checksum generation belongs in an adapter timer.
        const checksum = verifyAndChecksum(dataset, timed);

        initialized = true;
        return {
          operation, iteration, elapsedMs: timed.elapsedMs, verified: true as const, checksum,
          rowCount: dataset.rowIds.length, formulaCount: dataset.formulaIds.length, revision: timed.revision,
        };
      } catch (error) {
        failed = error instanceof Error ? error : new Error(String(error));
        throw failed;
      } finally {
        measuring = false;
      }
    },

    async dispose() {
      if (measuring) throw new Error('Cannot dispose during a measurement');
      await dispose();
    },
  };
}
