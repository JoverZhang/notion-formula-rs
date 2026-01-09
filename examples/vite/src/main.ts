import "./style.css";
import init, * as wasm from "./pkg/analyzer_wasm.js";
import { EditorState, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import { linter } from "@codemirror/lint";
import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { Decoration, DecorationSet, EditorView, keymap } from "@codemirror/view";
import { defaultKeymap } from "@codemirror/commands";
import {
  computeTokenDecorationRanges,
  getTokenSpanIssues,
  setTokenDecosEffect,
  sortTokens,
  tokenDecoStateField,
} from "./editor_decorations";
import type { Token } from "./editor_decorations";

type Diagnostic = {
  kind?: string;
  message?: string;
  span?: {
    start?: number;
    end?: number;
    line?: number;
    col?: number;
  };
};

type AnalyzeResult = {
  diagnostics?: Diagnostic[];
  formatted?: string;
  tokens?: Token[];
};

type AnalyzerFn = (source: string, contextJson?: string) => AnalyzeResult;

type RelatedRow = {
  id: string;
  Text: string;
  Number: number;
  Select: "A" | "B" | "C";
};

type BaseRow = {
  id: string;
  Text: string;
  Number: number;
  Select: "A" | "B" | "C";
  Date: string;
  Relation: string[];
};

const PROPERTY_SCHEMA = [
  { name: "Text", type: "String" },
  { name: "Number", type: "Number" },
  { name: "Select", type: "String" },
  { name: "Date", type: "Date" },
  { name: "Relation", type: "Unknown" },
] as const;
const CONTEXT_JSON = JSON.stringify({ properties: PROPERTY_SCHEMA });
const DEBOUNCE_MS = 80;

const RELATED_TABLE: RelatedRow[] = [
  { id: "rel-1", Text: "North Star", Number: 3, Select: "A" },
  { id: "rel-2", Text: "Blueprint", Number: 8, Select: "B" },
  { id: "rel-3", Text: "Pulse", Number: 5, Select: "C" },
];

const BASE_ROWS: BaseRow[] = [
  {
    id: "row-1",
    Text: "Morning draft",
    Number: 12,
    Select: "A",
    Date: "2024-05-14",
    Relation: ["rel-1", "rel-2"],
  },
  {
    id: "row-2",
    Text: "Client check-in",
    Number: 7,
    Select: "B",
    Date: "2024-06-02",
    Relation: ["rel-3"],
  },
  {
    id: "row-3",
    Text: "QA pass",
    Number: 4,
    Select: "C",
    Date: "2024-06-09",
    Relation: ["rel-2"],
  },
  {
    id: "row-4",
    Text: "Wrap report",
    Number: 18,
    Select: "A",
    Date: "2024-06-16",
    Relation: ["rel-1", "rel-3"],
  },
];

const FORMULA_SAMPLES = [
  `if(prop("Number") > 10, prop("Text"), "Needs review")`,
  `formatDate(prop("Date"), "YYYY-MM-DD")`,
  `prop("Select") + " • " + prop("Text")`,
];

function expectEl<T extends Element>(selector: string): T {
  const el = document.querySelector<T>(selector);
  if (!el) {
    throw new Error(`Missing element: ${selector}`);
  }
  return el;
}

const appEl = expectEl<HTMLElement>("#app");

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

function analyzerToCmDiagnostics(
  diags: Diagnostic[],
  docLen: number,
): CmDiagnostic[] {
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

function debounce<T extends unknown[]>(fn: (...args: T) => void, delay: number) {
  let timer: ReturnType<typeof setTimeout> | null = null;
  return (...args: T) => {
    if (timer) {
      clearTimeout(timer);
    }
    timer = setTimeout(() => fn(...args), delay);
  };
}

function renderDiagnostics(listEl: HTMLUListElement, diagnostics: Diagnostic[]) {
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

function renderFormatted(formattedEl: HTMLElement, formatted: string, diagnostics: Diagnostic[]) {
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

function createTableHeader(labels: string[]): HTMLTableSectionElement {
  const thead = document.createElement("thead");
  const row = document.createElement("tr");
  labels.forEach((label) => {
    const th = document.createElement("th");
    th.textContent = label;
    row.appendChild(th);
  });
  thead.appendChild(row);
  return thead;
}

function createBaseTableSection(): HTMLElement {
  const section = document.createElement("section");
  section.className = "base-section pane";

  const title = document.createElement("h1");
  title.textContent = "Base Table";
  section.appendChild(title);

  const subtitle = document.createElement("p");
  subtitle.className = "section-subtitle";
  subtitle.textContent = "Read-only mock data with a relation into RelatedTable.";
  section.appendChild(subtitle);

  const grid = document.createElement("div");
  grid.className = "table-grid";

  const relatedMap = new Map(RELATED_TABLE.map((row) => [row.id, row]));

  const baseCard = document.createElement("div");
  baseCard.className = "table-card";
  const baseCardTitle = document.createElement("h2");
  baseCardTitle.textContent = "Tasks";
  baseCard.appendChild(baseCardTitle);

  const baseTable = document.createElement("table");
  baseTable.appendChild(createTableHeader(["Text", "Number", "Select", "Date", "Relation"]));
  const baseBody = document.createElement("tbody");

  BASE_ROWS.forEach((row) => {
    const tr = document.createElement("tr");

    [row.Text, row.Number.toString(), row.Select, row.Date].forEach((value) => {
      const td = document.createElement("td");
      td.textContent = value;
      tr.appendChild(td);
    });

    const relationTd = document.createElement("td");
    const pillWrap = document.createElement("div");
    pillWrap.className = "rel-pill-group";
    row.Relation.forEach((relId) => {
      const relRow = relatedMap.get(relId);
      const pill = document.createElement("span");
      pill.className = "rel-pill";
      pill.textContent = relRow ? relRow.Text : relId;
      pillWrap.appendChild(pill);
    });
    relationTd.appendChild(pillWrap);
    tr.appendChild(relationTd);

    baseBody.appendChild(tr);
  });

  baseTable.appendChild(baseBody);
  baseCard.appendChild(baseTable);

  const relatedCard = document.createElement("div");
  relatedCard.className = "table-card";
  const relatedTitle = document.createElement("h2");
  relatedTitle.textContent = "RelatedTable";
  relatedCard.appendChild(relatedTitle);

  const relatedTable = document.createElement("table");
  relatedTable.appendChild(createTableHeader(["Text", "Number", "Select"]));
  const relatedBody = document.createElement("tbody");
  RELATED_TABLE.forEach((row) => {
    const tr = document.createElement("tr");
    [row.Text, row.Number.toString(), row.Select].forEach((value) => {
      const td = document.createElement("td");
      td.textContent = value;
      tr.appendChild(td);
    });
    relatedBody.appendChild(tr);
  });
  relatedTable.appendChild(relatedBody);
  relatedCard.appendChild(relatedTable);

  grid.appendChild(baseCard);
  grid.appendChild(relatedCard);
  section.appendChild(grid);

  return section;
}

function resolveAnalyzeFn(): { fn: AnalyzerFn; passContext: boolean } | null {
  const analyze = (wasm as Record<string, unknown>).analyze;
  const analyzeWithContext = (wasm as Record<string, unknown>).analyzeWithContext;

  if (typeof analyze === "function") {
    return {
      fn: analyze as AnalyzerFn,
      passContext: typeof analyzeWithContext === "function",
    };
  }

  if (typeof analyzeWithContext === "function") {
    return {
      fn: ((source: string, contextJson?: string) =>
        (analyzeWithContext as AnalyzerFn)(source, contextJson)) as AnalyzerFn,
      passContext: true,
    };
  }

  return null;
}

type FormulaPanel = {
  editorView: EditorView;
  runAnalyze: (source: string) => void;
  setWasmReady: (ready: boolean) => void;
};

let analyzeFn: AnalyzerFn | null = null;
let shouldPassContext = false;

function createFormulaPanel(
  mountEl: HTMLElement,
  label: string,
  initialSource: string,
): FormulaPanel {
  let wasmReady = false;

  const panel = document.createElement("section");
  panel.className = "formula-panel pane";

  const leftCol = document.createElement("div");
  leftCol.className = "formula-left";

  const labelEl = document.createElement("div");
  labelEl.className = "formula-label";
  labelEl.textContent = label;
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
  mountEl.appendChild(panel);

  const runAnalyze = debounce((source: string) => {
    if (!wasmReady || !analyzeFn) {
      return;
    }

    let result: AnalyzeResult | null = null;
    try {
      result = shouldPassContext
        ? analyzeFn(source, CONTEXT_JSON)
        : analyzeFn(source);
    } catch (error) {
      formattedEl.textContent = "(analysis failed)";
      renderDiagnostics(diagnosticsEl, [
        { kind: "error", message: "analyze() threw; see console" },
      ]);
      setWarningVisible(warningEl, false);
      if (editorView) {
        editorView.dispatch({ effects: setTokenDecosEffect.of(Decoration.none) });
        editorView.dispatch({ effects: setLintDiagnosticsEffect.of([]) });
      }
      console.error(error);
      return;
    }

    if (!result) {
      formattedEl.textContent = "(no result)";
      renderDiagnostics(diagnosticsEl, []);
      setWarningVisible(warningEl, false);
      if (editorView) {
        editorView.dispatch({ effects: setTokenDecosEffect.of(Decoration.none) });
        editorView.dispatch({ effects: setLintDiagnosticsEffect.of([]) });
      }
      return;
    }

    const diagnostics = result.diagnostics || [];
    renderDiagnostics(diagnosticsEl, diagnostics);
    renderFormatted(formattedEl, result.formatted || "", diagnostics);

    if (editorView) {
      const cmDiags = analyzerToCmDiagnostics(diagnostics, source.length);
      editorView.dispatch({ effects: setLintDiagnosticsEffect.of(cmDiags) });
    }

    const sortedTokens = sortTokens(result.tokens || []);
    const { ok, decos } = buildTokenDecorations(source, sortedTokens);
    setWarningVisible(warningEl, !ok);
    if (editorView) {
      editorView.dispatch({ effects: setTokenDecosEffect.of(ok ? decos : Decoration.none) });
    }
  }, DEBOUNCE_MS);

  const editorView = new EditorView({
    state: EditorState.create({
      doc: initialSource,
      extensions: [
        keymap.of(defaultKeymap),
        EditorView.lineWrapping,
        tokenDecoStateField,
        lintDiagnosticsStateField,
        EditorView.updateListener.of((update) => {
          if (update.docChanged && wasmReady) {
            runAnalyze(update.state.doc.toString());
          }
        }),
        linter((view) => view.state.field(lintDiagnosticsStateField)),
      ],
    }),
    parent: editorEl,
  });

  renderDiagnostics(diagnosticsEl, []);

  return {
    editorView,
    runAnalyze: (source: string) => runAnalyze(source),
    setWasmReady: (ready: boolean) => {
      wasmReady = ready;
    },
  };
}

async function start() {
  const layout = document.createElement("div");
  layout.className = "layout";

  const baseSection = createBaseTableSection();
  layout.appendChild(baseSection);

  const divider = document.createElement("hr");
  divider.className = "divider";
  layout.appendChild(divider);

  const formulaSection = document.createElement("section");
  formulaSection.className = "formula-section";
  layout.appendChild(formulaSection);

  const panels = FORMULA_SAMPLES.map((sample, index) =>
    createFormulaPanel(formulaSection, `Formula ${index + 1}`, sample),
  );

  appEl.appendChild(layout);

  await init();
  const resolved = resolveAnalyzeFn();
  if (!resolved) {
    console.error("Analyzer exports not found.");
    return;
  }

  analyzeFn = resolved.fn;
  shouldPassContext = resolved.passContext;

  panels.forEach((panel) => {
    panel.setWasmReady(true);
    panel.runAnalyze(panel.editorView.state.doc.toString());
  });
}

start();
