import { defaultKeymap } from "@codemirror/commands";
import { linter } from "@codemirror/lint";
import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { EditorState, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import { Decoration, DecorationSet, EditorView, keymap } from "@codemirror/view";
import { completeSource, type CompletionItem, type SignatureHelp } from "../analyzer/wasm_client";
import { CONTEXT_JSON, PROPERTY_SCHEMA } from "../app/context";
import type { FormulaId, FormulaState, AnalyzerDiagnostic } from "../app/types";
import { buildChipOffsetMap, type ChipOffsetMap, type ChipSpan } from "../chip_spans";
import { registerPanelDebug } from "../debug/debug_bridge";
import {
  chipAtomicRangesExt,
  chipDecoStateField,
  chipRangesField,
  formulaIdFacet,
  setChipDecoListEffect,
  type ChipDecorationRange,
} from "../editor/chip_decorations";
import { applyCompletion } from "../editor/text_edits";
import {
  computePropChips,
  computeTokenDecorationRanges,
  getTokenSpanIssues,
  setTokenDecoListEffect,
  sortTokens,
  type Chip,
  type TokenDecorationRange,
  tokenDecoStateField,
} from "../editor_decorations";
import type { Token } from "../editor_decorations";

type FormulaPanelView = {
  root: HTMLElement;
  mount(parent: HTMLElement): void;
  update(state: FormulaState): void;
};

type PropName = (typeof PROPERTY_SCHEMA)[number]["name"];
const VALID_PROP_NAMES = new Set<PropName>(PROPERTY_SCHEMA.map((prop) => prop.name));

function isValidPropName(name: string): name is PropName {
  return VALID_PROP_NAMES.has(name as PropName);
}

function isValidPropChip(chip: Chip): chip is Chip & { argValue: PropName } {
  return isValidPropName(chip.argValue);
}

const setLintDiagnosticsEffect = StateEffect.define<CmDiagnostic[]>();
const lintDiagnosticsStateField = StateField.define<CmDiagnostic[]>({
  create() {
    return [];
  },
  update(value, tr) {
    for (const e of tr.effects) {
      if (e.is(setLintDiagnosticsEffect)) return e.value;
    }
    return value;
  },
});

function toCmSeverity(kind?: string): "error" | "warning" | "info" {
  if (kind === "warning") return "warning";
  if (kind === "info") return "info";
  return "error";
}

function clamp(n: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, n));
}

function remapDiagnosticToChip(
  from: number,
  to: number,
  chipSpans: ChipSpan[] | undefined,
): { from: number; to: number } {
  if (!chipSpans || chipSpans.length === 0) return { from, to };
  for (const span of chipSpans) {
    if (from < span.end && to > span.start) {
      return { from: span.start, to: span.end };
    }
  }
  return { from, to };
}

function chipIntersectsRange(chip: ChipDecorationRange, from: number, to: number): boolean {
  return from < chip.to && to > chip.from;
}

function applyDiagnosticsToChipRanges(
  ranges: ChipDecorationRange[],
  diagnostics: AnalyzerDiagnostic[],
  docLen: number,
): ChipDecorationRange[] {
  if (!ranges || ranges.length === 0) return [];
  if (!diagnostics || diagnostics.length === 0) {
    return ranges.map((range) => ({
      ...range,
      hasError: false,
      hasWarning: false,
      message: undefined,
    }));
  }

  return ranges.map((range) => {
    let hasError = false;
    let hasWarning = false;
    let message: string | undefined;
    for (const diag of diagnostics) {
      const start = diag.span?.start;
      if (typeof start !== "number") continue;
      const end = diag.span?.end;
      const from = clamp(start, 0, docLen);
      const toRaw = typeof end === "number" ? end : start + 1;
      const to = clamp(Math.max(toRaw, from + 1), 0, docLen);
      if (!chipIntersectsRange(range, from, to)) continue;

      if (!hasError) {
        message = diag.message;
      }
      hasError = true;
    }
    return {
      ...range,
      hasError,
      hasWarning,
      message,
    };
  });
}

