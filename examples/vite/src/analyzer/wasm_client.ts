import init, * as wasm from "../pkg/analyzer_wasm.js";
import type { AnalyzerDiagnostic } from "../app/types";
import type { Token } from "../editor_decorations";

export type AnalyzeResult = {
  diagnostics?: AnalyzerDiagnostic[];
  tokens?: Token[];
  formatted?: string;
};

export async function initWasm(): Promise<void> {
  await init();
}

export function analyzeSource(source: string, contextJson?: string): AnalyzeResult {
  return wasm.analyze(source, contextJson);
}
