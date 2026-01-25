import init, * as wasm from "../pkg/analyzer_wasm.js";
import type {
  AnalyzeResult,
  CompletionItemView,
  CompletionOutputView,
  LineColView,
  SignatureHelpView,
  TextEditView,
} from "./generated/wasm_dto";

export type { Span } from "./generated/wasm_dto";
export type TextEdit = TextEditView;
export type CompletionItem = CompletionItemView;
export type SignatureHelp = SignatureHelpView;
export type CompletionOutput = CompletionOutputView;

export async function initWasm(): Promise<void> {
  // wasm-pack's default init() uses `fetch(new URL(..., import.meta.url))`, which doesn't
  // support `file://` in Node. For tests, pass the `.wasm` bytes explicitly.
  if (typeof process !== "undefined" && Boolean(process.versions?.node)) {
    const { readFile } = await import("node:fs/promises");
    const { fileURLToPath } = await import("node:url");
    const wasmUrl = new URL("../pkg/analyzer_wasm_bg.wasm", import.meta.url);
    const wasmBytes = await readFile(fileURLToPath(wasmUrl));
    await init(wasmBytes);
    return;
  }

  await init();
}

export function analyzeSource(source: string, contextJson: string): AnalyzeResult {
  return wasm.analyze(source, contextJson) as AnalyzeResult;
}

export function completeSource(
  source: string,
  cursor: number,
  contextJson: string,
): CompletionOutput {
  return wasm.complete(source, cursor, contextJson) as CompletionOutput;
}

export function posToLineCol(source: string, pos: number): LineColView {
  return wasm.pos_to_line_col(source, pos) as LineColView;
}
