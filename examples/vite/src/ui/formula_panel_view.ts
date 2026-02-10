import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { linter, type Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { EditorState, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView, keymap } from "@codemirror/view";
import {
  applyCompletionItem,
  safeBuildCompletionState,
  type CompletionItem,
  type SignatureHelp,
} from "../analyzer/wasm_client";
import { CONTEXT_JSON, PROPERTY_SCHEMA } from "../app/context";
import type { AnalyzerDiagnostic, FormulaId, FormulaState } from "../app/types";
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
  setTokenDecoListEffect,
  sortTokens,
  tokenDecoStateField,
  type Chip,
  type TokenDecorationRange,
} from "../editor_decorations";
import {
  buildCompletionRows,
  getSelectedItemIndex,
  nextSelectedRowIndex,
  normalizeSelectedRowIndex,
  COMPLETION_ROW_ITEM_RECOMMENDED,
  COMPLETION_ROW_LABEL_RECOMMENDED,
  type CompletionRenderRow,
} from "../model/completions";
import {
  buildDiagnosticTextRows,
  mergeChipRangesWithDiagnostics,
  toCmDiagnostics,
} from "../model/diagnostics";
import { createSignaturePopover } from "./signature_popover";

type FormulaPanelView = {
  root: HTMLElement;
  mount(parent: HTMLElement): void;
  update(state: FormulaState): void;
};

type PropName = (typeof PROPERTY_SCHEMA)[number]["name"];

const VALID_PROP_NAMES = new Set<PropName>(PROPERTY_SCHEMA.map((prop) => prop.name));
const COMPLETION_DEBOUNCE_MS = 120;

type ActiveFormulaPanelUi = {
  show(): void;
  hide(): void;
};

const activeFormulaPanelUiById = new Map<FormulaId, ActiveFormulaPanelUi>();
let activeFormulaPanelId: FormulaId | null = null;

function setActiveFormulaPanel(id: FormulaId) {
  if (activeFormulaPanelId === id) return;
  if (activeFormulaPanelId) activeFormulaPanelUiById.get(activeFormulaPanelId)?.hide();
  activeFormulaPanelId = id;
  activeFormulaPanelUiById.get(id)?.show();
}

const setLintDiagnosticsEffect = StateEffect.define<CmDiagnostic[]>();
const lintDiagnosticsStateField = StateField.define<CmDiagnostic[]>({
  create() {
    return [];
  },
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setLintDiagnosticsEffect)) return effect.value;
    }
    return value;
  },
});

function must<T extends Element>(root: ParentNode, selector: string): T {
  const node = root.querySelector(selector);
  if (!node) throw new Error(`Missing node: ${selector}`);
  return node as T;
}

function renderDiagnosticList(listEl: HTMLUListElement, rows: string[]) {
  listEl.replaceChildren();
  for (const row of rows) {
    const li = document.createElement("li");
    li.textContent = row;
    listEl.appendChild(li);
  }
}

function isValidPropChip(chip: Chip): chip is Chip & { argValue: PropName } {
  return VALID_PROP_NAMES.has(chip.argValue as PropName);
}

