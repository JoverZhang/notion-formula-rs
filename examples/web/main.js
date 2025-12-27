import init, { analyze } from "./pkg/analyzer_wasm.js";

const DEFAULT_SOURCE = `if (
  prop( "feeling" ) == "😀",
    "I am absolutely not pretending.",
      "This is fine 🔥")
`
const DEBOUNCE_MS = 80;

const sourceEl = document.querySelector("#source");
const formattedEl = document.querySelector("#formatted");
const diagnosticsEl = document.querySelector("#diagnostics");
const highlightEl = document.querySelector("#highlight");
const tokenWarningEl = document.querySelector("#token-warning");

function sliceUtf16(source, start, end) {
  return source.slice(start, end);
}

function debounce(fn, delay) {
  let timer = null;
  return (...args) => {
    if (timer) {
      clearTimeout(timer);
    }
    timer = setTimeout(() => fn(...args), delay);
  };
}

function renderDiagnostics(diagnostics) {
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

function renderFormatted(formatted, diagnostics) {
  if (!formatted && diagnostics && diagnostics.length > 0) {
    formattedEl.textContent = "(no formatted output due to fatal error)";
    return;
  }
  formattedEl.textContent = formatted || "";
}

function buildHighlighted(source, tokens) {
  highlightEl.innerHTML = "";
  tokenWarningEl.classList.add("hidden");

  if (!tokens || tokens.length === 0) {
    highlightEl.textContent = source;
    return;
  }

  const sorted = [...tokens].sort((a, b) => {
    const startDiff = a.span.start - b.span.start;
    if (startDiff !== 0) return startDiff;
    return a.span.end - b.span.end;
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

const runAnalyze = debounce((source) => {
  let result = null;
  try {
    result = analyze(source);
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
    runAnalyze(event.target.value);
  });
}

start();
