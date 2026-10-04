// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { completionItem, cursorHelp, mountFormulaPanel } from "../helpers/formula_panel";

let mounted: ReturnType<typeof mountFormulaPanel> | undefined;

beforeEach(() => {
  vi.useFakeTimers();
  Object.defineProperty(Range.prototype, "getClientRects", { configurable: true, value: () => [] });
  Object.defineProperty(Range.prototype, "getBoundingClientRect", {
    configurable: true,
    value: () => new DOMRect(),
  });
});

afterEach(() => {
  mounted?.panel.dispose();
  mounted = undefined;
  vi.useRealTimers();
  document.body.innerHTML = "";
});

it("shows a disabled property's reason without letting it replace the active completion", async () => {
  mounted = mountFormulaPanel();
  const { panel, editor, actions } = mounted;
  const reason = "Formula property `Formula 2` is not ready";
  actions.help.mockResolvedValue(
    cursorHelp({
      completion: {
        items: [
          completionItem({
            label: "Formula 2",
            kind: "Property",
            detail: "number",
            is_disabled: true,
            disabled_reason: reason,
          }),
          completionItem({ label: "Number", kind: "Property" }),
        ],
        replace: { start: 0, end: 1 },
        preferred_indices: [0],
      },
    }),
  );
  editor.focus();
  await vi.advanceTimersByTimeAsync(200);

  const disabled = panel.root.querySelector<HTMLElement>('[data-completion-index="0"]')!;
  expect(disabled).not.toBeNull();
  expect(disabled.getAttribute("aria-disabled")).toBe("true");
  expect(disabled.querySelector(".completion-item-meta")?.textContent).toBe("number");
  expect(disabled.querySelector(".completion-item-reason")?.textContent).toBe(reason);
  expect(panel.root.querySelector(".completion-recommended-header")).toBeNull();

  disabled.dispatchEvent(new MouseEvent("mouseenter"));
  expect(disabled.classList.contains("is-selected")).toBe(false);
  expect(panel.root.querySelector(".completion-item.is-selected")?.textContent).toBe("Number");
  disabled.click();
  expect(actions.applyEdit).not.toHaveBeenCalled();
  expect(editor.state.doc.toString()).toBe("i");
});
