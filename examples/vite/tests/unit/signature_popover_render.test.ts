// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SignatureHelpView } from "../../src/analyzer/generated/wasm_dto";
import { createSignaturePopover } from "../../src/ui/signature_popover";

function makeLongSignatureHelp(): SignatureHelpView {
  return {
    signatures: [
      {
        segments: [
          { kind: "Name", text: "if" },
          { kind: "Separator", text: " " },
          { kind: "Punct", text: "(" },
          { kind: "Separator", text: " " },
          { kind: "Param", name: "condition", ty: "boolean", param_index: 0 },
          { kind: "Separator", text: " " },
          { kind: "Punct", text: "," },
          { kind: "Separator", text: " " },
          { kind: "Param", name: "then", ty: "string", param_index: 1 },
          { kind: "Separator", text: " " },
          { kind: "Punct", text: "," },
          { kind: "Separator", text: " " },
          { kind: "Param", name: "else", ty: "(number | string)", param_index: 2 },
          { kind: "Separator", text: " " },
          { kind: "Punct", text: ")" },
          { kind: "Separator", text: " " },
          { kind: "Arrow", text: "->" },
          { kind: "Separator", text: " " },
          { kind: "ReturnType", text: "number | string" },
        ],
      },
    ],
    active_signature: 0,
    active_parameter: 1,
  };
}

describe("signature popover render", () => {
  afterEach(() => {
    document.body.innerHTML = "";
    vi.restoreAllMocks();
  });

  it("uses wrapped mode when popover overflows even if main width appears equal", () => {
    let rafCb: FrameRequestCallback | null = null;
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((cb: FrameRequestCallback) => {
      rafCb = cb;
      return 1;
    });
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => {});

    const editorWrap = document.createElement("div");
    const signatureEl = document.createElement("div");
    signatureEl.className = "completion-signature hidden";
    signatureEl.setAttribute("data-formula-id", "f1");
    document.body.append(editorWrap, signatureEl);

    vi.spyOn(editorWrap, "getBoundingClientRect").mockReturnValue({
      x: 520,
      y: 0,
      width: 240,
      height: 120,
      top: 0,
      right: 760,
      bottom: 120,
      left: 520,
      toJSON() {
        return {};
      },
    } as DOMRect);

    Object.defineProperty(signatureEl, "clientWidth", { configurable: true, get: () => 280 });
    Object.defineProperty(signatureEl, "scrollWidth", { configurable: true, get: () => 520 });

    const popover = createSignaturePopover(signatureEl, editorWrap);
    popover.render(makeLongSignatureHelp(), [], true);

    const main = signatureEl.querySelector(".completion-signature-main");
    expect(main).toBeTruthy();
    Object.defineProperty(main!, "clientWidth", { configurable: true, get: () => 460 });
    Object.defineProperty(main!, "scrollWidth", { configurable: true, get: () => 460 });

    expect(rafCb).toBeTruthy();
    rafCb?.(0);

    expect(signatureEl.dataset.wrap).toBe("wrapped");
    expect(signatureEl.classList.contains("hidden")).toBe(false);
    expect(signatureEl.querySelectorAll(".completion-signature-main br").length).toBeGreaterThan(0);
  });

});
