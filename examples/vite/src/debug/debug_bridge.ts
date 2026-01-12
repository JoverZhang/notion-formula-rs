import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import type { FormulaId } from "../app/types";
import type { TokenDecorationRange } from "../editor_decorations";
import {
  getWindow,
  type DebugCmDiag,
  type DebugTokenDeco,
  type NfDebug,
  type PanelDebugHandle,
} from "./common";

export function isDebugEnabled(): boolean {
  const w = getWindow();
  if (!w) return false;
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

function getHandle(id: FormulaId): PanelDebugHandle {
  const handle = panelHandles.get(id);
  if (!handle) {
    throw new Error(`Unknown formula panel: ${id}`);
  }
  return handle;
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
    toChipPos(id, rawUtf16Pos) {
      return getHandle(id).toChipPos(rawUtf16Pos);
    },
    toRawPos(id, chipPos) {
      return getHandle(id).toRawPos(chipPos);
    },
  };
}

function ensureDebugApi() {
  if (!DEBUG_ENABLED) return;
  const w = getWindow();
  if (w && !w.__nf_debug) {
    w.__nf_debug = createDebugApi();
  }
}

export function registerPanelDebug(id: FormulaId, handle: PanelDebugHandle): void {
  if (!DEBUG_ENABLED) return;
  panelHandles.set(id, handle);
  ensureDebugApi();
}
