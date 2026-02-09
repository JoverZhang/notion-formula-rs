import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { linter } from "@codemirror/lint";
import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { EditorState, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import { Decoration, DecorationSet, EditorView, keymap } from "@codemirror/view";
import {
  applyCompletionItem as buildCompletionApplyResult,
  safeBuildCompletionState,
  type CompletionItem,
  type SignatureHelp,
} from "../analyzer/wasm_client";
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
import { mergeChipRangesWithDiagnostics } from "./chip_ranges";
import { toCmDiagnostics } from "./codemirror_diagnostics";
import {
  buildCompletionRows,
  getSelectedItemIndex,
  nextSelectedRowIndex,
  normalizeSelectedRowIndex,
  type CompletionRenderRow,
} from "./completion_rows";
import { buildDiagnosticTextRows } from "./diagnostics_rows";
import { createSignaturePopover } from "./signature_popover";

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

type ActiveFormulaPanelUi = {
  show(): void;
  hide(): void;
};

const activeFormulaPanelUiById = new Map<FormulaId, ActiveFormulaPanelUi>();
let activeFormulaPanelId: FormulaId | null = null;

function setActiveFormulaPanel(id: FormulaId) {
  if (activeFormulaPanelId === id) return;
  if (activeFormulaPanelId) {
    activeFormulaPanelUiById.get(activeFormulaPanelId)?.hide();
  }
  activeFormulaPanelId = id;
  activeFormulaPanelUiById.get(id)?.show();
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

function renderDiagnostics(
  listEl: HTMLUListElement,
  source: string,
  diagnostics: AnalyzerDiagnostic[],
  chipMap: ChipOffsetMap | null,
  chipSpans: ChipSpan[],
) {
  listEl.innerHTML = "";
  const rows = buildDiagnosticTextRows(source, diagnostics, chipMap, chipSpans);
  rows.forEach((row) => {
    const li = document.createElement("li");
    li.textContent = row;
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

  const editorWrap = document.createElement("div");
  editorWrap.className = "formula-editor-wrap";
  leftCol.appendChild(editorWrap);

  const signatureEl = document.createElement("div");
  signatureEl.className = "completion-signature hidden";
  signatureEl.setAttribute("data-testid", "suggestion-signature");
  signatureEl.setAttribute("data-formula-id", opts.id);
  editorWrap.appendChild(signatureEl);

  const editorEl = document.createElement("div");
  editorEl.className = "editor";
  editorEl.setAttribute("data-testid", "formula-editor");
  editorEl.setAttribute("data-formula-id", opts.id);
  editorWrap.appendChild(editorEl);

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

  const diagTitle = document.createElement("div");
  diagTitle.className = "diagnostics-title";
  diagTitle.textContent = "Diagnostics";
  leftCol.appendChild(diagTitle);

  const diagnosticsEl = document.createElement("ul");
  diagnosticsEl.className = "diag-list";
  diagnosticsEl.setAttribute("data-testid", "formula-diagnostics");
  diagnosticsEl.setAttribute("data-formula-id", opts.id);
  leftCol.appendChild(diagnosticsEl);

  const completionPanel = document.createElement("div");
  completionPanel.className = "completion-panel hidden";
  completionPanel.setAttribute("data-testid", "completion-panel");
  completionPanel.setAttribute("data-formula-id", opts.id);
  leftCol.appendChild(completionPanel);

  const completionHeader = document.createElement("div");
  completionHeader.className = "completion-header";
  completionHeader.textContent = "Completions";
  completionPanel.appendChild(completionHeader);

  const itemsEl = document.createElement("ul");
  itemsEl.className = "completion-items";
  completionPanel.appendChild(itemsEl);

  const emptyEl = document.createElement("div");
  emptyEl.className = "completion-empty";
  emptyEl.textContent = "No suggestions";
  completionPanel.appendChild(emptyEl);

  panel.appendChild(leftCol);

  let isUiActive = false;
  let completionItems: CompletionItem[] = [];
  let signatureHelp: SignatureHelp | null = null;
  let preferredCompletionIndices: number[] = [];
  let completionRows: CompletionRenderRow[] = [];
  let selectedRowIndex = -1;
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

  const signaturePopover = createSignaturePopover(signatureEl, editorWrap);

  function renderSignature(sig: SignatureHelp | null) {
    signaturePopover.render(sig, isUiActive);
  }

  function renderItems() {
    itemsEl.innerHTML = "";
    if (!completionRows || completionRows.length === 0) {
      emptyEl.classList.remove("hidden");
      return;
    }
    emptyEl.classList.add("hidden");

    completionRows.forEach((row, rowIndex) => {
      const li = document.createElement("li");
      if (row.type === "header") {
        li.className = "completion-group-header";
        li.textContent = row.label;
        li.setAttribute("data-completion-group", row.kind);
        itemsEl.appendChild(li);
        return;
      }
      if (row.type === "section") {
        li.className = "completion-recommended-header";
        li.textContent = row.label;
        li.setAttribute("data-completion-section", row.section);
        itemsEl.appendChild(li);
        return;
      }
      if (row.type === "category") {
        li.className = "completion-category-header";
        li.textContent = row.label;
        li.setAttribute("data-completion-category", row.category);
        itemsEl.appendChild(li);
        return;
      }

      const item = row.item;
      li.className = "completion-item";
      if (rowIndex === selectedRowIndex) li.classList.add("is-selected");
      if (item.is_disabled) li.classList.add("is-disabled");
      if (row.isRecommended) {
        li.classList.add("is-recommended");
        li.setAttribute("data-completion-recommended", "true");
      }
      li.setAttribute("data-completion-index", String(row.itemIndex));

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
        if (!completionRows.length) return;
        selectedRowIndex = rowIndex;
        renderItems();
        scrollSelectedIntoView();
      });

      li.addEventListener("mousedown", (e) => {
        e.preventDefault();
      });

      li.addEventListener("click", () => {
        applySelectedCompletion(row.itemIndex);
      });

      itemsEl.appendChild(li);
    });
  }

  function rerenderCompletions() {
    if (isUiActive) {
      renderSignature(signatureHelp);
    } else {
      signaturePopover.hide();
    }
    completionRows = buildCompletionRows(completionItems, preferredCompletionIndices);
    const preferredTop = preferredCompletionIndices[0];
    if (typeof preferredTop === "number") {
      const rowIndex = completionRows.findIndex(
        (row) => row.type === "item" && row.itemIndex === preferredTop,
      );
      if (rowIndex !== -1) selectedRowIndex = rowIndex;
    }
    selectedRowIndex = normalizeSelectedRowIndex(completionRows, selectedRowIndex);
    renderItems();
    scrollSelectedIntoView();
  }

  function requestCompletions(view: EditorView) {
    if (completionTimer) clearTimeout(completionTimer);
    completionTimer = setTimeout(() => {
      completionTimer = null;
      const source = view.state.doc.toString();
      const cursor = view.state.selection.main.head;
      const next = safeBuildCompletionState(source, cursor, CONTEXT_JSON);
      completionItems = next.items;
      signatureHelp = next.signatureHelp;
      preferredCompletionIndices = next.preferredIndices;
      if (completionItems.length === 0) selectedRowIndex = -1;
      rerenderCompletions();
    }, COMPLETION_DEBOUNCE_MS);
  }

  function selectNext(delta: number) {
    selectedRowIndex = nextSelectedRowIndex(completionRows, selectedRowIndex, delta);
    renderItems();
    scrollSelectedIntoView();
  }

  function scrollSelectedIntoView() {
    const selectedItem = itemsEl.querySelector(".completion-item.is-selected");
    if (selectedItem instanceof HTMLElement) {
      selectedItem.scrollIntoView({ block: "nearest" });
    }
  }

  function applySelectedCompletion(index: number): boolean {
    const item = completionItems[index];
    const applyResult = buildCompletionApplyResult(item);
    if (!applyResult) return false;

    editorView.dispatch({
      changes: applyResult.changes,
      selection: { anchor: applyResult.cursor },
    });
    requestAnimationFrame(() => {
      editorView.focus();
    });

    selectedRowIndex = -1;
    requestCompletions(editorView);
    return true;
  }

  const editorView = new EditorView({
    state: EditorState.create({
      doc: opts.initialSource,
      extensions: [
        history(),
        keymap.of([
          {
            key: "ArrowDown",
            run: () => {
              if (!isUiActive) return false;
              if (!completionItems.length) return false;
              selectNext(1);
              return true;
            },
          },
          {
            key: "ArrowUp",
            run: () => {
              if (!isUiActive) return false;
              if (!completionItems.length) return false;
              selectNext(-1);
              return true;
            },
          },
          {
            key: "Escape",
            run: () => {
              if (!isUiActive) return false;
              if (selectedRowIndex < 0) return false;
              selectedRowIndex = -1;
              renderItems();
              return true;
            },
          },
          {
            key: "Enter",
            run: () => {
              if (!isUiActive) return false;
              const itemIndex = getSelectedItemIndex(completionRows, selectedRowIndex);
              if (typeof itemIndex !== "number") return false;
              return applySelectedCompletion(itemIndex);
            },
          },
          {
            key: "Tab",
            run: () => {
              if (!isUiActive) return false;
              const itemIndex = getSelectedItemIndex(completionRows, selectedRowIndex);
              if (typeof itemIndex !== "number") return false;
              return applySelectedCompletion(itemIndex);
            },
          },
        ]),
        keymap.of(historyKeymap),
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

  activeFormulaPanelUiById.set(opts.id, {
    show() {
      isUiActive = true;
      completionPanel.classList.remove("hidden");
      requestCompletions(editorView);
      rerenderCompletions();
    },
    hide() {
      isUiActive = false;
      completionPanel.classList.add("hidden");
      signaturePopover.hide();
    },
  });

  editorView.dom.addEventListener("focusin", () => {
    setActiveFormulaPanel(opts.id);
  });

  window.addEventListener("resize", () => {
    if (!isUiActive) return;
    signaturePopover.updateSide();
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

  renderDiagnostics(diagnosticsEl, lastSource, [], null, []);
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
        lastChipUiRanges = mergeChipRangesWithDiagnostics(rawChipRanges, state.diagnostics, docLen);
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

      renderDiagnostics(
        diagnosticsEl,
        state.source,
        state.diagnostics,
        lastChipMap,
        lastValidChipSpans,
      );
      const cmDiags = toCmDiagnostics(state.diagnostics, state.source.length, lastValidChipSpans);
      lastCmDiagnostics = cmDiags;
      editorView.dispatch({ effects: setLintDiagnosticsEffect.of(cmDiags) });
    },
  };
}
