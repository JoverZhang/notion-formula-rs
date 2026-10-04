import type { CompletionItem } from "@notion-formula/sdk";
import { describe, expect, it } from "vitest";
import {
  buildCompletionRows,
  COMPLETION_ROW_ITEM_RECOMMENDED,
  COMPLETION_ROW_LABEL_GROUP,
  COMPLETION_ROW_LABEL_RECOMMENDED,
  getSelectedItemIndex,
  nextSelectedRowIndex,
  normalizeSelectedRowIndex,
} from "../../src/model/completions";

function makeItem(overrides: Partial<CompletionItem>): CompletionItem {
  return {
    label: "x",
    kind: "FunctionGeneral",
    insert_text: "x",
    primary_edit: null,
    cursor: null,
    additional_edits: [],
    detail: null,
    is_disabled: false,
    disabled_reason: null,
    ...overrides,
  };
}

describe("completion row planning", () => {
  it("dedupes recommended indices and skips disabled recommended items", () => {
    const items: CompletionItem[] = [
      makeItem({ label: "a", kind: "FunctionGeneral" }),
      makeItem({ label: "b", kind: "FunctionText", is_disabled: true }),
      makeItem({ label: "c", kind: "Builtin" }),
    ];

    const rows = buildCompletionRows(items, [1, 0, 1, 2]);

    const recommendedHeaderIndex = rows.findIndex(
      (row) => row.kind === "label" && (row.flags & COMPLETION_ROW_LABEL_RECOMMENDED) !== 0,
    );
    expect(recommendedHeaderIndex).toBeGreaterThanOrEqual(0);

    const recommendedItems = rows.filter(
      (row) => row.kind === "item" && (row.flags & COMPLETION_ROW_ITEM_RECOMMENDED) !== 0,
    );
    expect(recommendedItems.map((row) => row.itemIndex)).toEqual([0, 2]);

    const itemIndexCounts = new Map<number, number>();
    for (const row of rows) {
      if (row.kind !== "item") continue;
      itemIndexCounts.set(row.itemIndex, (itemIndexCounts.get(row.itemIndex) ?? 0) + 1);
    }
    expect(itemIndexCounts.get(0)).toBe(1);
    expect(itemIndexCounts.get(2)).toBe(1);
    // Disabled item should not be marked recommended.
    expect(recommendedItems.some((row) => row.itemIndex === 1)).toBe(false);
  });

  it("emits kind group labels and groups non-recommended items by kind order", () => {
    const items: CompletionItem[] = [
      makeItem({ label: "textFn", kind: "FunctionText" }),
      makeItem({ label: "genFn", kind: "FunctionGeneral" }),
      makeItem({ label: "textFn2", kind: "FunctionText" }),
      makeItem({ label: "not", kind: "Builtin" }),
      makeItem({ label: "true", kind: "Builtin" }),
      makeItem({ label: "+", kind: "Operator" }),
    ];

    const rows = buildCompletionRows(items, []);

    const labels = rows
      .filter((row) => row.kind === "label" && (row.flags & COMPLETION_ROW_LABEL_GROUP) !== 0)
      .map((row) => row.label);
    expect(labels).toEqual([
      "Text Functions",
      "General Functions",
      "Text Functions",
      "Built-ins",
      "Operators",
    ]);
  });

  it("selection helpers skip non-items and wrap around", () => {
    const items: CompletionItem[] = [
      makeItem({ label: "genFn", kind: "FunctionGeneral" }),
      makeItem({ label: "textFn", kind: "FunctionText" }),
      makeItem({ label: "not", kind: "Builtin" }),
    ];
    const rows = buildCompletionRows(items, []);
    const itemRowIndices = rows
      .map((row, idx) => (row.kind === "item" ? idx : -1))
      .filter((idx) => idx !== -1);
    expect(itemRowIndices.length).toBeGreaterThan(0);

    const firstItemRow = itemRowIndices[0];
    const lastItemRow = itemRowIndices[itemRowIndices.length - 1];

    expect(normalizeSelectedRowIndex(rows, 0)).toBe(firstItemRow);
    expect(nextSelectedRowIndex(rows, -1, 1)).toBe(firstItemRow);
    expect(nextSelectedRowIndex(rows, -1, -1)).toBe(lastItemRow);
    expect(nextSelectedRowIndex(rows, lastItemRow, 1)).toBe(firstItemRow);

    expect(getSelectedItemIndex(rows, 0)).toBeNull();
    expect(getSelectedItemIndex(rows, firstItemRow)).toBeTypeOf("number");
  });

  it("keeps disabled properties visible while excluding them from every selection path", () => {
    const items = [
      makeItem({ label: "Formula 1", kind: "Property", is_disabled: true }),
      makeItem({ label: "Number", kind: "Property" }),
      makeItem({ label: "Formula 2", kind: "Property", is_disabled: true }),
    ];
    const rows = buildCompletionRows(items, [0, 2]);
    expect(rows.filter((row) => row.kind === "label").map((row) => row.label)).toEqual([
      "Properties",
    ]);
    expect(rows.filter((row) => row.kind === "item").map((row) => row.label)).toEqual([
      "Formula 1",
      "Number",
      "Formula 2",
    ]);
    const disabledRows = rows
      .map((row, index) => (row.kind === "item" && row.itemIndex !== 1 ? index : -1))
      .filter((index) => index >= 0);
    const enabledRow = rows.findIndex((row) => row.kind === "item" && row.itemIndex === 1);
    for (const index of disabledRows) {
      expect(getSelectedItemIndex(rows, index)).toBeNull();
      expect(normalizeSelectedRowIndex(rows, index)).toBe(enabledRow);
      expect(nextSelectedRowIndex(rows, index, 1)).toBe(enabledRow);
      expect(nextSelectedRowIndex(rows, index, -1)).toBe(enabledRow);
    }
    expect(nextSelectedRowIndex(rows, enabledRow, 1)).toBe(enabledRow);
    expect(nextSelectedRowIndex(rows, enabledRow, -1)).toBe(enabledRow);
  });

  it("has no selected row when every visible property is disabled", () => {
    const rows = buildCompletionRows(
      [makeItem({ label: "Formula 1", kind: "Property", is_disabled: true })],
      [0],
    );
    expect(rows.map((row) => row.label)).toEqual(["Properties", "Formula 1"]);
    expect(getSelectedItemIndex(rows, 1)).toBeNull();
    expect(normalizeSelectedRowIndex(rows, 1)).toBe(-1);
    expect(nextSelectedRowIndex(rows, -1, 1)).toBe(-1);
    expect(nextSelectedRowIndex(rows, 1, -1)).toBe(-1);
  });
});
