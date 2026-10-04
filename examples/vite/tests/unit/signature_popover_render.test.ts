// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SignatureHelp } from "../../src/formula/client";
import { createSignaturePopover } from "../../src/ui/signature_popover";

function makeLongSignatureHelp(): SignatureHelp {
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
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((cb) => {
      cb(0);
      return 1;
    });
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => {});

    const signatureEl = document.createElement("div");
    signatureEl.className = "completion-signature hidden";
    signatureEl.setAttribute("data-formula-id", "Formula 1");
    document.body.append(signatureEl);

    Object.defineProperty(signatureEl, "clientWidth", { configurable: true, get: () => 280 });
    Object.defineProperty(signatureEl, "scrollWidth", { configurable: true, get: () => 520 });

    const popover = createSignaturePopover(signatureEl);
    popover.render(makeLongSignatureHelp(), [], true);

    const main = signatureEl.querySelector(".completion-signature-main");
    expect(main).toBeTruthy();
    Object.defineProperty(main!, "clientWidth", { configurable: true, get: () => 460 });
    Object.defineProperty(main!, "scrollWidth", { configurable: true, get: () => 460 });

    expect(signatureEl.dataset.wrap).toBe("wrapped");
    expect(signatureEl.classList.contains("hidden")).toBe(false);
    expect(signatureEl.querySelectorAll(".completion-signature-main br").length).toBeGreaterThan(0);
  });

  it("ignores a pending wrap frame after hide or dispose", () => {
    let wrap!: FrameRequestCallback;
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
      wrap = callback;
      return 1;
    });
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => {});
    const signatureEl = document.createElement("div");
    Object.defineProperty(signatureEl, "clientWidth", { configurable: true, value: 280 });
    Object.defineProperty(signatureEl, "scrollWidth", { configurable: true, value: 520 });
    const popover = createSignaturePopover(signatureEl);
    popover.render(makeLongSignatureHelp(), [], true);
    const pending = wrap;
    popover.hide();
    popover.render(null, ["error 1:1: current diagnostic"], true);
    pending(0);
    expect(signatureEl.querySelector(".completion-signature-main")).toBeNull();
    expect(signatureEl.textContent).toContain("current diagnostic");
    popover.dispose();
    pending(0);
    popover.render(makeLongSignatureHelp(), [], true);
    expect(signatureEl.textContent).toBe("");
    expect(signatureEl.classList.contains("hidden")).toBe(true);
  });
});
