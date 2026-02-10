import type { CompletionItem } from "../analyzer/wasm_client";

export const COMPLETION_ROW_LABEL_GROUP = 1 << 0;
export const COMPLETION_ROW_LABEL_RECOMMENDED = 1 << 1;
export const COMPLETION_ROW_ITEM_RECOMMENDED = 1 << 2;

export type CompletionRenderRow = {
  kind: "label" | "item";
  label: string;
  itemIndex: number;
  flags: number;
};

function kindLabel(kind: CompletionItem["kind"]): string {
  if (kind === "FunctionGeneral") return "General Functions";
  if (kind === "FunctionText") return "Text Functions";
  if (kind === "FunctionNumber") return "Number Functions";
  if (kind === "FunctionDate") return "Date Functions";
  if (kind === "FunctionPeople") return "People Functions";
  if (kind === "FunctionList") return "List Functions";
  if (kind === "FunctionSpecial") return "Special Functions";
  if (kind === "Builtin") return "Built-ins";
  if (kind === "Property") return "Properties";
  if (kind === "Operator") return "Operators";
  return String(kind);
}

export function buildCompletionRows(
  items: CompletionItem[],
  preferredIndices: number[],
): CompletionRenderRow[] {
  const rows: CompletionRenderRow[] = [];
  const recommended = new Set<number>();
  const recommendedOrdered: number[] = [];

  for (const itemIndex of preferredIndices) {
    if (!Number.isInteger(itemIndex)) continue;
    if (itemIndex < 0 || itemIndex >= items.length) continue;
    if (recommended.has(itemIndex)) continue;
    const item = items[itemIndex];
    if (!item || item.is_disabled) continue;
    recommended.add(itemIndex);
    recommendedOrdered.push(itemIndex);
  }

  if (recommendedOrdered.length > 0) {
    rows.push({
      kind: "label",
      label: "Recommended",
      itemIndex: -1,
      flags: COMPLETION_ROW_LABEL_RECOMMENDED,
    });
    for (const itemIndex of recommendedOrdered) {
      rows.push({
        kind: "item",
        label: items[itemIndex].label,
        itemIndex,
        flags: COMPLETION_ROW_ITEM_RECOMMENDED,
      });
    }
  }

  let lastKind: CompletionItem["kind"] | null = null;
  for (let i = 0; i < items.length; i += 1) {
    const item = items[i];
    if (item.is_disabled || recommended.has(i)) continue;
    if (item.kind !== lastKind) {
      rows.push({
        kind: "label",
        label: kindLabel(item.kind),
        itemIndex: -1,
        flags: COMPLETION_ROW_LABEL_GROUP,
      });
      lastKind = item.kind;
    }
    rows.push({ kind: "item", label: item.label, itemIndex: i, flags: 0 });
  }

  return rows;
}

export function getSelectedItemIndex(
  rows: CompletionRenderRow[],
  selectedRowIndex: number,
): number | null {
  if (selectedRowIndex < 0 || selectedRowIndex >= rows.length) return null;
  const row = rows[selectedRowIndex];
  return row?.kind === "item" ? row.itemIndex : null;
}

export function normalizeSelectedRowIndex(
  rows: CompletionRenderRow[],
  selectedRowIndex: number,
): number {
  if (rows.length === 0) return -1;
  if (selectedRowIndex >= 0 && selectedRowIndex < rows.length) {
    if (rows[selectedRowIndex].kind === "item") return selectedRowIndex;
    for (let i = selectedRowIndex + 1; i < rows.length; i += 1) {
      if (rows[i].kind === "item") return i;
    }
    for (let i = selectedRowIndex - 1; i >= 0; i -= 1) {
      if (rows[i].kind === "item") return i;
    }
    return -1;
  }
  for (let i = 0; i < rows.length; i += 1) {
    if (rows[i].kind === "item") return i;
  }
  return -1;
}

export function nextSelectedRowIndex(
  rows: CompletionRenderRow[],
  selectedRowIndex: number,
  delta: number,
): number {
  let hasItem = false;
  for (const row of rows) {
    if (row.kind === "item") {
      hasItem = true;
      break;
    }
  }
  if (!hasItem) return -1;
  const direction = delta >= 0 ? 1 : -1;
  let next =
    selectedRowIndex < 0 || selectedRowIndex >= rows.length
      ? direction > 0
        ? -1
        : 0
      : selectedRowIndex;
  for (let i = 0; i < rows.length; i += 1) {
    next = (next + direction + rows.length) % rows.length;
    if (rows[next].kind === "item") return next;
  }
  return -1;
}
