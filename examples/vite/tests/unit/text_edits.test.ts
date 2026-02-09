import { describe, expect, it } from "vitest";
import { applyCompletionItem, type CompletionItem } from "../../src/analyzer/wasm_client";

function applyChanges(
  source: string,
  changes: Array<{ from: number; to: number; insert: string }>,
): string {
  let text = source;
  for (const change of [...changes].sort((a, b) => b.from - a.from || b.to - a.to)) {
    text = text.slice(0, change.from) + change.insert + text.slice(change.to);
  }
  return text;
}

describe("applyCompletionItem", () => {
  const baseItem: CompletionItem = {
    label: "x",
    kind: "Function",
    category: "General",
    insert_text: "x",
    primary_edit: { range: { start: 0, end: 0 }, new_text: "x" },
    cursor: null,
    additional_edits: [],
    detail: null,
    is_disabled: false,
    disabled_reason: null,
  };

  it("uses explicit cursor when provided", () => {
    const result = applyCompletionItem({
      ...baseItem,
      primary_edit: { range: { start: 0, end: 5 }, new_text: "hi" },
      cursor: 1,
    });
    expect(result).not.toBeNull();
    expect(result?.cursor).toBe(1);
    expect(applyChanges("hello", result?.changes ?? [])).toBe("hi");
  });

  it("falls back to primary + pre-primary edit offset when cursor is missing", () => {
    const result = applyCompletionItem({
      ...baseItem,
      primary_edit: { range: { start: 2, end: 4 }, new_text: "sum()" },
      cursor: null,
      additional_edits: [{ range: { start: 0, end: 0 }, new_text: "qq" }],
    });
    expect(result).not.toBeNull();
    expect(result?.cursor).toBe(9);
    expect(applyChanges("abxxcd", result?.changes ?? [])).toBe("qqabsum()cd");
  });

  it("returns null for disabled or edit-less items", () => {
    expect(applyCompletionItem({ ...baseItem, is_disabled: true })).toBeNull();
    expect(applyCompletionItem({ ...baseItem, primary_edit: null })).toBeNull();
  });
});
