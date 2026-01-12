import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import type { FormulaId } from "../app/types";
import type { TokenDecorationRange } from "../editor_decorations";
import type {
  DebugChipUiRange,
  DebugCmDiag,
  DebugTokenDeco,
  NfDebug,
  PanelDebugHandle,
} from "./common";

export function isDebugEnabled(): boolean {
  if (typeof window !== "object" || window === null) return false;
  const env = import.meta.env as unknown as { DEV?: boolean; MODE?: string };
  if (env.DEV || env.MODE === "test") return true;
  const params = new URLSearchParams(window.location.search);
  return params.get("debug") === "1";
}

export const DEBUG_ENABLED = isDebugEnabled();

const panelHandles = new Map<FormulaId, PanelDebugHandle>();

function normalizeTokenDecos(ranges: TokenDecorationRange[]): DebugTokenDeco[] {
  return ranges.map((range) => ({
    from: range.from,
    to: range.to,
    className: range.className,
  }));
}

function normalizeCmDiagnostics(diags: CmDiagnostic[]): DebugCmDiag[] {
  return diags.map(
    (diag): DebugCmDiag => ({
      from: diag.from,
      to: diag.to,
      severity: diag.severity ?? "error",
      message: diag.message,
    }),
  );
}

function normalizeChipUiRanges(ranges: DebugChipUiRange[]): DebugChipUiRange[] {
  return ranges.map((range) => ({
    from: range.from,
    to: range.to,
    propName: range.propName,
    hasError: range.hasError,
    hasWarning: range.hasWarning,
  }));
}

function getHandle(id: FormulaId): PanelDebugHandle {
  const handle = panelHandles.get(id);
  if (!handle) {
    throw new Error(`Unknown formula panel: ${id}`);
  }
  return handle;
}

function getAnyHandle(): PanelDebugHandle | null {
  const iter = panelHandles.values().next();
  return iter.done ? null : iter.value;
}

function createDebugApi(): NfDebug {
  return {
    listPanels() {
      return Array.from(panelHandles.keys());
    },
    getState(id) {
      return getHandle(id).getState();
    },
    getSelectionHead(id) {
      return getHandle(id).getSelectionHead();
    },
    getAnalyzerDiagnostics(id) {
      return getHandle(id).getAnalyzerDiagnostics();
    },
    getCmDiagnostics(id) {
      return normalizeCmDiagnostics(getHandle(id).getCmDiagnostics());
    },
    getTokenDecorations(id) {
      return normalizeTokenDecos(getHandle(id).getTokenDecorations());
    },
    getChipSpans(id) {
      return getHandle(id)
        .getChipSpans()
        .map((span) => ({ start: span.start, end: span.end }));
    },
    getChipUiRanges(id) {
      return normalizeChipUiRanges(getHandle(id).getChipUiRanges());
    },
    toChipPos(id, rawUtf16Pos) {
      return getHandle(id).toChipPos(rawUtf16Pos);
    },
    toRawPos(id, chipPos) {
      return getHandle(id).toRawPos(chipPos);
    },
    setSelectionHead(id, pos) {
      return getHandle(id).setSelectionHead(pos);
    },
    isChipUiEnabled() {
      return getAnyHandle()?.isChipUiEnabled() ?? false;
    },
    getChipUiCount(id) {
      return getHandle(id).getChipUiCount();
    },
  };
}

function ensureDebugApi() {
  if (!DEBUG_ENABLED) return;
  if (!window.__nf_debug) {
    window.__nf_debug = createDebugApi();
  }
}

export function registerPanelDebug(id: FormulaId, handle: PanelDebugHandle): void {
  if (!DEBUG_ENABLED) return;
  panelHandles.set(id, handle);
  ensureDebugApi();
}
