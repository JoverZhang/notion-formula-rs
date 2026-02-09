import type { SignatureHelp } from "../analyzer/wasm_client";
import {
  computePopoverWidthPx,
  planSignatureTokens,
  pickPopoverSide,
  shouldUseWrappedSignature,
  type SignatureWrapMode,
} from "../model/signature";

function paintSignature(
  signatureEl: HTMLElement,
  signature: SignatureHelp,
  mode: SignatureWrapMode,
): boolean {
  const plan = planSignatureTokens(signature, mode);
  if (!plan.tokens.length) return false;

  signatureEl.classList.remove("hidden");
  signatureEl.dataset.wrap = mode;
  signatureEl.replaceChildren();
  for (const token of plan.tokens) {
    if (token.text === "\n") {
      signatureEl.append(document.createElement("br"));
      continue;
    }
    const span = document.createElement("span");
    span.className = "completion-signature-seg";
    if (token.active) span.classList.add("is-active");
    span.textContent = token.text;
    signatureEl.append(span);
  }
  return true;
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

  const render = (signature: SignatureHelp | null, isActive: boolean) => {
    if (!isActive || !signature) {
      hide();
      return;
    }

    updateSide();
    if (!paintSignature(signatureEl, signature, "unwrapped")) {
      hide();
      return;
    }

    if (wrapRaf !== null) cancelAnimationFrame(wrapRaf);
    wrapRaf = requestAnimationFrame(() => {
      wrapRaf = null;
      if (!isActive || signatureEl.classList.contains("hidden") || signatureEl.clientWidth === 0)
        return;
      if (
        shouldUseWrappedSignature({
          scrollWidth: signatureEl.scrollWidth,
          clientWidth: signatureEl.clientWidth,
        })
      ) {
        paintSignature(signatureEl, signature, "wrapped");
      }
    });
  };

  return { render, hide, updateSide };
}
