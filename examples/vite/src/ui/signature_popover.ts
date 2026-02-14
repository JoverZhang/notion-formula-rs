import type { SignatureHelp } from "../analyzer/wasm_client";
import {
  computePopoverWidthPx,
  planSignatureTokens,
  pickPopoverSide,
  shouldUseWrappedSignature,
  type SignatureWrapMode,
} from "../model/signature";

function buildSignatureMain(signature: SignatureHelp, mode: SignatureWrapMode): HTMLElement | null {
  const plan = planSignatureTokens(signature, mode);
  if (!plan.tokens.length) return null;

  const main = document.createElement("div");
  main.className = "completion-signature-main";
  for (const token of plan.tokens) {
    if (token.text === "\n") {
      main.append(document.createElement("br"));
      continue;
    }
    const span = document.createElement("span");
    span.className = "completion-signature-seg";
    if (token.active) span.classList.add("is-active");
    span.textContent = token.text;
    main.append(span);
  }
  return main;
}

function hasActionableDiagnostics(rows: string[]): boolean {
  return rows.length > 0 && !(rows.length === 1 && rows[0] === "No diagnostics");
}

function buildDiagnosticsSection(signatureEl: HTMLElement, rows: string[]): HTMLElement | null {
  if (!hasActionableDiagnostics(rows)) return null;

  const section = document.createElement("section");
  section.className = "completion-signature-diagnostics";

  const title = document.createElement("div");
  title.className = "completion-signature-diag-title";
  title.textContent = "Diagnostics";

  const list = document.createElement("ul");
  list.className = "completion-signature-diag-list";
  list.setAttribute("data-testid", "formula-diagnostics");
  const formulaId = signatureEl.getAttribute("data-formula-id");
  if (formulaId) list.setAttribute("data-formula-id", formulaId);

  for (const row of rows) {
    const item = document.createElement("li");
    item.className = "is-error";
    item.textContent = row;
    list.append(item);
  }

  section.append(title, list);
  return section;
}

function paintPopover(
  signatureEl: HTMLElement,
  signature: SignatureHelp | null,
  diagnostics: string[],
  mode: SignatureWrapMode,
): { hasContent: boolean; signatureMain: HTMLElement | null } {
  const signatureMain = signature ? buildSignatureMain(signature, mode) : null;
  const diagnosticsSection = buildDiagnosticsSection(signatureEl, diagnostics);
  const hasContent = Boolean(signatureMain || diagnosticsSection);
  if (!hasContent) {
    signatureEl.replaceChildren();
    return { hasContent: false, signatureMain: null };
  }

  signatureEl.classList.remove("hidden");
  if (signatureMain) {
    signatureEl.dataset.wrap = mode;
  } else {
    delete signatureEl.dataset.wrap;
  }
  signatureEl.replaceChildren();
  if (signatureMain) signatureEl.append(signatureMain);
  if (diagnosticsSection) signatureEl.append(diagnosticsSection);
  return { hasContent: true, signatureMain };
}

export function createSignaturePopover(signatureEl: HTMLElement, editorWrap: HTMLElement) {
  let wrapRaf: number | null = null;

  const updateSide = () => {
    const viewportWidth = document.documentElement.clientWidth || window.innerWidth || 0;
    const wrapRect = editorWrap.getBoundingClientRect();
    signatureEl.dataset.side = pickPopoverSide({
      viewportWidth,
      wrapLeft: wrapRect.left,
      wrapRight: wrapRect.right,
      popoverWidthPx: computePopoverWidthPx(viewportWidth),
    });
  };

  const hide = () => {
    signatureEl.classList.add("hidden");
    signatureEl.textContent = "";
    delete signatureEl.dataset.wrap;
  };

  const render = (signature: SignatureHelp | null, diagnostics: string[], isActive: boolean) => {
    if (!isActive) {
      hide();
      return;
    }

    updateSide();
    const unwrapped = paintPopover(signatureEl, signature, diagnostics, "unwrapped");
    if (!unwrapped.hasContent) {
      hide();
      return;
    }

    if (!signature || !unwrapped.signatureMain) {
      return;
    }
    const signatureMain = unwrapped.signatureMain;

    if (wrapRaf !== null) cancelAnimationFrame(wrapRaf);
    wrapRaf = requestAnimationFrame(() => {
      wrapRaf = null;
      if (!isActive || signatureEl.classList.contains("hidden") || signatureEl.clientWidth === 0)
        return;
      const hasMainOverflow = shouldUseWrappedSignature({
        scrollWidth: signatureMain.scrollWidth,
        clientWidth: signatureMain.clientWidth,
      });
      const hasPopoverOverflow = shouldUseWrappedSignature({
        scrollWidth: signatureEl.scrollWidth,
        clientWidth: signatureEl.clientWidth,
      });
      if (hasMainOverflow || hasPopoverOverflow) {
        paintPopover(signatureEl, signature, diagnostics, "wrapped");
      }
    });
  };

  return { render, hide, updateSide };
}
