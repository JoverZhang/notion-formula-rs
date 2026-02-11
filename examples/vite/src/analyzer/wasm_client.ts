import init, * as wasm from "../pkg/analyzer_wasm.js";
import type {
  AnalyzeResult,
  ApplyResultView,
  CompletionItemView,
  CompletionOutputView,
  SignatureHelpView,
  TextEditView,
} from "./generated/wasm_dto";

export type { Span } from "./generated/wasm_dto";
export type CompletionItem = CompletionItemView;
export type SignatureHelp = SignatureHelpView;

export type CompletionState = {
  items: CompletionItem[];
  signatureHelp: SignatureHelp | null;
  preferredIndices: number[];
};

export type CompletionApplyResult = {
  changes: Array<{ from: number; to: number; insert: string }>;
  cursor: number;
};

let initPromise: Promise<void> | null = null;

export async function initWasm(): Promise<void> {
  if (initPromise) {
    return initPromise;
  }

  initPromise = (async () => {
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
  })();

  return initPromise;
}

export function analyzeSource(source: string, contextJson: string): AnalyzeResult {
  return wasm.analyze(source, contextJson) as AnalyzeResult;
}

export function formatSource(source: string, cursorUtf16: number): ApplyResultView {
  return wasm.format(source, cursorUtf16) as ApplyResultView;
}

export function applyEditsSource(
  source: string,
  edits: TextEditView[],
  cursorUtf16: number,
): ApplyResultView {
  return wasm.apply_edits(source, edits, cursorUtf16) as ApplyResultView;
}

export function completeSource(
  source: string,
  cursor: number,
  contextJson: string,
): CompletionOutputView {
  return wasm.complete(source, cursor, contextJson) as CompletionOutputView;
}

export function buildCompletionState(
  source: string,
  cursor: number,
  contextJson: string,
): CompletionState {
  const output = completeSource(source, cursor, contextJson);
  return {
    items: output.items ?? [],
    signatureHelp: output.signature_help ?? null,
    preferredIndices: Array.isArray(output.preferred_indices)
      ? output.preferred_indices.filter((n) => typeof n === "number")
      : [],
  };
}

export function safeBuildCompletionState(
  source: string,
  cursor: number,
  contextJson: string,
): CompletionState {
  try {
    return buildCompletionState(source, cursor, contextJson);
  } catch {
    return { items: [], signatureHelp: null, preferredIndices: [] };
  }
}

export function applyCompletionItem(
  item: CompletionItem | undefined,
): CompletionApplyResult | null {
  if (!item || item.is_disabled || !item.primary_edit) return null;

  // Invariants: edit ranges are in original-document coordinates and can be applied
  // in sorted order; additional edits before the primary edit shift fallback cursor.
  const edits = [item.primary_edit, ...(item.additional_edits ?? [])];
  const changes = edits
    .map((edit) => ({ from: edit.range.start, to: edit.range.end, insert: edit.new_text }))
    .sort((a, b) => a.from - b.from || a.to - b.to);

  let offsetBeforePrimary = 0;
  const primaryStart = item.primary_edit.range.start;
  for (const edit of item.additional_edits ?? []) {
    if (edit.range.end <= primaryStart) {
      offsetBeforePrimary += edit.new_text.length - (edit.range.end - edit.range.start);
    }
  }

  const fallbackCursor = primaryStart + item.primary_edit.new_text.length + offsetBeforePrimary;
  const cursor = Math.max(0, item.cursor ?? fallbackCursor);
  return { changes, cursor };
}
