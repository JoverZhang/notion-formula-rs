import type { FormulaId } from "../app/types";
import type { NfDebug, PanelDebugHandle } from "./common";

export function isDebugEnabled(): boolean {
  if (typeof window !== "object" || window === null) return false;
  const env = import.meta.env as { DEV?: boolean; MODE?: string };
  if (env.DEV || env.MODE === "test") return true;
  return new URLSearchParams(window.location.search).get("debug") === "1";
}

export const DEBUG_ENABLED = isDebugEnabled();

const panelHandles = new Map<FormulaId, PanelDebugHandle>();
const handleFor = (id: FormulaId) => {
  const handle = panelHandles.get(id);
  if (!handle) throw new Error(`Unknown formula panel: ${id}`);
  return handle;
};

const debugApi: NfDebug = {
  listPanels: () => Array.from(panelHandles.keys()),
  getState: (id) => handleFor(id).getState(),
  getSelectionHead: (id) => handleFor(id).getSelectionHead(),
  getAnalyzerDiagnostics: (id) => handleFor(id).getAnalyzerDiagnostics(),
  getCmDiagnostics: (id) => handleFor(id).getCmDiagnostics(),
  getTokenDecorations: (id) => handleFor(id).getTokenDecorations(),
  getChipSpans: (id) => handleFor(id).getChipSpans(),
  getChipUiRanges: (id) => handleFor(id).getChipUiRanges(),
  toChipPos: (id, rawUtf16Pos) => handleFor(id).toChipPos(rawUtf16Pos),
  toRawPos: (id, chipPos) => handleFor(id).toRawPos(chipPos),
  setSelectionHead: (id, pos) => handleFor(id).setSelectionHead(pos),
  isChipUiEnabled: () => {
    const first = panelHandles.values().next();
    return first.done ? false : first.value.isChipUiEnabled();
  },
  getChipUiCount: (id) => handleFor(id).getChipUiCount(),
};

export function registerPanelDebug(id: FormulaId, handle: PanelDebugHandle): void {
  if (!DEBUG_ENABLED) return;
  panelHandles.set(id, handle);
  if (!window.__nf_debug) window.__nf_debug = debugApi;
}
