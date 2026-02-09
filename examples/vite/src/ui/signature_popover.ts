import type { SignatureHelp } from "../analyzer/wasm_client";

type SignatureWrapMode = "unwrapped" | "wrapped";

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function signatureSegText(
  seg: NonNullable<SignatureHelp>["signatures"][number]["segments"][number],
): string {
  switch (seg.kind) {
    case "Ellipsis":
      return "...";
    case "Param":
      return `${seg.name}: ${seg.ty}`;
    default:
      return seg.text;
  }
}

function createSigSegEl(
  seg: NonNullable<SignatureHelp>["signatures"][number]["segments"][number],
  activeParam: number,
): HTMLElement {
  const segEl = document.createElement("span");
  segEl.className = `completion-signature-seg completion-signature-seg--${seg.kind}`;
  if (seg.kind === "Param" && seg.param_index === activeParam) {
    segEl.classList.add("is-active");
  }
  segEl.textContent = signatureSegText(seg);
  return segEl;
}

function splitArgSegments(
  segments: NonNullable<SignatureHelp>["signatures"][number]["segments"],
): Array<NonNullable<SignatureHelp>["signatures"][number]["segments"]> {
  const args: Array<NonNullable<SignatureHelp>["signatures"][number]["segments"]> = [];
  let current: NonNullable<SignatureHelp>["signatures"][number]["segments"] = [];

  for (const seg of segments) {
    const isComma =
      (seg.kind === "Punct" && seg.text === ",") ||
      (seg.kind === "Separator" && seg.text.includes(","));
    if (isComma) {
      if (current.length) args.push(current);
      current = [];
      continue;
    }
    current.push(seg);
  }

  if (current.length) args.push(current);

  return args.map((arg) => {
    let start = 0;
    let end = arg.length;
    while (start < end && signatureSegText(arg[start])?.trim() === "") start += 1;
    while (end > start && signatureSegText(arg[end - 1])?.trim() === "") end -= 1;
    return arg.slice(start, end);
  });
}

function renderSignatureMode(
  signatureEl: HTMLElement,
  signature: SignatureHelp,
  mode: SignatureWrapMode,
): void {
  const activeSig = signature.signatures[signature.active_signature] ?? signature.signatures[0];
  if (!activeSig) {
    signatureEl.classList.add("hidden");
    signatureEl.textContent = "";
    return;
  }

  signatureEl.classList.remove("hidden");
  signatureEl.replaceChildren();
  signatureEl.dataset.wrap = mode;

  const arrowIndex = activeSig.segments.findIndex((seg) => seg.kind === "Arrow");
  const beforeArrow = arrowIndex === -1 ? activeSig.segments.length : arrowIndex;
  const openIndex = [...activeSig.segments]
    .map((seg, idx) => ({ seg, idx }))
    .reverse()
    .find(({ seg, idx }) => idx < beforeArrow && seg.kind === "Punct" && seg.text === "(")?.idx;
  const closeIndex =
    typeof openIndex === "number"
      ? activeSig.segments.findIndex(
          (seg, idx) =>
            idx > openIndex && idx < beforeArrow && seg.kind === "Punct" && seg.text === ")",
        )
      : -1;

  if (mode === "wrapped" && typeof openIndex === "number" && closeIndex !== -1) {
    const prefix = activeSig.segments.slice(0, openIndex + 1);
    const argSegs = activeSig.segments.slice(openIndex + 1, closeIndex);
    const suffix = activeSig.segments.slice(closeIndex);
    const args = splitArgSegments(argSegs);

    prefix.forEach((seg) => signatureEl.append(createSigSegEl(seg, signature.active_parameter)));
    signatureEl.append(document.createElement("br"));

    args.forEach((arg, idx) => {
      signatureEl.append("  ");
      arg.forEach((seg) => signatureEl.append(createSigSegEl(seg, signature.active_parameter)));
      if (idx < args.length - 1) {
        signatureEl.append(
          createSigSegEl({ kind: "Punct", text: "," }, signature.active_parameter),
        );
      }
      signatureEl.append(document.createElement("br"));
    });

    suffix.forEach((seg) => signatureEl.append(createSigSegEl(seg, signature.active_parameter)));
    return;
  }

  activeSig.segments.forEach((seg) =>
    signatureEl.append(createSigSegEl(seg, signature.active_parameter)),
  );
}

export function createSignaturePopover(signatureEl: HTMLElement, editorWrap: HTMLElement) {
  const SIG_POPOVER_GAP_PX = 12;
  const SIG_POPOVER_MIN_W_PX = 240;
  const SIG_POPOVER_MAX_W_PX = 360;
  const SIG_POPOVER_PREF_VW = 0.28;

  let signatureWrapRaf: number | null = null;
  let signatureWrapObservedWidth = -1;
  let lastSignature: SignatureHelp | null = null;
  let active = false;

  const observer =
    typeof ResizeObserver === "undefined"
      ? null
      : new ResizeObserver((entries) => {
          const entry = entries[0];
          const width = Math.floor(entry?.contentRect.width ?? 0);
          if (width === signatureWrapObservedWidth) return;
          signatureWrapObservedWidth = width;
          render(lastSignature, active);
        });
  observer?.observe(signatureEl);

  function updateSide() {
    const viewportWidth = document.documentElement.clientWidth || window.innerWidth || 0;
    const wrapRect = editorWrap.getBoundingClientRect();
    const leftSpace = wrapRect.left - SIG_POPOVER_GAP_PX;
    const rightSpace = viewportWidth - wrapRect.right - SIG_POPOVER_GAP_PX;
    const popoverWidth = clamp(
      viewportWidth * SIG_POPOVER_PREF_VW,
      SIG_POPOVER_MIN_W_PX,
      SIG_POPOVER_MAX_W_PX,
    );
    const canFitLeft = leftSpace >= popoverWidth;
    const canFitRight = rightSpace >= popoverWidth;

    if (canFitLeft) {
      signatureEl.dataset.side = "left";
      return;
    }
    if (canFitRight) {
      signatureEl.dataset.side = "right";
      return;
    }

    signatureEl.dataset.side = rightSpace > leftSpace ? "right" : "left";
  }

  function hide() {
    active = false;
    signatureEl.classList.add("hidden");
    signatureEl.textContent = "";
    delete signatureEl.dataset.wrap;
  }

  function render(signature: SignatureHelp | null, isActive: boolean) {
    lastSignature = signature;
    active = isActive;

    if (!isActive || !signature) {
      hide();
      return;
    }

    updateSide();
    renderSignatureMode(signatureEl, signature, "unwrapped");
    if (signatureWrapRaf !== null) cancelAnimationFrame(signatureWrapRaf);
    signatureWrapRaf = requestAnimationFrame(() => {
      signatureWrapRaf = null;
      if (!active || signatureEl.classList.contains("hidden")) return;
      if (signatureEl.clientWidth === 0) return;
      if (signatureEl.scrollWidth > signatureEl.clientWidth + 1) {
        renderSignatureMode(signatureEl, signature, "wrapped");
      }
    });
  }

  return {
    render,
    hide,
    updateSide,
  };
}
