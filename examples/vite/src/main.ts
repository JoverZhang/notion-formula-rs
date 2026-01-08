import "./style.css";
import init, { analyze } from "./pkg/analyzer_wasm.js";
import { RangeSetBuilder, EditorState } from "@codemirror/state";
import { linter } from "@codemirror/lint";
import { StateField, StateEffect } from "@codemirror/state";
import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import {
  Decoration,
  DecorationSet,
  EditorView,
  keymap,
} from "@codemirror/view";
import { defaultKeymap } from "@codemirror/commands";
import {
  computeTokenDecorationRanges,
  getTokenSpanIssues,
  setTokenDecosEffect,
  sortTokens,
  tokenDecoStateField,
} from "./editor_decorations";
import type { Token } from "./editor_decorations";
import { buildChipOffsetMap, computeChipSpans } from "./chip_spans";
import type { ChipOffsetMap } from "./chip_spans";

const DEFAULT_SOURCE = `if (
  prop( "feeling" ) == "😀",
    "I am absolutely not pretending.",
      "This is fine 🔥")
`;
const DEBOUNCE_MS = 80;

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

const PROPERTY_SCHEMA = [
  { name: "Title", type: "String" },
  { name: "feeling", type: "String" },
  { name: "Status", type: "String" },
  { name: "Due Date", type: "Date" },
] as const;
const CONTEXT_JSON = JSON.stringify({ properties: PROPERTY_SCHEMA });

function expectEl<T extends Element>(selector: string): T {
  const el = document.querySelector<T>(selector);
  if (!el) {
    throw new Error(`Missing element: ${selector}`);
  }
  return el;
}

const editorParentEl = expectEl<HTMLElement>("#editor");
const cursorInfoEl = expectEl<HTMLElement>("#cursor-info");
const warningEl = expectEl<HTMLElement>("#hl-warning");
const formattedEl = expectEl<HTMLElement>("#formatted");
const diagnosticsEl = expectEl<HTMLUListElement>("#diagnostics");

const setLintDiagnosticsEffect = StateEffect.define<CmDiagnostic[]>();
const lintDiagnosticsStateField = StateField.define<CmDiagnostic[]>({
  create() { return []; },
  update(value, tr) {
    for (const e of tr.effects) {
      if (e.is(setLintDiagnosticsEffect)) return e.value;
    }
    return value;
  },
});

type AnalyzerDiagnostic = Diagnostic; // 你现有的 type，建议直接改名

function toCmSeverity(kind?: string): "error" | "warning" | "info" {
  if (kind === "warning") return "warning";
  if (kind === "info") return "info";
  return "error";
}

function clamp(n: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, n));
}

