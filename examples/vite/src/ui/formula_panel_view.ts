import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { linter } from "@codemirror/lint";
import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { EditorState, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import { Decoration, DecorationSet, EditorView, keymap } from "@codemirror/view";
import {
  completeSource,
  type CompletionItem,
  type SignatureHelp,
  posToLineCol,
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
    const hasWarning = false;
    let message: string | undefined;
    for (const diag of diagnostics) {
      const start = diag.span?.range?.start;
      if (typeof start !== "number") continue;
      const end = diag.span?.range?.end;
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
    const start = d.span?.range?.start;
    const end = d.span?.range?.end;
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
  const start = diag.span?.range?.start;
  const end = diag.span?.range?.end;
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
  source: string,
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
    const start = diag.span?.range?.start;
    let line = 0;
    let col = 0;
    if (typeof start === "number") {
      try {
        const lc = posToLineCol(source, start);
        line = lc.line;
        col = lc.col;
      } catch {
        line = 0;
        col = 0;
      }
    }
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
  let preferredCompletionIndices: number[] = [];
  type CompletionRenderRow =
    | { type: "header"; kind: CompletionItem["kind"]; label: string }
    | { type: "section"; section: "recommended"; label: string }
    | {
        type: "category";
        kind: "Function";
        category: NonNullable<CompletionItem["category"]>;
        label: string;
      }
    | {
        type: "item";
        item: CompletionItem;
        itemIndex: number;
        isRecommended?: boolean;
      };

  let completionRows: CompletionRenderRow[] = [];
  let selectedRowIndex = -1;
  let completionTimer: ReturnType<typeof setTimeout> | null = null;
  let statusTimer: ReturnType<typeof setTimeout> | null = null;
  const COMPLETION_DEBOUNCE_MS = 120;

  function completionGroupLabel(kind: CompletionItem["kind"]): string {
    switch (kind) {
      case "Function":
        return "Functions";
      case "Builtin":
        return "Built-ins";
      case "Property":
        return "Properties";
      case "Operator":
        return "Operators";
      default:
        return String(kind);
    }
  }

  function functionCategoryLabel(category: NonNullable<CompletionItem["category"]>): string {
    return `${category} Functions`;
  }

  function buildCompletionRows(
    items: CompletionItem[],
    preferredIndices: number[],
  ): CompletionRenderRow[] {
    const rows: CompletionRenderRow[] = [];
    const categoryOrder: Array<NonNullable<CompletionItem["category"]>> = [
      "General",
      "Text",
      "Number",
      "Date",
      "People",
      "List",
      "Special",
    ];

    const recommendedIndices: number[] = [];
    const recommendedSet = new Set<number>();
    for (const idx of preferredIndices) {
      if (!Number.isInteger(idx)) continue;
      if (idx < 0 || idx >= items.length) continue;
      if (recommendedSet.has(idx)) continue;
      const item = items[idx];
      if (!item || item.is_disabled) continue;
      recommendedSet.add(idx);
      recommendedIndices.push(idx);
    }

    if (recommendedIndices.length > 0) {
      rows.push({ type: "section", section: "recommended", label: "Recommended" });
      for (const itemIndex of recommendedIndices) {
        const item = items[itemIndex];
        if (!item) continue;
        rows.push({ type: "item", item, itemIndex, isRecommended: true });
      }
    }

    let scanIdx = 0;
    while (scanIdx < items.length) {
      const kind = items[scanIdx].kind;

      if (kind === "Function") {
        const functionItems: Array<{ item: CompletionItem; itemIndex: number }> = [];
        while (scanIdx < items.length && items[scanIdx].kind === "Function") {
          if (!recommendedSet.has(scanIdx)) {
            functionItems.push({ item: items[scanIdx], itemIndex: scanIdx });
          }
          scanIdx += 1;
        }

        if (functionItems.length === 0) {
          continue;
        }

        rows.push({ type: "header", kind, label: completionGroupLabel(kind) });

        const groups = new Map<NonNullable<CompletionItem["category"]>, typeof functionItems>();
        for (const category of categoryOrder) groups.set(category, []);

        for (const row of functionItems) {
          const category = (row.item.category ?? "General") as NonNullable<
            CompletionItem["category"]
          >;
          const group = groups.get(category);
          if (group) group.push(row);
        }

        for (const category of categoryOrder) {
          const group = groups.get(category);
          if (!group || group.length === 0) continue;
          rows.push({
            type: "category",
            kind: "Function",
            category,
            label: functionCategoryLabel(category),
          });
          group.forEach(({ item, itemIndex }) => rows.push({ type: "item", item, itemIndex }));
        }

        continue;
      }

      const segmentItems: Array<{ item: CompletionItem; itemIndex: number }> = [];
      while (scanIdx < items.length && items[scanIdx].kind === kind) {
        if (!recommendedSet.has(scanIdx)) {
          segmentItems.push({ item: items[scanIdx], itemIndex: scanIdx });
        }
        scanIdx += 1;
      }

      if (segmentItems.length === 0) {
        continue;
      }

      rows.push({ type: "header", kind, label: completionGroupLabel(kind) });
      segmentItems.forEach(({ item, itemIndex }) => rows.push({ type: "item", item, itemIndex }));
    }
    return rows;
  }

  function getSelectedItemIndex(): number | null {
    if (selectedRowIndex < 0 || selectedRowIndex >= completionRows.length) return null;
    const row = completionRows[selectedRowIndex];
    if (!row || row.type !== "item") return null;
    return row.itemIndex;
  }

  function normalizeSelectedRowIndex() {
    if (completionRows.length === 0) {
      selectedRowIndex = -1;
      return;
    }
    if (selectedRowIndex < 0 || selectedRowIndex >= completionRows.length) {
      selectedRowIndex = completionRows.findIndex((r) => r.type === "item");
      return;
    }
    if (completionRows[selectedRowIndex]?.type !== "item") {
      const forward = completionRows.findIndex(
        (r, idx) => idx > selectedRowIndex && r.type === "item",
      );
      if (forward !== -1) {
        selectedRowIndex = forward;
        return;
      }
      const backward = [...completionRows]
        .map((r, idx) => ({ r, idx }))
        .reverse()
        .find(({ r, idx }) => idx < selectedRowIndex && r.type === "item")?.idx;
      selectedRowIndex = typeof backward === "number" ? backward : -1;
    }
  }

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
    signatureEl.replaceChildren();
    if (sig.params.length === 0) {
      signatureEl.append(document.createTextNode(sig.label));
      return;
    }

    const openParen = sig.label.indexOf("(");
    const closeParen = sig.label.lastIndexOf(")");

    if (openParen === -1 || closeParen === -1 || closeParen <= openParen) {
      signatureEl.append(document.createTextNode(sig.label));
      return;
    }

    signatureEl.append(document.createTextNode(sig.label.slice(0, openParen + 1)));
    sig.params.forEach((param, idx) => {
      const paramEl = document.createElement("span");
      paramEl.className = "completion-signature-param";
      if (idx === sig.active_param) paramEl.classList.add("is-active");
      paramEl.textContent = param;
      signatureEl.append(paramEl);
      if (idx !== sig.params.length - 1) signatureEl.append(document.createTextNode(", "));
    });
    signatureEl.append(document.createTextNode(sig.label.slice(closeParen)));
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
        applyCompletionItem(row.itemIndex);
      });

      itemsEl.appendChild(li);
    });
  }

  function rerenderCompletions() {
    renderSignature(signatureHelp);
    completionRows = buildCompletionRows(completionItems, preferredCompletionIndices);
    const preferredTop = preferredCompletionIndices[0];
    if (typeof preferredTop === "number") {
      const rowIndex = completionRows.findIndex(
        (row) => row.type === "item" && row.itemIndex === preferredTop,
      );
      if (rowIndex !== -1) selectedRowIndex = rowIndex;
    }
    normalizeSelectedRowIndex();
    renderItems();
    scrollSelectedIntoView();
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
        preferredCompletionIndices = Array.isArray(output.preferred_indices)
          ? output.preferred_indices.filter((n) => typeof n === "number")
          : [];
      } catch {
        completionItems = [];
        signatureHelp = null;
        preferredCompletionIndices = [];
      }
      if (completionItems.length === 0) selectedRowIndex = -1;
      rerenderCompletions();
    }, COMPLETION_DEBOUNCE_MS);
  }

  function selectNext(delta: number) {
    if (!completionRows.length) return;
    const itemRowIndices = completionRows
      .map((r, idx) => (r.type === "item" ? idx : -1))
      .filter((idx) => idx !== -1);
    if (itemRowIndices.length === 0) return;

    if (selectedRowIndex < 0) {
      selectedRowIndex = delta > 0 ? itemRowIndices[0] : itemRowIndices[itemRowIndices.length - 1];
      renderItems();
      scrollSelectedIntoView();
      return;
    }

    const dir = delta >= 0 ? 1 : -1;
    let next = selectedRowIndex;
    for (let i = 0; i < completionRows.length; i++) {
      next = (next + dir + completionRows.length) % completionRows.length;
      if (completionRows[next]?.type === "item") {
        selectedRowIndex = next;
        renderItems();
        scrollSelectedIntoView();
        return;
      }
    }
  }

  function scrollSelectedIntoView() {
    const selectedItem = itemsEl.querySelector(".completion-item.is-selected");
    if (selectedItem instanceof HTMLElement) {
      selectedItem.scrollIntoView({ block: "nearest" });
    }
  }

  function applyCompletionItem(index: number): boolean {
    const item = completionItems[index];
    if (!item || item.is_disabled || !item.primary_edit) return false;

    const edits = [item.primary_edit, ...(item.additional_edits ?? [])];
    const changes = edits
      .map((e) => ({ from: e.range.start, to: e.range.end, insert: e.new_text }))
      .sort((a, b) => a.from - b.from || a.to - b.to);

    // Calculate the offset from additional edits before the primary edit
    let offset = 0;
    const primaryStart = item.primary_edit.range.start;
    for (const edit of item.additional_edits ?? []) {
      if (edit.range.end <= primaryStart) {
        offset += edit.new_text.length - (edit.range.end - edit.range.start);
      }
    }

    // Calculate the final cursor position considering the offset
    const fallbackCursor = primaryStart + item.primary_edit.new_text.length + offset;
    const newCursor = Math.max(0, item.cursor ?? fallbackCursor);

    editorView.dispatch({
      changes,
      selection: { anchor: newCursor },
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
              if (selectedRowIndex < 0) return false;
              selectedRowIndex = -1;
              renderItems();
              return true;
            },
          },
          {
            key: "Enter",
            run: () => {
              const itemIndex = getSelectedItemIndex();
              if (typeof itemIndex !== "number") return false;
              return applyCompletionItem(itemIndex);
            },
          },
          {
            key: "Tab",
            run: () => {
              const itemIndex = getSelectedItemIndex();
              if (typeof itemIndex !== "number") return false;
              return applyCompletionItem(itemIndex);
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

      renderDiagnostics(
        diagnosticsEl,
        state.source,
        state.diagnostics,
        lastChipMap,
        lastValidChipSpans,
      );
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
