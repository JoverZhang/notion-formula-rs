import init, * as wasm from "../pkg/analyzer_wasm.js";
import type {
  AnalyzeResult,
  AnalyzerConfig,
  ApplyResult,
  CompletionItem as CompletionItemDto,
  HelpResult,
  SignatureHelp as SignatureHelpDto,
  TextEdit,
} from "./generated/wasm_dto";

export type { AnalyzerConfig, Span } from "./generated/wasm_dto";
export type CompletionItem = CompletionItemDto;
export type SignatureHelp = SignatureHelpDto;

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
let analyzer: wasm.Analyzer | null = null;

export async function initWasm(config: AnalyzerConfig): Promise<void> {
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
      await init({ module_or_path: wasmBytes });
    } else {
      await init();
    }
    analyzer = new wasm.Analyzer(config);
  })();

  return initPromise;
}

function getAnalyzer(): wasm.Analyzer {
  if (analyzer) {
    return analyzer;
  }
  throw new Error("WASM analyzer is not initialized");
}

export function analyzeSource(source: string): AnalyzeResult {
  return getAnalyzer().analyze(source) as AnalyzeResult;
}

export function formatSource(source: string, cursorUtf16: number): ApplyResult {
  return getAnalyzer().ide_format(source, cursorUtf16) as ApplyResult;
}

export function applyEditsSource(
  source: string,
  edits: TextEdit[],
  cursorUtf16: number,
): ApplyResult {
  return getAnalyzer().ide_apply_edits(source, edits, cursorUtf16) as ApplyResult;
}

export function helpSource(source: string, cursor: number): HelpResult {
  return getAnalyzer().ide_help(source, cursor) as HelpResult;
}

export function buildCompletionState(source: string, cursor: number): CompletionState {
  const output = helpSource(source, cursor);
  const completion = output.completion;
  return {
    items: completion?.items ?? [],
    signatureHelp: output.signature_help ?? null,
    preferredIndices: Array.isArray(completion?.preferred_indices)
      ? completion.preferred_indices.filter((n) => typeof n === "number")
      : [],
  };
}

export function safeBuildCompletionState(source: string, cursor: number): CompletionState {
  try {
    return buildCompletionState(source, cursor);
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
