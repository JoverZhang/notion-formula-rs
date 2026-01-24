import init, * as wasm from "../pkg/analyzer_wasm.js";
import type {
  AnalyzeResult,
  CompletionItemView,
  CompletionOutputView,
  SignatureHelpView,
  TextEditView,
  Utf16Span,
} from "./generated/wasm_dto";

export type Span = Utf16Span;
export type TextEdit = TextEditView;
export type CompletionItem = CompletionItemView;
export type SignatureHelp = SignatureHelpView;
export type CompletionOutput = CompletionOutputView;

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
