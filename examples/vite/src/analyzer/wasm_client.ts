import type { AnalyzerDiagnostic } from "../app/types";
import type { Token } from "../editor_decorations";
import init, * as wasm from "../pkg/analyzer_wasm.js";

export type AnalyzeResult = {
  diagnostics: AnalyzerDiagnostic[];
  tokens: Token[];
  formatted: string;
};

export type Span = { start: number; end: number };

export type TextEdit = {
  range: Span;
  new_text: string;
};

export type CompletionItem = {
  label: string;
  kind: string;
  insert_text: string;
  primary_edit: TextEdit | null;
  cursor: number | null;
  additional_edits: TextEdit[];
  detail: string | null;
  is_disabled: boolean;
  disabled_reason: string | null;
};

export type SignatureHelp = {
  label: string;
  params: string[];
  active_param: number;
};

export type CompletionOutput = {
  items: CompletionItem[];
  replace: Span;
  signature_help: SignatureHelp | null;
};

export async function initWasm(): Promise<void> {
  await init();
}

export function analyzeSource(source: string, contextJson?: string): AnalyzeResult {
  return wasm.analyze(source, contextJson) as AnalyzeResult;
}

export function completeSource(
  source: string,
  cursorUtf16: number,
  contextJson?: string,
): CompletionOutput {
  return wasm.complete(source, cursorUtf16, contextJson) as CompletionOutput;
}