export function createFormulaPanelView(opts: {
  id: FormulaId;
  label: string;
  initialSource: string;
  onSourceChange: (id: FormulaId, source: string) => void;
}): FormulaPanelView {
  const panel = document.createElement("section");
  panel.className = "formula-panel";
  panel.setAttribute("data-testid", "formula-panel");
  panel.setAttribute("data-formula-id", opts.id);

  panel.innerHTML = `
    <div class="formula-left">
      <div class="formula-label"></div>
      <div class="formula-editor-wrap">
        <div class="completion-signature hidden" data-testid="suggestion-signature" data-formula-id="${opts.id}"></div>
        <div class="editor" data-testid="formula-editor" data-formula-id="${opts.id}"></div>
        <div class="formula-actions">
          <button class="format-button" type="button" data-testid="format-button" data-formula-id="${opts.id}">Format</button>
          <div class="formula-output-type" data-testid="formula-output-type" data-formula-id="${opts.id}"></div>
        </div>
        <div class="completion-panel hidden" data-testid="completion-panel" data-formula-id="${opts.id}">
          <div class="completion-header">Completions</div>
          <ul class="completion-items"></ul>
          <div class="completion-empty">No suggestions</div>
        </div>
      </div>
      <div class="diagnostics-title">Diagnostics</div>
      <ul class="diag-list" data-testid="formula-diagnostics" data-formula-id="${opts.id}"></ul>
    </div>
  `;

  const labelEl = must<HTMLElement>(panel, ".formula-label");
  const editorWrap = must<HTMLElement>(panel, ".formula-editor-wrap");
  const signatureEl = must<HTMLElement>(
    panel,
    '.completion-signature[data-testid="suggestion-signature"]',
  );
  const editorEl = must<HTMLElement>(panel, '.editor[data-testid="formula-editor"]');
  const formatBtn = must<HTMLButtonElement>(panel, ".format-button");
  const outputTypeEl = must<HTMLElement>(panel, ".formula-output-type");
  const diagnosticsEl = must<HTMLUListElement>(panel, ".diag-list");
  const completionPanel = must<HTMLElement>(
    panel,
    '.completion-panel[data-testid="completion-panel"]',
  );
  const itemsEl = must<HTMLUListElement>(panel, ".completion-items");
  const emptyEl = must<HTMLElement>(panel, ".completion-empty");

  labelEl.textContent = opts.label;

  let isUiActive = false;
  let completionItems: CompletionItem[] = [];
  let signatureHelp: SignatureHelp | null = null;
  let preferredCompletionIndices: number[] = [];
  let completionRows: CompletionRenderRow[] = [];
  let selectedRowIndex = -1;
  let completionTimer: ReturnType<typeof setTimeout> | null = null;

  const signaturePopover = createSignaturePopover(signatureEl, editorWrap);

  function scrollSelectedIntoView() {
    const selected = itemsEl.querySelector(".completion-item.is-selected");
    if (!(selected instanceof HTMLElement)) return;
    if (itemsEl.clientHeight <= 0) return;

    const listRect = itemsEl.getBoundingClientRect();
    const itemRect = selected.getBoundingClientRect();

    const padding = 2;
    const itemTop = Math.max(0, itemRect.top - listRect.top + itemsEl.scrollTop - padding);
    const itemBottom = Math.max(
      itemTop,
      itemRect.bottom - listRect.top + itemsEl.scrollTop + padding,
    );
    const viewTop = itemsEl.scrollTop;
    const viewBottom = viewTop + itemsEl.clientHeight;

    let nextTop = viewTop;
    if (itemTop < viewTop) {
      nextTop = itemTop;
    } else if (itemBottom > viewBottom) {
      nextTop = itemBottom - itemsEl.clientHeight;
    } else {
      return;
    }

    const maxTop = Math.max(0, itemsEl.scrollHeight - itemsEl.clientHeight);
    itemsEl.scrollTop = Math.min(Math.max(0, nextTop), maxTop);
  }

  function renderCompletionRows() {
    itemsEl.replaceChildren();
    if (!completionRows.length) {
      emptyEl.classList.remove("hidden");
      return;
    }
    emptyEl.classList.add("hidden");

    completionRows.forEach((row, rowIndex) => {
      const li = document.createElement("li");
      if (row.kind === "label") {
        const recommended = (row.flags & COMPLETION_ROW_LABEL_RECOMMENDED) !== 0;
        li.className = recommended ? "completion-recommended-header" : "completion-group-header";
        li.textContent = row.label;
        if (recommended) li.setAttribute("data-completion-section", "recommended");
        itemsEl.appendChild(li);
        return;
      }

      const item = completionItems[row.itemIndex];
      if (!item) return;
      li.className = "completion-item";
      if (rowIndex === selectedRowIndex) li.classList.add("is-selected");
      if (item.is_disabled) li.classList.add("is-disabled");
      if ((row.flags & COMPLETION_ROW_ITEM_RECOMMENDED) !== 0) {
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
      meta.textContent = item.detail ?? (item.is_disabled ? (item.disabled_reason ?? "") : "");
      main.append(label, meta);
      li.appendChild(main);

      li.addEventListener("mouseenter", () => {
        selectedRowIndex = rowIndex;
        renderCompletionRows();
        scrollSelectedIntoView();
      });
      li.addEventListener("mousedown", (event) => event.preventDefault());
      li.addEventListener("click", () => {
        applySelectedCompletion(row.itemIndex);
      });
      itemsEl.appendChild(li);
    });
  }

  function rerenderCompletions() {
    signaturePopover.render(signatureHelp, isUiActive);

    completionRows = buildCompletionRows(completionItems, preferredCompletionIndices);
    const preferredTop = preferredCompletionIndices[0];
    if (typeof preferredTop === "number") {
      selectedRowIndex = completionRows.findIndex(
        (row) => row.kind === "item" && row.itemIndex === preferredTop,
      );
    }
    selectedRowIndex = normalizeSelectedRowIndex(completionRows, selectedRowIndex);
    renderCompletionRows();
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
      if (!completionItems.length) selectedRowIndex = -1;
      rerenderCompletions();
    }, COMPLETION_DEBOUNCE_MS);
  }

  function applySelectedCompletion(index: number): boolean {
    const applyResult = applyCompletionItem(completionItems[index]);
    if (!applyResult) return false;
    editorView.dispatch({
      changes: applyResult.changes,
      selection: { anchor: applyResult.cursor },
    });
    requestAnimationFrame(() => editorView.focus());
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
              selectedRowIndex = nextSelectedRowIndex(completionRows, selectedRowIndex, 1);
              renderCompletionRows();
              scrollSelectedIntoView();
              return true;
            },
          },
          {
            key: "ArrowUp",
            run: () => {
              if (!isUiActive) return false;
              if (!completionItems.length) return false;
              selectedRowIndex = nextSelectedRowIndex(completionRows, selectedRowIndex, -1);
              renderCompletionRows();
              scrollSelectedIntoView();
              return true;
            },
          },
          {
            key: "Escape",
            run: () => {
              if (!isUiActive) return false;
              if (selectedRowIndex < 0) return false;
              selectedRowIndex = -1;
              renderCompletionRows();
              return true;
            },
          },
          {
            key: "Tab",
            run: () => {
              if (!isUiActive) return false;
              const itemIndex = getSelectedItemIndex(completionRows, selectedRowIndex);
              return typeof itemIndex === "number" ? applySelectedCompletion(itemIndex) : false;
            },
          },
          {
            key: "Enter",
            run: () => {
              if (!isUiActive) return false;
              const itemIndex = getSelectedItemIndex(completionRows, selectedRowIndex);
              return typeof itemIndex === "number" ? applySelectedCompletion(itemIndex) : false;
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
          if (update.docChanged) opts.onSourceChange(opts.id, update.state.doc.toString());
          if (update.docChanged || update.selectionSet) requestCompletions(update.view);
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
  let lastChipSpans: ChipSpan[] = [];
  let lastChipMap: ChipOffsetMap | null = null;
  let lastFormatted = "";
  let lastOutputType = "unknown";
  let lastSource = opts.initialSource;

  renderDiagnosticList(diagnosticsEl, ["No diagnostics"]);
  rerenderCompletions();

  formatBtn.addEventListener("click", () => {
    const formatted = lastFormatted || "";
    if (!formatted) return;
    const current = editorView.state.doc.toString();
    if (current === formatted) return;
    const cursor = editorView.state.selection.main.head;
    editorView.dispatch({
      changes: { from: 0, to: editorView.state.doc.length, insert: formatted },
      selection: { anchor: Math.min(cursor, formatted.length) },
    });
    editorView.focus();
  });

  registerPanelDebug(opts.id, {
    getState: () => ({
      source: lastSource,
      formatted: lastFormatted,
      outputType: lastOutputType,
      diagnosticsCount: lastDiagnostics.length,
      tokenCount: lastTokenRanges.length,
    }),
    getSelectionHead: () => editorView.state.selection.main.head,
    getAnalyzerDiagnostics: () => lastDiagnostics,
    getCmDiagnostics: () => lastCmDiagnostics,
    getTokenDecorations: () => lastTokenRanges,
    getChipSpans: () => lastChipSpans,
    getChipUiRanges: () => lastChipUiRanges,
    toChipPos: (rawPos) => (lastChipMap ? lastChipMap.toChipPos(rawPos) : rawPos),
    toRawPos: (chipPos) => (lastChipMap ? lastChipMap.toRawPos(chipPos) : chipPos),
    setSelectionHead: (pos) => {
      editorView.dispatch({ selection: { anchor: pos } });
      editorView.focus();
    },
    isChipUiEnabled: () => true,
    getChipUiCount: () => lastChipUiRanges.length,
  });

  return {
    root: panel,
    mount(parent: HTMLElement) {
      parent.appendChild(panel);
    },
    update(state: FormulaState) {
      lastSource = state.source;
      lastDiagnostics = state.diagnostics;
      lastFormatted = state.formatted;
      lastOutputType = state.outputType;
      const outputTypeLabel = `output: ${state.outputType}`;
      outputTypeEl.textContent = outputTypeLabel;
      outputTypeEl.title = outputTypeLabel;

      const docLen = state.source.length;
      const sortedTokens = sortTokens(state.tokens || []);
      const tokenRanges = computeTokenDecorationRanges(docLen, sortedTokens);
      lastTokenRanges = tokenRanges;

      const tokenBuilder = new RangeSetBuilder<Decoration>();
      for (const range of tokenRanges) {
        tokenBuilder.add(range.from, range.to, Decoration.mark({ class: range.className }));
      }
      editorView.dispatch({ effects: setTokenDecoListEffect.of(tokenBuilder.finish()) });

      try {
        const chips = computePropChips(state.source, sortedTokens).filter(isValidPropChip);
        const chipRanges = chips
          .map((chip) => ({ from: chip.spanStart, to: chip.spanEnd, propName: chip.argValue }))
          .filter((range) => range.from >= 0 && range.to > range.from && range.to <= docLen);

        lastChipUiRanges = mergeChipRangesWithDiagnostics(chipRanges, state.diagnostics, docLen);
        lastChipSpans = chips
          .map((chip) => ({ start: chip.spanStart, end: chip.spanEnd }))
          .filter((span) => span.start >= 0 && span.end > span.start && span.end <= docLen)
          .sort((a, b) => a.start - b.start || a.end - b.end);
        editorView.dispatch({ effects: setChipDecoListEffect.of(lastChipUiRanges) });
      } catch {
        lastChipUiRanges = [];
        lastChipSpans = [];
        editorView.dispatch({ effects: setChipDecoListEffect.of([]) });
      }

      try {
        lastChipMap = buildChipOffsetMap(docLen, lastChipSpans);
      } catch {
        lastChipMap = null;
      }

      renderDiagnosticList(
        diagnosticsEl,
        buildDiagnosticTextRows(state.source, state.diagnostics, lastChipMap, lastChipSpans),
      );
      const cmDiagnostics = toCmDiagnostics(state.diagnostics, docLen, lastChipSpans);
      lastCmDiagnostics = cmDiagnostics;
      editorView.dispatch({ effects: setLintDiagnosticsEffect.of(cmDiagnostics) });
    },
  };
}
