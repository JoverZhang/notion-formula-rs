import { defaultKeymap } from "@codemirror/commands";
import { linter } from "@codemirror/lint";
import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { EditorState, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import { Decoration, DecorationSet, EditorView, keymap } from "@codemirror/view";
import { PROPERTY_SCHEMA } from "../app/context";
import type { FormulaId, FormulaState, AnalyzerDiagnostic } from "../app/types";
import {
  buildChipOffsetMap,
  computeChipSpans,
  type ChipOffsetMap,
  type ChipSpan,
} from "../chip_spans";
import { registerPanelDebug } from "../debug/debug_bridge";
import {
  chipDecoStateField,
  formulaIdFacet,
  setChipDecosEffect,
  type ChipDecorationRange,
} from "../editor/chip_decorations";
import {
  computePropChips,
  computeTokenDecorationRanges,
  getTokenSpanIssues,
  setTokenDecosEffect,
  sortTokens,
  type TokenDecorationRange,
  tokenDecoStateField,
} from "../editor_decorations";
import type { Token } from "../editor_decorations";

type FormulaPanelView = {
  root: HTMLElement;
  mount(parent: HTMLElement): void;
  update(state: FormulaState): void;
};

const VALID_PROP_NAMES = new Set(PROPERTY_SCHEMA.map((prop) => prop.name));

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

function renderDiagnostics(listEl: HTMLUListElement, diagnostics: AnalyzerDiagnostic[]) {
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
    li.textContent = `${kind}: ${diag.message} @ ${line}:${col}`;
    listEl.appendChild(li);
  });
}

function renderFormatted(
  formattedEl: HTMLElement,
  formatted: string,
  diagnostics: AnalyzerDiagnostic[],
) {
  if (!formatted && diagnostics && diagnostics.length > 0) {
    formattedEl.textContent = "(no formatted output due to fatal error)";
    return;
  }
  formattedEl.textContent = formatted || "";
}

function setWarningVisible(warningEl: HTMLElement, visible: boolean) {
  warningEl.classList.toggle("hidden", !visible);
}

function buildTokenDecorations(
  source: string,
  tokens: Token[],
): { ok: boolean; decos: DecorationSet; ranges: TokenDecorationRange[] } {
  if (!tokens || tokens.length === 0) {
    return { ok: true, decos: Decoration.none, ranges: [] };
  }

  const builder = new RangeSetBuilder<Decoration>();
  const docLen = source.length;
  const ranges = computeTokenDecorationRanges(docLen, tokens);
  for (const range of ranges) {
    builder.add(range.from, range.to, Decoration.mark({ class: range.className }));
  }

  const issues = getTokenSpanIssues(docLen, tokens);
  const ok = !issues.outOfBounds && !issues.overlap;
  return { ok, decos: builder.finish(), ranges };
}