function analyzerToCmDiagnostics(
  diags: AnalyzerDiagnostic[],
  docLen: number,
  chipSpans?: ChipSpan[],
): CmDiagnostic[] {
  const out: CmDiagnostic[] = [];
  for (const d of diags) {
    const start = d.span?.start;
    const end = d.span?.end;
    if (typeof start !== "number") continue;

    let from = clamp(start, 0, docLen);
    const toRaw = typeof end === "number" ? end : start + 1;
    let to = clamp(Math.max(toRaw, from + 1), 0, docLen);
    const remapped = remapDiagnosticToChip(from, to, chipSpans);
    from = clamp(remapped.from, 0, docLen);
    to = clamp(Math.max(remapped.to, from + 1), 0, docLen);

    out.push({
      from,
      to,
      severity: toCmSeverity(d.kind),
      message: d.message || "(no message)",
    });
  }
  return out;
}

function formatChipPosLabel(
  diag: AnalyzerDiagnostic,
  chipMap: ChipOffsetMap | null,
  chipSpans: ChipSpan[],
): string | null {
  if (!chipMap) return null;
  const start = diag.span?.start;
  const end = diag.span?.end;
  if (typeof start !== "number") return null;
  const rawEnd = typeof end === "number" ? end : start + 1;
  const normalizedEnd = Math.max(rawEnd, start + 1);

  for (const span of chipSpans) {
    if (start < span.end && normalizedEnd > span.start) {
      const chipStart = chipMap.toChipPos(span.start);
      return `chipPos=[${chipStart},${chipStart + 1})`;
    }
  }

  const chipStart = chipMap.toChipPos(start);
  const chipEnd = chipMap.toChipPos(normalizedEnd);
  return `chipPos=[${chipStart},${Math.max(chipEnd, chipStart + 1)})`;
}

function renderDiagnostics(
  listEl: HTMLUListElement,
  diagnostics: AnalyzerDiagnostic[],
  chipMap: ChipOffsetMap | null,
  chipSpans: ChipSpan[],
) {
  listEl.innerHTML = "";
  if (!diagnostics || diagnostics.length === 0) {
    const li = document.createElement("li");
    li.textContent = "No diagnostics";
    listEl.appendChild(li);
    return;
  }

  diagnostics.forEach((diag) => {
    const li = document.createElement("li");
    const kind = diag.kind || "error";
    const line = diag.span?.line ?? 0;
    const col = diag.span?.col ?? 0;
    const chipLabel = formatChipPosLabel(diag, chipMap, chipSpans);
    const position = chipLabel ? `${chipLabel} line=${line} col=${col}` : `line=${line} col=${col}`;
    li.textContent = `${kind}: ${diag.message} ${position}`;
    listEl.appendChild(li);
  });
}

function setWarningVisible(warningEl: HTMLElement, visible: boolean) {
  warningEl.classList.toggle("hidden", !visible);
}

function buildTokenDecorations(
  source: string,
  tokens: Token[],
): { ok: boolean; decoSet: DecorationSet; ranges: TokenDecorationRange[] } {
  if (!tokens || tokens.length === 0) {
    return { ok: true, decoSet: Decoration.none, ranges: [] };
  }

  const builder = new RangeSetBuilder<Decoration>();
  const docLen = source.length;
  const ranges = computeTokenDecorationRanges(docLen, tokens);
  for (const range of ranges) {
    builder.add(range.from, range.to, Decoration.mark({ class: range.className }));
  }

  const issues = getTokenSpanIssues(docLen, tokens);
  const ok = !issues.outOfBounds && !issues.overlap;
  return { ok, decoSet: builder.finish(), ranges };
}

