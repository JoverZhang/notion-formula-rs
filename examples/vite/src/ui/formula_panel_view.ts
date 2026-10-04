import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { linter, type Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { EditorState, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView, keymap } from "@codemirror/view";
import { PROPERTY_SCHEMA } from "../app/context";
import {
  FORMULA_IDS,
  type FormulaDiagnostic,
  type FormulaEditorActions,
  type FormulaId,
  type FormulaState,
} from "../app/types";
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
import type {
  CompletionItem,
  DraftVersion,
  QuickFix,
  SignatureHelp,
  UpdateExpressionResult,
} from "../formula/client";
import {
  buildCompletionRows,
  createCompletionEdit,
  getCompletionCursor,
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
import { formatValueType } from "../model/values";
import { createSignaturePopover } from "./signature_popover";

type FormulaPanelView = {
  root: HTMLElement;
  mount(parent: HTMLElement): void;
  update(state: FormulaState, saving?: boolean): void;
  dispose(): void;
};

const VALID_PROP_NAMES = new Set<string>([
  ...PROPERTY_SCHEMA.map((prop) => prop.id),
  ...FORMULA_IDS,
]);
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

function clearActiveFormulaPanel(id: FormulaId) {
  if (activeFormulaPanelId !== id) return;
  activeFormulaPanelUiById.get(id)?.hide();
  activeFormulaPanelId = null;
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

function isValidPropChip(chip: Chip): boolean {
  return VALID_PROP_NAMES.has(chip.argValue);
}

export function createFormulaPanelView(opts: {
  id: FormulaId;
  label: string;
  initialSource: string;
  actions: FormulaEditorActions;
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
          <button class="quick-fix-button" type="button" data-testid="quick-fix-button" data-formula-id="${opts.id}" disabled>Quick Fix</button>
          <button class="save-button" type="button" data-testid="save-button" data-formula-id="${opts.id}" disabled>Save</button>
          <button class="discard-button" type="button" data-testid="discard-button" data-formula-id="${opts.id}" disabled>Discard</button>
          <span class="formula-dirty hidden" data-testid="formula-dirty" data-formula-id="${opts.id}">Unsaved changes</span>
          <div class="formula-output-type" data-testid="formula-output-type" data-formula-id="${opts.id}">
            <span class="formula-output-type-value"></span>
          </div>
        </div>
        <div class="formula-error hidden" role="alert" data-testid="formula-error" data-formula-id="${opts.id}"></div>
        <div class="completion-panel hidden" data-testid="completion-panel" data-formula-id="${opts.id}">
          <div class="completion-header">Completions</div>
          <div class="completion-body">
            <ul class="completion-items"></ul>
            <div class="completion-empty">No suggestions</div>
          </div>
        </div>
      </div>
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
  const quickFixBtn = must<HTMLButtonElement>(panel, ".quick-fix-button");
  const saveBtn = must<HTMLButtonElement>(panel, ".save-button");
  const discardBtn = must<HTMLButtonElement>(panel, ".discard-button");
  const dirtyEl = must<HTMLElement>(panel, ".formula-dirty");
  const errorEl = must<HTMLElement>(panel, ".formula-error");
  const outputTypeEl = must<HTMLElement>(panel, ".formula-output-type");
  const outputTypeValueEl = must<HTMLElement>(panel, ".formula-output-type-value");
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
  let diagnosticRows: string[] = ["No diagnostics"];
  let preferredCompletionIndices: number[] = [];
  let completionRows: CompletionRenderRow[] = [];
  let selectedRowIndex = -1;
  let completionTimer: ReturnType<typeof setTimeout> | null = null;
  let helpGeneration = 0;
  let quickFixGeneration = 0;
  let helpContext: { source: string; cursor: number; version: DraftVersion } | null = null;
  let disposed = false;
  let suppressSourceChange = false;
  let inSourceChange = false;
  let queuedUpdate: { state: FormulaState; saving: boolean } | null = null;
  let updateQueued = false;
  let userRevision = 0;
  let blurRaf: number | null = null;
  let lastDiagnostics: FormulaDiagnostic[] = [];
  let lastCmDiagnostics: CmDiagnostic[] = [];
  let lastTokenRanges: TokenDecorationRange[] = [];
  let lastChipUiRanges: ChipDecorationRange[] = [];
  let lastChipSpans: ChipSpan[] = [];
  let lastChipMap: ChipOffsetMap | null = null;
  let lastQuickFixAction: QuickFix | null = null;
  let lastOutputType = "unknown";
  let lastSource = opts.initialSource;
  let lastVersion: DraftVersion | null = null;
  let lastStateCursor: number | null = null;
  let lastDirty = false;
  let lastSaving = false;
  let commandPending = false;
  let commandRevision: number | null = null;
  let quickFixKey = "";

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
    signaturePopover.render(signatureHelp, diagnosticRows, isUiActive);

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
    completionTimer = null;
    const generation = ++helpGeneration;
    const source = view.state.doc.toString();
    const cursor = view.state.selection.main.head;
    helpContext = null;
    completionItems = [];
    signatureHelp = null;
    preferredCompletionIndices = [];
    selectedRowIndex = -1;
    rerenderCompletions();
    if (disposed || !isUiActive || lastSaving) return;

    const isCurrent = () =>
      !disposed &&
      isUiActive &&
      !lastSaving &&
      generation === helpGeneration &&
      view.state.doc.toString() === source &&
      view.state.selection.main.head === cursor;
    completionTimer = setTimeout(() => {
      completionTimer = null;
      if (!isCurrent()) return;
      void opts.actions.help(opts.id, cursor).then(
        (next) => {
          if (!next || !isCurrent()) return;
          helpContext = { source, cursor, version: next.base_version };
          completionItems = next.completion.items;
          signatureHelp = next.signature_help;
          preferredCompletionIndices = next.completion.preferred_indices;
          rerenderCompletions();
        },
        () => {},
      );
    }, COMPLETION_DEBOUNCE_MS);
  }

  function applyNativeSelection(
    result: UpdateExpressionResult | null,
    revision: number,
    cursor?: number,
  ) {
    if (
      disposed ||
      !result ||
      revision !== userRevision ||
      result.state.definition.expression !== editorView.state.doc.toString()
    )
      return;
    const nextCursor = cursor ?? result.cursor;
    suppressSourceChange = true;
    try {
      editorView.dispatch({
        selection: { anchor: Math.max(0, Math.min(nextCursor, editorView.state.doc.length)) },
      });
    } finally {
      suppressSourceChange = false;
    }
    if (activeFormulaPanelId === opts.id || panel.contains(document.activeElement)) {
      editorView.focus();
    }
    requestCompletions(editorView);
  }

  function refreshButtons() {
    formatBtn.disabled = lastSaving || commandPending || lastVersion === null;
    quickFixBtn.disabled = lastSaving || commandPending || !lastQuickFixAction;
    saveBtn.disabled = lastSaving || !lastDirty || lastVersion === null;
    discardBtn.disabled = lastSaving || !lastDirty || lastVersion === null;
    saveBtn.textContent = lastSaving ? "Saving…" : "Save";
    quickFixBtn.title = lastQuickFixAction?.title ?? "No quick fix available";
  }

  function runEditorCommand(
    operation: () => Promise<UpdateExpressionResult | null>,
    item?: CompletionItem,
  ) {
    const revision = userRevision;
    commandPending = true;
    commandRevision = revision;
    refreshButtons();
    void operation().then(
      (result) => {
        commandPending = false;
        commandRevision = null;
        if (disposed) return;
        refreshButtons();
        applyNativeSelection(
          result,
          revision,
          item && result ? getCompletionCursor(item, result.cursor) : undefined,
        );
      },
      (error: unknown) => {
        commandPending = false;
        commandRevision = null;
        if (disposed) return;
        refreshButtons();
        showError(error);
      },
    );
  }

  function showError(error: unknown) {
    if (disposed) return;
    errorEl.textContent =
      error instanceof Error ? error.message : "The operation could not finish.";
    errorEl.classList.remove("hidden");
  }

  function requestQuickFixes(state: FormulaState) {
    if (lastSaving) return;
    const key = `${state.version ?? "none"}:${JSON.stringify(state.diagnostics.map((diag) => diag.id))}`;
    if (key === quickFixKey) return;
    quickFixKey = key;
    const generation = ++quickFixGeneration;
    lastQuickFixAction = null;
    refreshButtons();
    if (state.version === null || !state.diagnostics.length) return;
    const source = state.source;
    const version = state.version;
    const isCurrent = () =>
      !disposed &&
      generation === quickFixGeneration &&
      lastVersion === version &&
      editorView.state.doc.toString() === source;
    const findFirst = async () => {
      for (const diagnostic of state.diagnostics) {
        if (!isCurrent()) return;
        const fixes = await opts.actions.quickFixes(opts.id, diagnostic.id);
        if (!isCurrent()) return;
        const first = fixes[0];
        if (!first) continue;
        lastQuickFixAction = first;
        refreshButtons();
        return;
      }
    };
    void findFirst().catch(() => {});
  }

  function applySelectedCompletion(index: number): boolean {
    const item = completionItems[index];
    const context = helpContext;
    if (
      disposed ||
      !item ||
      !context ||
      commandPending ||
      lastSaving ||
      context.source !== editorView.state.doc.toString() ||
      context.cursor !== editorView.state.selection.main.head
    )
      return false;
    const edit = createCompletionEdit(item, context.version);
    if (!edit) return false;
    selectedRowIndex = -1;
    runEditorCommand(() => opts.actions.applyEdit(opts.id, edit, context.cursor), item);
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
          if (disposed || suppressSourceChange) return;
          if (update.docChanged || update.selectionSet) userRevision += 1;
          if (update.docChanged) {
            quickFixGeneration += 1;
            quickFixKey = "";
            lastQuickFixAction = null;
            refreshButtons();
            inSourceChange = true;
            try {
              opts.actions.setSource(opts.id, update.state.doc.toString());
            } finally {
              inSourceChange = false;
            }
          }
          if (update.docChanged || update.selectionSet) requestCompletions(update.view);
        }),
        linter((view) => view.state.field(lintDiagnosticsStateField)),
      ],
    }),
    parent: editorEl,
  });

  const activeUi: ActiveFormulaPanelUi = {
    show() {
      isUiActive = true;
      completionPanel.classList.remove("hidden");
      requestCompletions(editorView);
      rerenderCompletions();
    },
    hide() {
      isUiActive = false;
      helpGeneration += 1;
      if (completionTimer) clearTimeout(completionTimer);
      completionTimer = null;
      completionPanel.classList.add("hidden");
      signaturePopover.hide();
    },
  };
  activeFormulaPanelUiById.set(opts.id, activeUi);

  const onFocusIn = () => {
    if (activeFormulaPanelId === opts.id) requestCompletions(editorView);
    else setActiveFormulaPanel(opts.id);
  };

  const onFocusOut = () => {
    helpGeneration += 1;
    if (completionTimer) clearTimeout(completionTimer);
    completionTimer = null;
    signaturePopover.hide();
    if (blurRaf !== null) cancelAnimationFrame(blurRaf);
    blurRaf = requestAnimationFrame(() => {
      blurRaf = null;
      if (disposed || editorView.hasFocus) return;
      clearActiveFormulaPanel(opts.id);
    });
  };

  const onResize = () => {
    if (!isUiActive) return;
    signaturePopover.updateSide();
  };
  editorView.dom.addEventListener("focusin", onFocusIn);
  editorView.dom.addEventListener("focusout", onFocusOut);
  window.addEventListener("resize", onResize);

  rerenderCompletions();

  const onFormat = () => {
    if (formatBtn.disabled || disposed) return;
    const cursor = editorView.state.selection.main.head;
    runEditorCommand(() => opts.actions.format(opts.id, cursor));
  };

  const onQuickFix = () => {
    if (quickFixBtn.disabled || disposed) return;
    const action = lastQuickFixAction;
    if (!action) return;
    const cursor = editorView.state.selection.main.head;
    runEditorCommand(() => opts.actions.applyEdit(opts.id, action.edit, cursor));
  };
  const onSave = () => {
    if (saveBtn.disabled || disposed) return;
    void opts.actions.save(opts.id).catch(showError);
  };
  const onDiscard = () => {
    if (discardBtn.disabled || disposed) return;
    void opts.actions.discard(opts.id).catch(showError);
  };
  formatBtn.addEventListener("click", onFormat);
  quickFixBtn.addEventListener("click", onQuickFix);
  saveBtn.addEventListener("click", onSave);
  discardBtn.addEventListener("click", onDiscard);

  const unregisterDebug = registerPanelDebug(opts.id, {
    getState: () => ({
      source: lastSource,
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

  const result: FormulaPanelView = {
    root: panel,
    mount(parent: HTMLElement) {
      parent.appendChild(panel);
    },
    update(state: FormulaState, saving = false) {
      if (disposed) return;
      // setSource may notify synchronously from inside CodeMirror's update listener.
      if (inSourceChange) {
        queuedUpdate = { state, saving };
        if (!updateQueued) {
          updateQueued = true;
          queueMicrotask(() => {
            updateQueued = false;
            const pending = queuedUpdate;
            queuedUpdate = null;
            if (pending && !disposed) result.update(pending.state, pending.saving);
          });
        }
        return;
      }
      queuedUpdate = null;
      const sourceChanged = state.source !== editorView.state.doc.toString();
      const versionChanged = state.version !== lastVersion;
      const cursorChanged = state.cursor !== lastStateCursor;
      // AppVM publishes native state before the command Promise resolves.
      const stateCursorCurrent =
        state.cursor !== null && (commandRevision === null || commandRevision === userRevision);
      const updateCursor = cursorChanged && stateCursorCurrent;
      const savingChanged = saving !== lastSaving;
      lastSource = state.source;
      lastDiagnostics = state.diagnostics;
      lastVersion = state.version;
      lastStateCursor = state.cursor;
      lastDirty = state.dirty;
      lastSaving = saving;
      if (savingChanged) {
        quickFixGeneration += 1;
        quickFixKey = "";
        lastQuickFixAction = null;
      }
      lastOutputType = formatValueType(state.outputType);
      dirtyEl.classList.toggle("hidden", !state.dirty);
      errorEl.textContent = state.error ?? "";
      errorEl.classList.toggle("hidden", !state.error);
      refreshButtons();
      const outputTypeLabel = `output: ${lastOutputType}`;
      outputTypeValueEl.textContent = outputTypeLabel;
      outputTypeEl.title = outputTypeLabel;

      if (sourceChanged || updateCursor) {
        const selection = editorView.state.selection.main;
        const anchor = stateCursorCurrent ? state.cursor! : selection.anchor;
        const head = stateCursorCurrent ? state.cursor! : selection.head;
        suppressSourceChange = true;
        try {
          editorView.dispatch({
            changes: sourceChanged
              ? { from: 0, to: editorView.state.doc.length, insert: state.source }
              : undefined,
            selection: {
              anchor: Math.max(0, Math.min(anchor, state.source.length)),
              head: Math.max(0, Math.min(head, state.source.length)),
            },
          });
        } finally {
          suppressSourceChange = false;
        }
      }

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

      diagnosticRows = buildDiagnosticTextRows(
        state.source,
        state.diagnostics,
        lastChipMap,
        lastChipSpans,
      );
      signaturePopover.render(signatureHelp, diagnosticRows, isUiActive);
      const cmDiagnostics = toCmDiagnostics(state.diagnostics, docLen, lastChipSpans);
      lastCmDiagnostics = cmDiagnostics;
      editorView.dispatch({ effects: setLintDiagnosticsEffect.of(cmDiagnostics) });
      requestQuickFixes(state);
      if (sourceChanged || versionChanged || savingChanged || updateCursor) {
        requestCompletions(editorView);
      }
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      helpGeneration += 1;
      quickFixGeneration += 1;
      if (completionTimer) clearTimeout(completionTimer);
      if (blurRaf !== null) cancelAnimationFrame(blurRaf);
      if (activeFormulaPanelUiById.get(opts.id) === activeUi) {
        clearActiveFormulaPanel(opts.id);
        activeFormulaPanelUiById.delete(opts.id);
      }
      editorView.dom.removeEventListener("focusin", onFocusIn);
      editorView.dom.removeEventListener("focusout", onFocusOut);
      window.removeEventListener("resize", onResize);
      formatBtn.removeEventListener("click", onFormat);
      quickFixBtn.removeEventListener("click", onQuickFix);
      saveBtn.removeEventListener("click", onSave);
      discardBtn.removeEventListener("click", onDiscard);
      unregisterDebug();
      signaturePopover.dispose();
      editorView.destroy();
      panel.remove();
    },
  };
  return result;
}
