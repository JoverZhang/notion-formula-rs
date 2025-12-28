import "./style.css";
import init, { analyze } from "./pkg/analyzer_wasm.js";
import { RangeSetBuilder, StateEffect, StateField, EditorState } from "@codemirror/state";
import {
  Decoration,
  DecorationSet,
  EditorView,
  keymap,
} from "@codemirror/view";
import { defaultKeymap } from "@codemirror/commands";

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
    line?: number;
    col?: number;
  };
};

type Token = {
  kind?: string;
  span?: {
    start?: number;
    end?: number;
    line?: number;
    col?: number;
  };
  text?: string;
};

type AnalyzeResult = {
  diagnostics?: Diagnostic[];
  formatted?: string;
  tokens?: Token[];
};

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

const setTokenDecosEffect = StateEffect.define<DecorationSet>();
const tokenDecosField = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setTokenDecosEffect)) {
        return effect.value;
      }
    }
    if (tr.docChanged) {
      return value.map(tr.changes);
    }
    return value;
  },
  provide: (field) => EditorView.decorations.from(field),
});

function debounce<T extends unknown[]>(fn: (...args: T) => void, delay: number) {
  let timer: ReturnType<typeof setTimeout> | null = null;
  return (...args: T) => {
    if (timer) {
      clearTimeout(timer);
    }
    timer = setTimeout(() => fn(...args), delay);
  };
}

function renderDiagnostics(diagnostics: Diagnostic[]) {
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
    li.textContent = `${kind}: ${diag.message} @ ${line}:${col}`;
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

function sortTokens(tokens: Token[]): Token[] {
  return [...tokens].sort((a, b) => {
    const aStart = a.span?.start ?? Number.MAX_SAFE_INTEGER;
    const bStart = b.span?.start ?? Number.MAX_SAFE_INTEGER;
    if (aStart !== bStart) return aStart - bStart;
    const aEnd = a.span?.end ?? Number.MAX_SAFE_INTEGER;
    const bEnd = b.span?.end ?? Number.MAX_SAFE_INTEGER;
    return aEnd - bEnd;
  });
}

function buildTokenDecorations(
  source: string,
  tokens: Token[],
): { ok: boolean; decos: DecorationSet } {
  if (!tokens || tokens.length === 0) {
    return { ok: true, decos: Decoration.none };
  }

  const builder = new RangeSetBuilder<Decoration>();
  const sourceLength = source.length;
  let prevEnd = 0;
  let hasPrev = false;

  for (const token of tokens) {
    if (!token || token.kind === "Eof") {
      continue;
    }

    const start = token.span?.start;
    const end = token.span?.end;
    if (typeof start !== "number" || typeof end !== "number") {
      return { ok: false, decos: Decoration.none };
    }
    if (start === end || token.text === "") {
      continue;
    }
    if (start < 0 || end < 0 || end < start || start > sourceLength || end > sourceLength) {
      return { ok: false, decos: Decoration.none };
    }
    if (hasPrev && start < prevEnd) {
      return { ok: false, decos: Decoration.none };
    }

    builder.add(start, end, Decoration.mark({ class: `tok tok-${token.kind}` }));
    prevEnd = end;
    hasPrev = true;
  }

  return { ok: true, decos: builder.finish() };
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

  renderDiagnostics(result.diagnostics || []);
  renderFormatted(result.formatted || "", result.diagnostics || []);
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
        tokenDecosField,
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
}

start();
