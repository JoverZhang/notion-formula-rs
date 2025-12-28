import "./style.css";
import init, { analyze } from "./pkg/analyzer_wasm.js";
import { RangeSetBuilder, EditorState } from "@codemirror/state";
import {
  Decoration,
  DecorationSet,
  EditorView,
  keymap,
} from "@codemirror/view";
import { defaultKeymap } from "@codemirror/commands";
import {
  computePropChips,
  computeTokenDecorationRanges,
  getTokenSpanIssues,
  setTokenDecosEffect,
  sortTokens,
  tokenDecoStateField,
} from "./editor_decorations";
import type { Chip, Token } from "./editor_decorations";
import { buildChipOffsetMap, computeChipSpans } from "./chip_spans";
import type { ChipOffsetMap } from "./chip_spans";

const DEFAULT_SOURCE = `if (
  prop( "feeling" ) == "😀",
    "I am absolutely not pretending.",
      "This is fine 🔥")
`;
const DEBOUNCE_MS = 80;
const ENABLE_SEMANTIC_DIAGNOSTICS = false;

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

const PROPERTIES = ["Title", "feeling", "Status", "Due Date"];

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

function lineColFromUtf16Offset(source: string, offset: number): { line: number; col: number } {
  const prefix = source.slice(0, offset);
  let line = 1;
  for (let i = 0; i < prefix.length; i += 1) {
    if (prefix[i] === "\n") {
      line += 1;
    }
  }
  const lastNewline = prefix.lastIndexOf("\n");
  const col = lastNewline === -1 ? offset + 1 : offset - lastNewline;
  return { line, col };
}

function computeSemanticDiagnostics(source: string, chips: Chip[]): Diagnostic[] {
  const diagnostics: Diagnostic[] = [];
  for (const chip of chips) {
    if (!PROPERTIES.includes(chip.argValue)) {
      const { line, col } = lineColFromUtf16Offset(source, chip.argContentStart);
      diagnostics.push({
        kind: "error",
        message: `${chip.argValue} is not a valid property.`,
        span: {
          start: chip.argContentStart,
          end: chip.argContentEnd,
          line,
          col,
        },
      });
    }
  }
  return diagnostics;
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
    result = analyze(source) as AnalyzeResult;
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

  const syntaxDiagnostics = result.diagnostics || [];
  const chipSpans = computeChipSpans(source, result.tokens || []);
  let chipMap: ChipOffsetMap | undefined;
  try {
    chipMap = buildChipOffsetMap(source.length, chipSpans);
  } catch (error) {
    chipMap = undefined;
    console.warn("Failed to build chip offset map:", error);
  }
  let combinedDiagnostics = [...syntaxDiagnostics];
  if (ENABLE_SEMANTIC_DIAGNOSTICS) {
    const propChips = computePropChips(source, result.tokens || []);
    console.log("propChips", propChips);
    const semanticDiagnostics = computeSemanticDiagnostics(source, propChips);
    combinedDiagnostics = [...combinedDiagnostics, ...semanticDiagnostics];
  }
  renderDiagnostics(combinedDiagnostics, chipMap);
  renderFormatted(result.formatted || "", syntaxDiagnostics);
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
  console.log("[demo] start()");
  try {
    let wasmReady = false;
    editorView = new EditorView({
      state: EditorState.create({
        doc: DEFAULT_SOURCE,
        extensions: [
          keymap.of(defaultKeymap),
          EditorView.lineWrapping,
          tokenDecoStateField,
          EditorView.updateListener.of((update) => {
            if (update.docChanged && wasmReady) {
              runAnalyze(update.state.doc.toString());
            }
            if (update.selectionSet) {
              renderCursorInfo(update.state.selection.main.head, lastSortedTokens);
            }
          }),
        ],
      }),
      parent: editorParentEl,
    });

    renderCursorInfo(editorView.state.selection.main.head, lastSortedTokens);
    await init();
    wasmReady = true;
    runAnalyze(editorView.state.doc.toString());
    console.log("[demo] initialized" );
  } catch (e) {
    console.error("Failed to initialize:", e);
  }
}

start();
