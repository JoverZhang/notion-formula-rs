import type { CompletionItem } from "@notion-formula/sdk";
import { describe, expect, it } from "vitest";
import { createCompletionEdit, getCompletionCursor } from "../../src/model/completions";

describe("completion edits", () => {
  const baseItem: CompletionItem = {
    label: "x",
    kind: "FunctionGeneral",
    insert_text: "x",
    primary_edit: { range: { start: 0, end: 0 }, new_text: "x" },
    cursor: null,
    additional_edits: [],
    detail: null,
    is_disabled: false,
    disabled_reason: null,
  };

  it("uses explicit cursor when provided", () => {
    const item = {
      ...baseItem,
      primary_edit: { range: { start: 0, end: 5 }, new_text: "hi" },
      cursor: 1,
    };
    expect(getCompletionCursor(item, 5)).toBe(1);
  });

  it("preserves help version and original edit objects for Rust", () => {
    const item = {
      ...baseItem,
      primary_edit: { range: { start: 2, end: 4 }, new_text: "sum()" },
      cursor: null,
      additional_edits: [{ range: { start: 0, end: 0 }, new_text: "qq" }],
    };
    const result = createCompletionEdit(item, 7n);
    expect(result).toEqual({
      base_version: 7n,
      edits: [item.primary_edit, ...item.additional_edits],
    });
    expect(result?.edits[0]).toBe(item.primary_edit);
    expect(result?.edits[1]).toBe(item.additional_edits[0]);
    expect(getCompletionCursor(item, 9)).toBe(9);
  });

  it("returns null for disabled or edit-less items", () => {
    expect(createCompletionEdit({ ...baseItem, is_disabled: true }, 3n)).toBeNull();
    expect(createCompletionEdit({ ...baseItem, primary_edit: null }, 3n)).toBeNull();
    expect(createCompletionEdit(undefined, 3n)).toBeNull();
  });
});
