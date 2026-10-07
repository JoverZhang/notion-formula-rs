import { EditorView } from "@codemirror/view";
import type { CompletionItem, CursorHelp, UpdateExpressionResult } from "@notion-formula/sdk";
import { vi } from "vitest";
import type { FormulaEditorActions, FormulaState } from "../../src/app/types";
import { createFormulaPanelView } from "../../src/ui/formula_panel_view";

export const completionItem = (overrides: Partial<CompletionItem> = {}): CompletionItem => ({
  label: "if()",
  kind: "FunctionGeneral",
  insert_text: "if()",
  primary_edit: { range: { start: 0, end: 1 }, new_text: "if()" },
  cursor: 3,
  additional_edits: [],
  detail: null,
  is_disabled: false,
  disabled_reason: null,
  ...overrides,
});

export const cursorHelp = (overrides: Partial<CursorHelp> = {}): CursorHelp => ({
  base_version: 0n,
  completion: { items: [completionItem()], replace: { start: 0, end: 1 }, preferred_indices: [] },
  signature_help: null,
  ...overrides,
});

export function mountFormulaPanel(initialSource = "i") {
  const actions = {
    setSource: vi.fn<FormulaEditorActions["setSource"]>(),
    help: vi.fn<FormulaEditorActions["help"]>().mockResolvedValue(cursorHelp()),
    format: vi.fn<FormulaEditorActions["format"]>().mockResolvedValue(null),
    quickFixes: vi.fn<FormulaEditorActions["quickFixes"]>().mockResolvedValue([]),
    applyEdit: vi.fn<FormulaEditorActions["applyEdit"]>().mockResolvedValue(null),
    save: vi.fn<FormulaEditorActions["save"]>().mockResolvedValue(),
    discard: vi.fn<FormulaEditorActions["discard"]>().mockResolvedValue(),
  };
  const panel = createFormulaPanelView({ id: "Formula 1", label: "Test", initialSource, actions });
  panel.mount(document.body);
  const editorNode = panel.root.querySelector<HTMLElement>(".cm-editor")!;
  const editor = EditorView.findFromDOM(editorNode)!;
  let state: FormulaState = {
    id: "Formula 1",
    source: initialSource,
    savedSource: "",
    version: 0n,
    diagnostics: [],
    tokens: [],
    outputType: "Unknown",
    dirty: true,
    error: null,
    cursor: null,
    status: "ok",
  };
  const update = (next: Partial<FormulaState> = {}, saving = false) => {
    state = { ...state, ...next };
    panel.update(state, saving);
  };
  const publishNative = (source: string, cursor: number): UpdateExpressionResult => {
    const result: UpdateExpressionResult = {
      state: {
        version: (state.version ?? 0n) + 1n,
        definition: { id: "Formula 1", expression: source },
        output_type: "Unknown",
        diagnostics: [],
        tokens: [],
        property_references: [],
      },
      cursor,
    };
    update({ source, cursor, version: result.state.version });
    return result;
  };
  update();
  return { actions, panel, editor, update, publishNative };
}
