import { describe, expect, it } from "vitest";
import { applyCompletion, applyTextEdits, type CompletionItem } from "../../src/editor/text_edits";

describe("applyTextEdits", () => {
  it("applies a single replacement", () => {
    const source = "abc";
    const updated = applyTextEdits(source, [{ range: { start: 1, end: 2 }, new_text: "Z" }]);
    expect(updated).toBe("aZc");
  });

  it("applies multiple edits in reverse order", () => {
    const source = "abcdef";
    const edits = [
      { range: { start: 4, end: 6 }, new_text: "Y" },
      { range: { start: 1, end: 3 }, new_text: "X" },
    ];
    const updated = applyTextEdits(source, edits);
    expect(updated).toBe("aXdef".replace("ef", "Y"));
    expect(updated).toBe("aXdY");
  });
});

describe("applyCompletion", () => {
  const baseItem: CompletionItem = {
    label: "x",
    kind: "Keyword",
    insert_text: "x",
    primary_edit: { range: { start: 0, end: 0 }, new_text: "x" },
    cursor: null,
    additional_edits: [],
    detail: null,
    is_disabled: false,
    disabled_reason: null,
  };

  it("uses explicit cursor when provided", () => {
    const { newText, newCursor } = applyCompletion("hello", {
      ...baseItem,
      primary_edit: { range: { start: 0, end: 5 }, new_text: "hi" },
      cursor: 1,
    });
    expect(newText).toBe("hi");
    expect(newCursor).toBe(1);
  });

  it("falls back to after inserted text when cursor is missing", () => {
    const { newText, newCursor } = applyCompletion("su", {
      ...baseItem,
      primary_edit: { range: { start: 0, end: 2 }, new_text: "sum()" },
      cursor: null,
    });
    expect(newText).toBe("sum()");
    expect(newCursor).toBe(5);
  });
});