function analyzerToCmDiagnostics(
  diags: AnalyzerDiagnostic[],
  docLen: number,
): CmDiagnostic[] {
  const out: CmDiagnostic[] = [];
  for (const d of diags) {
    const start = d.span?.start;
    const end = d.span?.end;
    if (typeof start !== "number") continue;

    const from = clamp(start, 0, docLen);
    // when end is missing, give a minimum range to avoid 0 length causing it to be invisible
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

function renderDiagnostics(diagnostics: Diagnostic[], chipMap?: ChipOffsetMap) {
  diagnosticsEl.innerHTML = "";
  if (!diagnostics || diagnostics.length === 0) {
    const li = document.createElement("li");
    li.textContent = "No diagnostics";
    diagnosticsEl.appendChild(li);
    return;
  }

  diagnostics.forEach((diag) => {
    const li = document.createElement("li");
    const kind = diag.kind || "error";
    const line = diag.span?.line ?? 0;
    const col = diag.span?.col ?? 0;
    const chipPos =
      chipMap && typeof diag.span?.start === "number"
        ? ` (chipPos=${chipMap.toChipPos(diag.span.start)})`
        : "";
    li.textContent = `${kind}: ${diag.message} @ ${line}:${col}${chipPos}`;
    diagnosticsEl.appendChild(li);
  });
}

function renderFormatted(formatted: string, diagnostics: Diagnostic[]) {
  if (!formatted && diagnostics && diagnostics.length > 0) {
    formattedEl.textContent = "(no formatted output due to fatal error)";
    return;
  }
  formattedEl.textContent = formatted || "";
}

function setWarningVisible(visible: boolean) {
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

function findTokenBefore(tokens: Token[], cursor: number): Token | null {
  let match: Token | null = null;
  for (const token of tokens) {
    if (!token || token.kind === "Eof") {
      continue;
    }
    const start = token.span?.start;
    const end = token.span?.end;
    if (typeof start !== "number" || typeof end !== "number") {
      continue;
    }
    if (end <= cursor || start < cursor) {
      match = token;
    }
  }
  return match;
}

function findTokenAfter(tokens: Token[], cursor: number): Token | null {
  for (const token of tokens) {
    if (!token || token.kind === "Eof") {
      continue;
    }
    const start = token.span?.start;
    const end = token.span?.end;
    if (typeof start !== "number" || typeof end !== "number") {
      continue;
    }
    if (start >= cursor || end > cursor) {
      return token;
    }
  }
  return null;
}

function renderCursorInfo(cursor: number, tokens: Token[]) {
  const before = findTokenBefore(tokens, cursor);
  const after = findTokenAfter(tokens, cursor);
  const beforeKind = before?.kind ?? "none";
  const afterKind = after?.kind ?? "none";
  cursorInfoEl.textContent = `Cursor: ${cursor} | before: ${beforeKind} | after: ${afterKind}`;
}

let editorView: EditorView | null = null;
let lastTokens: Token[] = [];
let lastSortedTokens: Token[] = [];

const runAnalyze = debounce((source: string) => {
  let result: AnalyzeResult | null = null;
  try {
    result = analyze(source, CONTEXT_JSON) as AnalyzeResult;
  } catch (error) {
    formattedEl.textContent = "(analysis failed)";
    diagnosticsEl.innerHTML = "";
    const li = document.createElement("li");
    li.textContent = "error: analyze() threw; see console";
    diagnosticsEl.appendChild(li);
    setWarningVisible(false);
    if (editorView) {
      editorView.dispatch({ effects: setTokenDecosEffect.of(Decoration.none) });
    }
    console.error(error);
    return;
  }

  if (!result) {
    formattedEl.textContent = "(no result)";
    diagnosticsEl.innerHTML = "";
    setWarningVisible(false);
    if (editorView) {
      editorView.dispatch({ effects: setTokenDecosEffect.of(Decoration.none) });
    }
    return;
  }

  const diagnostics = result.diagnostics || [];
  const chipSpans = computeChipSpans(source, result.tokens || []);
  let chipMap: ChipOffsetMap | undefined;
  try {
    chipMap = buildChipOffsetMap(source.length, chipSpans);
  } catch (error) {
    chipMap = undefined;
    console.warn("Failed to build chip offset map:", error);
  }
  renderDiagnostics(diagnostics, chipMap);
  renderFormatted(result.formatted || "", diagnostics);
  if (editorView) {
    const cmDiags = analyzerToCmDiagnostics(diagnostics, source.length);
    editorView.dispatch({ effects: setLintDiagnosticsEffect.of(cmDiags) });
  }
  lastTokens = result.tokens || [];
  lastSortedTokens = sortTokens(lastTokens);
  const { ok, decos } = buildTokenDecorations(source, lastSortedTokens);
  setWarningVisible(!ok);
  if (editorView) {
    editorView.dispatch({ effects: setTokenDecosEffect.of(ok ? decos : Decoration.none) });
    renderCursorInfo(editorView.state.selection.main.head, lastSortedTokens);
  }
}, DEBOUNCE_MS);

async function start() {
  let wasmReady = false;
  editorView = new EditorView({
    state: EditorState.create({
      doc: DEFAULT_SOURCE,
      extensions: [
        keymap.of(defaultKeymap),
        EditorView.lineWrapping,
        tokenDecoStateField,
        lintDiagnosticsStateField,
        EditorView.updateListener.of((update) => {
          if (update.docChanged && wasmReady) {
            runAnalyze(update.state.doc.toString());
          }
          if (update.selectionSet) {
            renderCursorInfo(update.state.selection.main.head, lastSortedTokens);
          }
        }),
        linter((view) => view.state.field(lintDiagnosticsStateField)),
      ],
    }),
    parent: editorParentEl,
  });

  renderCursorInfo(editorView.state.selection.main.head, lastSortedTokens);
  await init();
  wasmReady = true;
  runAnalyze(editorView.state.doc.toString());
}

start();