export function createFormulaPanelView(opts: {
  id: FormulaId;
  label: string;
  initialSource: string;
  onSourceChange: (id: FormulaId, source: string) => void;
}): FormulaPanelView {
  const panel = document.createElement("section");
  panel.className = "formula-panel pane";
  panel.setAttribute("data-testid", "formula-panel");
  panel.setAttribute("data-formula-id", opts.id);

  const leftCol = document.createElement("div");
  leftCol.className = "formula-left";

  const labelEl = document.createElement("div");
  labelEl.className = "formula-label";
  labelEl.textContent = opts.label;
  leftCol.appendChild(labelEl);

  const warningEl = document.createElement("div");
  warningEl.className = "warning hidden";
  warningEl.textContent = "Token spans overlap or exceed the document length.";
  leftCol.appendChild(warningEl);

  const editorEl = document.createElement("div");
  editorEl.className = "editor";
  editorEl.setAttribute("data-testid", "formula-editor");
  editorEl.setAttribute("data-formula-id", opts.id);
  leftCol.appendChild(editorEl);

  const diagTitle = document.createElement("div");
  diagTitle.className = "diagnostics-title";
  diagTitle.textContent = "Diagnostics";
  leftCol.appendChild(diagTitle);

  const diagnosticsEl = document.createElement("ul");
  diagnosticsEl.className = "diag-list";
  diagnosticsEl.setAttribute("data-testid", "formula-diagnostics");
  diagnosticsEl.setAttribute("data-formula-id", opts.id);
  leftCol.appendChild(diagnosticsEl);

  const rightCol = document.createElement("div");
  rightCol.className = "result-panel";

  const resultTitle = document.createElement("div");
  resultTitle.className = "result-title";
  resultTitle.textContent = "Result";
  rightCol.appendChild(resultTitle);

  const resultPlaceholder = document.createElement("div");
  resultPlaceholder.className = "result-placeholder";
  resultPlaceholder.textContent = "Evaluator not implemented yet";
  rightCol.appendChild(resultPlaceholder);

  const formattedLabel = document.createElement("div");
  formattedLabel.className = "result-subtitle";
  formattedLabel.textContent = "Formatted";
  rightCol.appendChild(formattedLabel);

  const formattedEl = document.createElement("pre");
  formattedEl.className = "result-formatted";
  formattedEl.setAttribute("data-testid", "formula-formatted");
  formattedEl.setAttribute("data-formula-id", opts.id);
  rightCol.appendChild(formattedEl);

  panel.appendChild(leftCol);
  panel.appendChild(rightCol);

  const editorView = new EditorView({
    state: EditorState.create({
      doc: opts.initialSource,
      extensions: [
        keymap.of(defaultKeymap),
        EditorView.lineWrapping,
        formulaIdFacet.of(opts.id),
        tokenDecoStateField,
        chipDecoStateField,
        lintDiagnosticsStateField,
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            opts.onSourceChange(opts.id, update.state.doc.toString());
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
  let lastChipSpans: ChipSpan[] = [];
  let lastChipUiRanges: ChipDecorationRange[] = [];
  let lastValidChipSpans: ChipSpan[] = [];
  let lastChipMap: ChipOffsetMap | null = null;
  let lastFormatted = "";
  let lastStatus: FormulaState["status"] = "idle";
  let lastSource = opts.initialSource;

  renderDiagnostics(diagnosticsEl, []);

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
    getChipSpans: () => lastChipSpans,
    toChipPos: (rawPos) => (lastChipMap ? lastChipMap.toChipPos(rawPos) : rawPos),
    toRawPos: (chipPos) => (lastChipMap ? lastChipMap.toRawPos(chipPos) : chipPos),
    // Future chip UI must reflect actual chip widgets/decorations here.
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
      lastStatus = state.status;
      lastDiagnostics = state.diagnostics;
      lastFormatted = state.formatted;

      renderDiagnostics(diagnosticsEl, state.diagnostics);
      renderFormatted(formattedEl, state.formatted, state.diagnostics);

      const sortedTokens = sortTokens(state.tokens || []);
      const { ok, decos, ranges } = buildTokenDecorations(state.source, sortedTokens);
      lastTokenRanges = ranges;
      setWarningVisible(warningEl, !ok);
      editorView.dispatch({ effects: setTokenDecosEffect.of(ok ? decos : Decoration.none) });

      try {
        const docLen = state.source.length;
        const chips = computePropChips(state.source, sortedTokens);
        const validChips = chips.filter((chip) => VALID_PROP_NAMES.has(chip.argValue));
        lastChipUiRanges = validChips
          .map((chip) => ({
            from: chip.spanStart,
            to: chip.spanEnd,
            propName: chip.argValue,
          }))
          .filter((range) => range.from >= 0 && range.to > range.from && range.to <= docLen);
        lastValidChipSpans = validChips
          .map((chip) => ({ start: chip.spanStart, end: chip.spanEnd }))
          .filter((span) => span.start >= 0 && span.end > span.start && span.end <= docLen)
          .sort((a, b) => a.start - b.start || a.end - b.end);
        editorView.dispatch({ effects: setChipDecosEffect.of(lastChipUiRanges) });

        lastChipSpans = computeChipSpans(state.source, sortedTokens);
        lastChipMap = buildChipOffsetMap(state.source.length, lastChipSpans);
      } catch {
        lastChipSpans = [];
        lastChipUiRanges = [];
        lastValidChipSpans = [];
        lastChipMap = null;
        editorView.dispatch({ effects: setChipDecosEffect.of([]) });
      }

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
