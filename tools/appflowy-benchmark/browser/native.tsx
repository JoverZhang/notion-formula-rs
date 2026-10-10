import { Component, ReactNode, useLayoutEffect } from 'react';
import { createRoot, Root } from 'react-dom/client';

import { DatabaseContext, DatabaseContextState } from '@/application/database-yjs/context';
import {
  NativeFormulaSnapshot,
  nativeFormulaOutcome,
  useNativeFormulaRuntime,
} from '@/application/database-yjs/formula/native-runtime';
import { projectNativeFormulaResult } from '@/application/database-yjs/formula/native-values';

import { BenchmarkDataset } from './dataset';
import { installBenchmark } from './harness';
import { BenchmarkAdapter, TimedResults } from './types';

interface PendingMeasurement {
  startedAt: number;
  minimumRevision: number;
  resolve: (timed: TimedResults) => void;
  reject: (error: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
}

class RuntimeBoundary extends Component<{ children: ReactNode; onError: (error: Error) => void }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  componentDidCatch(error: Error) { this.props.onError(error); }
  render() { return this.state.failed ? null : this.props.children; }
}

function RuntimeProbe({ dataset, accept }: { dataset: BenchmarkDataset; accept: (snapshot: NativeFormulaSnapshot) => void }) {
  const snapshot = useNativeFormulaRuntime({ rows: dataset.rows, formulaIds: dataset.formulaIds });

  useLayoutEffect(() => accept(snapshot), [snapshot, accept]);
  return null;
}

class NativeAdapter implements BenchmarkAdapter {
  readonly variant = 'native' as const;
  private readonly root: Root;
  private readonly container = document.createElement('div');
  private readonly context: DatabaseContextState;
  private snapshot?: NativeFormulaSnapshot;
  private pending?: PendingMeasurement;

  constructor(private readonly dataset: BenchmarkDataset) {
    // DOM/root and Yjs data construction happen during setup; the initial
    // measurement starts when rendering retains the real production runtime.
    document.body.append(this.container);
    this.root = createRoot(this.container);
    this.context = {
      databaseDoc: dataset.databaseDoc,
      databasePageId: 'benchmark-view', activeViewId: 'benchmark-view',
      readOnly: false, workspaceId: 'benchmark-workspace', rowMap: dataset.rows,
      seedsReady: true,
    };
  }

  private collect(snapshot: NativeFormulaSnapshot) {
    if (snapshot.phase !== 'ready') throw new Error(`Native runtime is ${snapshot.phase}`);
    if (snapshot.outcomes.size !== this.dataset.rowIds.length) throw new Error('Native runtime did not publish all rows');
    return this.dataset.rowIds.map((rowId) => this.dataset.formulaIds.map((fieldId) => {
      const outcome = nativeFormulaOutcome(snapshot, rowId, fieldId);

      if (outcome.status !== 'value' && outcome.status !== 'null')
        throw new Error(`${rowId}/${fieldId}: ${'error' in outcome ? outcome.error : outcome.status}`);
      // This is the same AppFlowy projection used by formula cells/consumers.
      return projectNativeFormulaResult(outcome, this.dataset.fields.get(fieldId));
    }));
  }

  private fail = (error: Error) => {
    const pending = this.pending;

    if (!pending) return;
    this.pending = undefined;
    clearTimeout(pending.timeout);
    pending.reject(error);
  };

  private accept = (snapshot: NativeFormulaSnapshot) => {
    this.snapshot = snapshot;
    const pending = this.pending;

    if (!pending || snapshot.revision < pending.minimumRevision || snapshot.phase === 'pending') return;
    if (snapshot.phase === 'error') {
      this.fail(new Error(snapshot.error ?? 'Native formula Worker failed'));
      return;
    }

    try {
      const results = this.collect(snapshot);
      const stoppedAt = performance.now();

      this.pending = undefined;
      clearTimeout(pending.timeout);
      pending.resolve({ elapsedMs: stoppedAt - pending.startedAt, results, revision: snapshot.revision });
    } catch (error) {
      this.fail(error instanceof Error ? error : new Error(String(error)));
    }
  };

  private measure(trigger: () => void, minimumRevision: number): Promise<TimedResults> {
    if (this.pending) return Promise.reject(new Error('Native measurement is already pending'));
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => this.fail(new Error('Native runtime did not finish within 120 seconds')), 120_000);
      const pending = { startedAt: 0, minimumRevision, resolve, reject, timeout };

      this.pending = pending;
      pending.startedAt = performance.now();
      try {
        trigger();
      } catch (error) {
        this.fail(error instanceof Error ? error : new Error(String(error)));
      }
    });
  }

  initial() {
    return this.measure(() => this.root.render(
      <RuntimeBoundary onError={this.fail}>
        <DatabaseContext.Provider value={this.context}>
          <RuntimeProbe dataset={this.dataset} accept={this.accept} />
        </DatabaseContext.Provider>
      </RuntimeBoundary>
    ), 1);
  }

  read(): TimedResults {
    const startedAt = performance.now();
    const snapshot = this.snapshot;

    if (!snapshot) throw new Error('Native runtime has not initialized');
    const results = this.collect(snapshot);
    const stoppedAt = performance.now();

    return { elapsedMs: stoppedAt - startedAt, results, revision: snapshot.revision };
  }

  edit(mutate: () => void) {
    if (!this.snapshot || this.snapshot.phase !== 'ready') return Promise.reject(new Error('Native runtime is not ready for editing'));
    // Yjs updates synchronously invalidate the runtime. Accept only a newer
    // ready revision, while the production scheduler chooses its own batching.
    return this.measure(mutate, this.snapshot.revision + 1);
  }

  dispose() {
    this.fail(new Error('Native benchmark disposed'));
    this.root.unmount();
    this.container.remove();
    this.snapshot = undefined;
  }
}

installBenchmark((dataset) => new NativeAdapter(dataset));
