import { evaluateFormulaCell } from '@/application/database-yjs/fields/formula/evaluate';
import { readFormulaSchema } from '@/application/database-yjs/fields/formula/schema';

import { BenchmarkDataset } from './dataset';
import { installBenchmark } from './harness';
import { BenchmarkAdapter, TimedResults } from './types';

class LegacyAdapter implements BenchmarkAdapter {
  readonly variant = 'legacy' as const;
  private revision = 0;

  constructor(private readonly dataset: BenchmarkDataset) {}

  private collect() {
    // The production schema reader attaches the source identity required by
    // result-cache.ts. Never construct an untracked schema or reset its caches.
    const schema = readFormulaSchema(this.dataset.fields);

    return this.dataset.rowValues.map((row, index) => this.dataset.formulaIds.map((fieldId) =>
      evaluateFormulaCell({
        schema, field: this.dataset.fields.get(fieldId), fieldId,
        row, rowId: this.dataset.rowIds[index],
      })
    ));
  }

  private measure(mutate?: () => void): TimedResults {
    const startedAt = performance.now();

    mutate?.();
    const results = this.collect();
    const stoppedAt = performance.now();

    if (mutate || this.revision === 0) this.revision += 1;
    return { elapsedMs: stoppedAt - startedAt, results, revision: this.revision };
  }

  async initial() { return this.measure(); }
  read() { return this.measure(); }
  async edit(mutate: () => void) { return this.measure(mutate); }
  dispose() {}
}

installBenchmark((dataset) => new LegacyAdapter(dataset));
