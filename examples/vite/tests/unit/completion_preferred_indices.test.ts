// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { EditorView } from "@codemirror/view";

const { completeSourceMock } = vi.hoisted(() => ({
  completeSourceMock: vi.fn(),
}));

vi.mock("../../src/analyzer/wasm_client", () => ({
  completeSource: completeSourceMock,
  posToLineCol: () => ({ line: 1, col: 1 }),
}));

import { createFormulaPanelView } from "../../src/ui/formula_panel_view";

beforeEach(() => {
  vi.useFakeTimers();
  // jsdom may not implement this; the view calls it opportunistically.
  if (!("scrollIntoView" in HTMLElement.prototype)) {
    (HTMLElement.prototype as unknown as { scrollIntoView: () => void }).scrollIntoView = () => {};
  }
});

afterEach(() => {
  vi.useRealTimers();
  completeSourceMock.mockReset();
  document.body.innerHTML = "";
});

function mountAndGetEditorView(initialSource: string): EditorView {
  const view = createFormulaPanelView({
    id: "f1",
    label: "Test",
    initialSource,
    onSourceChange() {},
  });
  view.mount(document.body);

  const editorHost = document.querySelector(
    '[data-testid="formula-editor"][data-formula-id="f1"]',
  ) as HTMLElement | null;
  expect(editorHost).toBeTruthy();

  const cmNode =
    (editorHost?.querySelector(".cm-editor") as HTMLElement | null) ??
    (editorHost?.querySelector(".cm-content") as HTMLElement | null) ??
    editorHost;
  const editorView = cmNode ? EditorView.findFromDOM(cmNode) : null;
  expect(editorView).toBeTruthy();
  return editorView as EditorView;
}

describe("recommended completions", () => {
  const items = [
    {
      label: "textFn",
      kind: "Function",
      category: "Text",
      insert_text: "textFn()",
      primary_edit: null,
      cursor: null,
      additional_edits: [],
      detail: null,
      is_disabled: false,
      disabled_reason: null,
    },
    {
      label: "generalFn",
      kind: "Function",
      category: "General",
      insert_text: "generalFn()",
      primary_edit: null,
      cursor: null,
      additional_edits: [],
      detail: null,
      is_disabled: false,
      disabled_reason: null,
    },
  ];

  it("does not show Recommended when preferred_indices is empty", () => {
    completeSourceMock.mockReturnValue({
      items,
      replace: { start: 0, end: 0 },
      signature_help: null,
      preferred_indices: [],
    });

    const editorView = mountAndGetEditorView("");
    editorView.dispatch({ selection: { anchor: 0 } });
    vi.advanceTimersByTime(200);

    expect(
      document.querySelector('[data-formula-id="f1"] .completion-recommended-header'),
    ).toBeFalsy();
    expect(document.querySelector('[data-formula-id="f1"] .completion-item.is-recommended')).toBe(
      null,
    );
  });

  it("shows Recommended when preferred_indices is non-empty, and marks items", () => {
    completeSourceMock.mockReturnValue({
      items,
      replace: { start: 0, end: 2 },
      signature_help: null,
      preferred_indices: [0],
    });

    const editorView = mountAndGetEditorView("ge");
    editorView.dispatch({ selection: { anchor: 2 } });
    vi.advanceTimersByTime(200);

    expect(
      document.querySelector('[data-formula-id="f1"] .completion-recommended-header'),
    ).toBeTruthy();

    const firstItem = document.querySelector(
      '[data-formula-id="f1"] .completion-item',
    ) as HTMLElement | null;
    expect(firstItem?.getAttribute("data-completion-index")).toBe("0");
    expect(firstItem?.getAttribute("data-completion-recommended")).toBe("true");
    expect(firstItem?.classList.contains("is-recommended")).toBe(true);

    const preferredInDom = document.querySelectorAll(
      '[data-formula-id="f1"] [data-completion-index="0"]',
    );
    expect(preferredInDom.length).toBe(1);
  });

  it("renders multiple preferred_indices once each (no duplicates) and preserves their order", () => {
    completeSourceMock.mockReturnValue({
      items,
      replace: { start: 0, end: 2 },
      signature_help: null,
      preferred_indices: [1, 0],
    });

    const editorView = mountAndGetEditorView("zz");
    editorView.dispatch({ selection: { anchor: 2 } });
    vi.advanceTimersByTime(200);

    const allItems = Array.from(
      document.querySelectorAll('[data-formula-id="f1"] .completion-item'),
    ) as HTMLElement[];
    expect(allItems.length).toBeGreaterThanOrEqual(2);
    expect(allItems[0]?.getAttribute("data-completion-index")).toBe("1");
    expect(allItems[1]?.getAttribute("data-completion-index")).toBe("0");

    const recommended = document.querySelectorAll(
      '[data-formula-id="f1"] .completion-item[data-completion-recommended="true"]',
    );
    expect(recommended.length).toBe(2);

    const preferred0 = document.querySelectorAll(
      '[data-formula-id="f1"] [data-completion-index="0"]',
    );
    const preferred1 = document.querySelectorAll(
      '[data-formula-id="f1"] [data-completion-index="1"]',
    );
    expect(preferred0.length).toBe(1);
    expect(preferred1.length).toBe(1);
  });
});
