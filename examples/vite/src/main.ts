import "./style.css";
import init, { analyze } from "./pkg/analyzer_wasm.js";

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
  };
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

const sourceEl = expectEl<HTMLTextAreaElement>("#source");
const formattedEl = expectEl<HTMLElement>("#formatted");
const diagnosticsEl = expectEl<HTMLUListElement>("#diagnostics");
const highlightEl = expectEl<HTMLElement>("#highlight");
const tokenWarningEl = expectEl<HTMLElement>("#token-warning");

function sliceUtf16(source: string, start: number, end?: number): string {
  return source.slice(start, end);
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

function buildHighlighted(source: string, tokens: Token[]) {
  highlightEl.innerHTML = "";
  tokenWarningEl.classList.add("hidden");

  if (!tokens || tokens.length === 0) {
    highlightEl.textContent = source;
    return;
  }

  const sorted = [...tokens].sort((a, b) => {
    const startDiff = (a.span?.start ?? 0) - (b.span?.start ?? 0);
    if (startDiff !== 0) return startDiff;
    return (a.span?.end ?? 0) - (b.span?.end ?? 0);
  });

  const fragment = document.createDocumentFragment();
  let cursor = 0;
  const sourceLength = source.length;

  for (const token of sorted) {
    if (!token || token.kind === "Eof") {
      continue;
    }

    const start = token.span?.start ?? 0;
    const end = token.span?.end ?? 0;

    if (start < cursor || end < start || start > sourceLength || end > sourceLength) {
      tokenWarningEl.classList.remove("hidden");
      highlightEl.textContent = source;
      return;
    }

    const gapText = sliceUtf16(source, cursor, start);
    if (gapText) {
      fragment.appendChild(document.createTextNode(gapText));
    }

    const tokenText = sliceUtf16(source, start, end);
    if (tokenText) {
      const span = document.createElement("span");
      span.className = `tok tok-${token.kind}`;
      span.textContent = tokenText;
      fragment.appendChild(span);
    }

    cursor = end;
  }

  const tail = sliceUtf16(source, cursor);
  if (tail) {
    fragment.appendChild(document.createTextNode(tail));
  }

  highlightEl.appendChild(fragment);
}

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
    highlightEl.textContent = source;
    console.error(error);
    return;
  }

  if (!result) {
    formattedEl.textContent = "(no result)";
    diagnosticsEl.innerHTML = "";
    highlightEl.textContent = source;
    return;
  }

  renderDiagnostics(result.diagnostics || []);
  renderFormatted(result.formatted || "", result.diagnostics || []);
  buildHighlighted(source, result.tokens || []);
}, DEBOUNCE_MS);

async function start() {
  sourceEl.value = DEFAULT_SOURCE;
  await init();
  runAnalyze(sourceEl.value);

  sourceEl.addEventListener("input", (event) => {
    runAnalyze((event.target as HTMLTextAreaElement).value);
  });
}

start();
