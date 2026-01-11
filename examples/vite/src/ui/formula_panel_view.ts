import { defaultKeymap } from "@codemirror/commands";
import { linter } from "@codemirror/lint";
import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { EditorState, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import { Decoration, DecorationSet, EditorView, keymap } from "@codemirror/view";
import type { FormulaId, FormulaState, AnalyzerDiagnostic } from "../app/types";
import {
  computeTokenDecorationRanges,
  getTokenSpanIssues,
  setTokenDecosEffect,
  sortTokens,
  tokenDecoStateField,
} from "../editor_decorations";
import type { Token } from "../editor_decorations";

type FormulaPanelView = {
  root: HTMLElement;
  mount(parent: HTMLElement): void;
  update(state: FormulaState): void;
};

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

function analyzerToCmDiagnostics(diags: AnalyzerDiagnostic[], docLen: number): CmDiagnostic[] {
  const out: CmDiagnostic[] = [];
  for (const d of diags) {
    const start = d.span?.start;
    const end = d.span?.end;
    if (typeof start !== "number") continue;

    const from = clamp(start, 0, docLen);
    const toRaw = typeof end === "number" ? end : start + 1;
    const to = clamp(Math.max(toRaw, from + 1), 0, docLen);

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
): { ok: boolean; decos: DecorationSet } {
  if (!tokens || tokens.length === 0) {
    return { ok: true, decos: Decoration.none };
  }

  const builder = new RangeSetBuilder<Decoration>();
  const docLen = source.length;
  const ranges = computeTokenDecorationRanges(docLen, tokens);
  for (const range of ranges) {
    builder.add(range.from, range.to, Decoration.mark({ class: range.className }));
  }

  const issues = getTokenSpanIssues(docLen, tokens);
  const ok = !issues.outOfBounds && !issues.overlap;
  return { ok, decos: builder.finish() };
}

export function createFormulaPanelView(opts: {
  id: FormulaId;
  label: string;
  initialSource: string;
  onSourceChange: (id: FormulaId, source: string) => void;
}): FormulaPanelView {
  const panel = document.createElement("section");
  panel.className = "formula-panel pane";

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
  leftCol.appendChild(editorEl);

  const diagTitle = document.createElement("div");
  diagTitle.className = "diagnostics-title";
  diagTitle.textContent = "Diagnostics";
  leftCol.appendChild(diagTitle);

  const diagnosticsEl = document.createElement("ul");
  diagnosticsEl.className = "diag-list";
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
  rightCol.appendChild(formattedEl);

  panel.appendChild(leftCol);
  panel.appendChild(rightCol);

  const editorView = new EditorView({
    state: EditorState.create({
      doc: opts.initialSource,
      extensions: [
        keymap.of(defaultKeymap),
        EditorView.lineWrapping,
        tokenDecoStateField,
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

  renderDiagnostics(diagnosticsEl, []);

  return {
    root: panel,
    mount(parent: HTMLElement) {
      parent.appendChild(panel);
    },
    update(state: FormulaState) {
      renderDiagnostics(diagnosticsEl, state.diagnostics);
      renderFormatted(formattedEl, state.formatted, state.diagnostics);

      const cmDiags = analyzerToCmDiagnostics(state.diagnostics, state.source.length);
      editorView.dispatch({ effects: setLintDiagnosticsEffect.of(cmDiags) });

      const sortedTokens = sortTokens(state.tokens || []);
      const { ok, decos } = buildTokenDecorations(state.source, sortedTokens);
      setWarningVisible(warningEl, !ok);
      editorView.dispatch({ effects: setTokenDecosEffect.of(ok ? decos : Decoration.none) });
    },
  };
}