export function createFormulaPanelView(opts: {
  id: FormulaId;
  label: string;
  initialSource: string;
  note?: string;
  onSourceChange: (id: FormulaId, source: string) => void;
}): FormulaPanelView {
  const panel = document.createElement("section");
  panel.className = "formula-panel";
  panel.setAttribute("data-testid", "formula-panel");
  panel.setAttribute("data-formula-id", opts.id);

  const leftCol = document.createElement("div");
  leftCol.className = "formula-left";

  const labelEl = document.createElement("div");
  labelEl.className = "formula-label";
  labelEl.textContent = opts.label;
  leftCol.appendChild(labelEl);

  if (opts.note) {
    const noteEl = document.createElement("div");
    noteEl.className = "formula-note";
    noteEl.textContent = opts.note;
    leftCol.appendChild(noteEl);
  }

  const warningEl = document.createElement("div");
  warningEl.className = "warning hidden";
  warningEl.textContent = "Token spans overlap or exceed the document length.";
  leftCol.appendChild(warningEl);

  const editorEl = document.createElement("div");
  editorEl.className = "editor";
  editorEl.setAttribute("data-testid", "formula-editor");
  editorEl.setAttribute("data-formula-id", opts.id);
  leftCol.appendChild(editorEl);

  const actionsRow = document.createElement("div");
  actionsRow.className = "formula-actions";
  leftCol.appendChild(actionsRow);

  const formatBtn = document.createElement("button");
  formatBtn.className = "format-button";
  formatBtn.type = "button";
  formatBtn.textContent = "Format";
  formatBtn.setAttribute("data-testid", "format-button");
  formatBtn.setAttribute("data-formula-id", opts.id);
  actionsRow.appendChild(formatBtn);

  const formatStatus = document.createElement("div");
  formatStatus.className = "format-status";
  formatStatus.setAttribute("data-testid", "format-status");
  formatStatus.setAttribute("data-formula-id", opts.id);
  actionsRow.appendChild(formatStatus);

  const completionPanel = document.createElement("div");
  completionPanel.className = "completion-panel";
  completionPanel.setAttribute("data-testid", "completion-panel");
  completionPanel.setAttribute("data-formula-id", opts.id);
  leftCol.appendChild(completionPanel);

  const completionHeader = document.createElement("div");
  completionHeader.className = "completion-header";
  completionHeader.textContent = "Suggestions";
  completionPanel.appendChild(completionHeader);

  const signatureEl = document.createElement("div");
  signatureEl.className = "completion-signature hidden";
  completionPanel.appendChild(signatureEl);

  const itemsEl = document.createElement("ul");
  itemsEl.className = "completion-items";
  completionPanel.appendChild(itemsEl);

  const emptyEl = document.createElement("div");
  emptyEl.className = "completion-empty";
  emptyEl.textContent = "No suggestions";
  completionPanel.appendChild(emptyEl);

  const diagTitle = document.createElement("div");
  diagTitle.className = "diagnostics-title";
  diagTitle.textContent = "Diagnostics";
  leftCol.appendChild(diagTitle);

  const diagnosticsEl = document.createElement("ul");
  diagnosticsEl.className = "diag-list";
  diagnosticsEl.setAttribute("data-testid", "formula-diagnostics");
  diagnosticsEl.setAttribute("data-formula-id", opts.id);
  leftCol.appendChild(diagnosticsEl);

  panel.appendChild(leftCol);

  let completionItems: CompletionItem[] = [];
  let signatureHelp: SignatureHelp | null = null;
  let selectedIndex = -1;
  let completionTimer: ReturnType<typeof setTimeout> | null = null;
  let statusTimer: ReturnType<typeof setTimeout> | null = null;
  const COMPLETION_DEBOUNCE_MS = 120;

  function setStatus(text: string, kind: "ok" | "warning" | "error" | "muted") {
    formatStatus.textContent = text;
    formatStatus.dataset.kind = kind;
    if (statusTimer) clearTimeout(statusTimer);
    statusTimer = setTimeout(() => {
      formatStatus.textContent = "";
      delete formatStatus.dataset.kind;
      statusTimer = null;
    }, 2200);
  }

  function renderSignature(sig: SignatureHelp | null) {
    if (!sig) {
      signatureEl.classList.add("hidden");
      signatureEl.textContent = "";
      return;
    }
    signatureEl.classList.remove("hidden");
    const params = sig.params.map((p, idx) => (idx === sig.active_param ? `[${p}]` : p));
    signatureEl.textContent = `${sig.label} ${params.length ? "— " + params.join(", ") : ""}`;
  }

  function renderItems() {
    itemsEl.innerHTML = "";
    if (!completionItems || completionItems.length === 0) {
      emptyEl.classList.remove("hidden");
      return;
    }
    emptyEl.classList.add("hidden");

    completionItems.forEach((item, idx) => {
      const li = document.createElement("li");
      li.className = "completion-item";
      if (idx === selectedIndex) li.classList.add("is-selected");
      if (item.is_disabled) li.classList.add("is-disabled");
      li.setAttribute("data-completion-index", String(idx));

      const main = document.createElement("div");
      main.className = "completion-item-main";
      const label = document.createElement("div");
      label.className = "completion-item-label";
      label.textContent = item.label;
      const meta = document.createElement("div");
      meta.className = "completion-item-meta";
      const detail = item.detail ?? (item.is_disabled ? (item.disabled_reason ?? "") : "");
      meta.textContent = detail;
      main.append(label, meta);
      li.appendChild(main);

      li.addEventListener("mouseenter", () => {
        if (!completionItems.length) return;
        selectedIndex = idx;
        renderItems();
      });

      li.addEventListener("mousedown", (e) => {
        e.preventDefault();
      });

      li.addEventListener("click", () => {
        applyCompletionItem(idx);
      });

      itemsEl.appendChild(li);
    });
  }

  function rerenderCompletions() {
    renderSignature(signatureHelp);
    if (selectedIndex >= completionItems.length) selectedIndex = completionItems.length - 1;
    renderItems();
  }

  function requestCompletions(view: EditorView) {
    if (completionTimer) clearTimeout(completionTimer);
    completionTimer = setTimeout(() => {
      completionTimer = null;
      const source = view.state.doc.toString();
      const cursor = view.state.selection.main.head;
      try {
        const output = completeSource(source, cursor, CONTEXT_JSON);
        completionItems = output.items ?? [];
        signatureHelp = output.signature_help ?? null;
      } catch {
        completionItems = [];
        signatureHelp = null;
      }
      if (completionItems.length === 0) selectedIndex = -1;
      rerenderCompletions();
    }, COMPLETION_DEBOUNCE_MS);
  }

  function selectNext(delta: number) {
    if (!completionItems.length) return;
    if (selectedIndex < 0) {
      selectedIndex = delta > 0 ? 0 : completionItems.length - 1;
    } else {
      selectedIndex = (selectedIndex + delta + completionItems.length) % completionItems.length;
    }
    renderItems();
  }

  function applyCompletionItem(index: number): boolean {
    const item = completionItems[index];
    if (!item || item.is_disabled || !item.primary_edit) return false;

    const source = editorView.state.doc.toString();
    const { newText, newCursor } = applyCompletion(source, item);
    const edits = [item.primary_edit, ...(item.additional_edits ?? [])];
    const changes = edits
      .map((e) => ({ from: e.range.start, to: e.range.end, insert: e.new_text }))
      .sort((a, b) => a.from - b.from || a.to - b.to);

    editorView.dispatch({
      changes,
      selection: { anchor: Math.max(0, Math.min(newCursor, newText.length)) },
    });
    editorView.focus();

    selectedIndex = -1;
    requestCompletions(editorView);
    return true;
  }

  const editorView = new EditorView({
    state: EditorState.create({
      doc: opts.initialSource,
      extensions: [
        keymap.of([
          {
            key: "ArrowDown",
            run: () => {
              if (!completionItems.length) return false;
              selectNext(1);
              return true;
            },
          },
          {
            key: "ArrowUp",
            run: () => {
              if (!completionItems.length) return false;
              selectNext(-1);
              return true;
            },
          },
          {
            key: "Escape",
            run: () => {
              if (selectedIndex < 0) return false;
              selectedIndex = -1;
              renderItems();
              return true;
            },
          },
          {
            key: "Enter",
            run: () => {
              if (selectedIndex < 0) return false;
              return applyCompletionItem(selectedIndex);
            },
          },
          {
            key: "Tab",
            run: () => {
              if (selectedIndex < 0) return false;
              return applyCompletionItem(selectedIndex);
            },
          },
        ]),
        keymap.of(defaultKeymap),
        EditorView.lineWrapping,
        formulaIdFacet.of(opts.id),
        tokenDecoStateField,
        chipDecoStateField,
        chipRangesField,
        chipAtomicRangesExt,
        lintDiagnosticsStateField,
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            opts.onSourceChange(opts.id, update.state.doc.toString());
          }
          if (update.docChanged || update.selectionSet) {
            requestCompletions(update.view);
          }
        }),
        linter((view) => view.state.field(lintDiagnosticsStateField)),
      ],
    }),
    parent: editorEl,
  });

  let lastDiagnostics: AnalyzerDiagnostic[] = [];
  let lastCmDiagnostics: CmDiagnostic[] = [];
  let lastTokenRanges: TokenDecorationRange[] = [];
  let lastChipUiRanges: ChipDecorationRange[] = [];
  let lastValidChipSpans: ChipSpan[] = [];
  let lastChipMap: ChipOffsetMap | null = null;
  let lastFormatted = "";
  let lastStatus: FormulaState["status"] = "idle";
  let lastSource = opts.initialSource;

  renderDiagnostics(diagnosticsEl, [], null, []);
  rerenderCompletions();

  formatBtn.addEventListener("click", () => {
    const formatted = lastFormatted || "";
    if (!formatted) {
      setStatus("Format unavailable", lastDiagnostics.length ? "error" : "muted");
      return;
    }
    const current = editorView.state.doc.toString();
    if (current === formatted) {
      setStatus("No change", "muted");
      return;
    }
    const cursor = editorView.state.selection.main.head;
    editorView.dispatch({
      changes: { from: 0, to: editorView.state.doc.length, insert: formatted },
      selection: { anchor: Math.min(cursor, formatted.length) },
    });
    editorView.focus();
    setStatus("Formatted", lastDiagnostics.length ? "warning" : "ok");
  });

  registerPanelDebug(opts.id, {
    getState: () => ({
      source: lastSource,
      formatted: lastFormatted,
      diagnosticsCount: lastDiagnostics.length,
      tokenCount: lastTokenRanges.length,
      status: lastStatus,
    }),
    getSelectionHead: () => editorView.state.selection.main.head,
    getAnalyzerDiagnostics: () => lastDiagnostics,
    getCmDiagnostics: () => lastCmDiagnostics,
    getTokenDecorations: () => lastTokenRanges,
    getChipSpans: () => lastValidChipSpans,
    toChipPos: (rawPos) => (lastChipMap ? lastChipMap.toChipPos(rawPos) : rawPos),
    toRawPos: (chipPos) => (lastChipMap ? lastChipMap.toRawPos(chipPos) : chipPos),
    // Future chip UI must reflect actual chip widgets/decorations here.
    isChipUiEnabled: () => true,
    getChipUiCount: () => lastChipUiRanges.length,
    getChipUiRanges: () =>
      lastChipUiRanges.map((range) => ({
        from: range.from,
        to: range.to,
        propName: range.propName,
        hasError: range.hasError ?? false,
        hasWarning: range.hasWarning ?? false,
      })),
    setSelectionHead: (pos) => {
      editorView.dispatch({ selection: { anchor: pos } });
      editorView.focus();
    },
  });

  return {
    root: panel,
    mount(parent: HTMLElement) {
      parent.appendChild(panel);
    },
    update(state: FormulaState) {
      lastSource = state.source;
      lastStatus = state.status;
      lastDiagnostics = state.diagnostics;
      lastFormatted = state.formatted;

      const sortedTokens = sortTokens(state.tokens || []);
      const { ok, decoSet, ranges } = buildTokenDecorations(state.source, sortedTokens);
      lastTokenRanges = ranges;
      setWarningVisible(warningEl, !ok);
      editorView.dispatch({ effects: setTokenDecoListEffect.of(ok ? decoSet : Decoration.none) });

      try {
        const docLen = state.source.length;
        const chips = computePropChips(state.source, sortedTokens);
        const validChips = chips.filter(isValidPropChip);
        const rawChipRanges = validChips
          .map((chip) => ({
            from: chip.spanStart,
            to: chip.spanEnd,
            propName: chip.argValue,
          }))
          .filter((range) => range.from >= 0 && range.to > range.from && range.to <= docLen);
        lastChipUiRanges = applyDiagnosticsToChipRanges(rawChipRanges, state.diagnostics, docLen);
        lastValidChipSpans = validChips
          .map((chip) => ({ start: chip.spanStart, end: chip.spanEnd }))
          .filter((span) => span.start >= 0 && span.end > span.start && span.end <= docLen)
          .sort((a, b) => a.start - b.start || a.end - b.end);
        editorView.dispatch({ effects: setChipDecoListEffect.of(lastChipUiRanges) });
      } catch {
        lastChipUiRanges = [];
        lastValidChipSpans = [];
        editorView.dispatch({ effects: setChipDecoListEffect.of([]) });
      }

      try {
        lastChipMap = buildChipOffsetMap(state.source.length, lastValidChipSpans);
      } catch {
        lastChipMap = null;
      }

      renderDiagnostics(diagnosticsEl, state.diagnostics, lastChipMap, lastValidChipSpans);
      const cmDiags = analyzerToCmDiagnostics(
        state.diagnostics,
        state.source.length,
        lastValidChipSpans,
      );
      lastCmDiagnostics = cmDiags;
      editorView.dispatch({ effects: setLintDiagnosticsEffect.of(cmDiags) });
    },
  };
}
