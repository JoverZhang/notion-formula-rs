import type { FormulaCellResult } from '@/application/database-yjs/fields/formula/formula.type';

import type { BenchmarkDataset } from './dataset';

export const FAMILIES = ['arithmetic', 'text-list', 'dependency', 'business-long'] as const;
export const OPERATIONS = ['initial', 'read', 'edit-cell', 'edit-all', 'edit-formula'] as const;
export type Family = (typeof FAMILIES)[number];
export type Operation = (typeof OPERATIONS)[number];
export type Variant = 'legacy' | 'native';

export interface SetupOptions {
  family: Family;
  rows: number;
  seed: number;
}

export interface BenchmarkMetadata {
  mode: 'production-computed-chain';
  variant: Variant;
  family: Family;
  rowCount: number;
  formulaCount: number;
  seed: number;
  inputFieldIds: string[];
  formulas: Array<{ id: string; expression: string; characters: number }>;
  timer: 'browser.performance.now';
  endpoint: 'all requested production-projected formula results';
}

export interface TimedResults {
  elapsedMs: number;
  results: FormulaCellResult[][];
  revision: number;
}

export interface BenchmarkSample {
  operation: Operation;
  iteration: number;
  elapsedMs: number;
  verified: true;
  checksum: string;
  rowCount: number;
  formulaCount: number;
  revision: number;
}

export interface BenchmarkAdapter {
  variant: Variant;
  initial(): Promise<TimedResults>;
  read(): TimedResults;
  edit(mutate: () => void): Promise<TimedResults>;
  dispose(): void;
}

export type AdapterFactory = (dataset: BenchmarkDataset) => BenchmarkAdapter;

export interface FormulaBenchmarkControl {
  setup(options: SetupOptions): Promise<BenchmarkMetadata>;
  measure(operation: Operation, iteration: number): Promise<BenchmarkSample>;
  dispose(): Promise<void>;
}

declare global {
  interface Window {
    formulaBenchmark: FormulaBenchmarkControl;
  }
}
