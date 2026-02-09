import type { CompletionItem } from "../analyzer/wasm_client";

export type CompletionRenderRow =
  | { type: "header"; kind: CompletionItem["kind"]; label: string }
  | { type: "section"; section: "recommended"; label: string }
  | {
      type: "category";
      kind: "Function";
      category: NonNullable<CompletionItem["category"]>;
      label: string;
    }
  | {
      type: "item";
      item: CompletionItem;
      itemIndex: number;
      isRecommended?: boolean;
    };

function completionGroupLabel(kind: CompletionItem["kind"]): string {
  switch (kind) {
    case "Function":
      return "Functions";
    case "Builtin":
      return "Built-ins";
    case "Property":
      return "Properties";
    case "Operator":
      return "Operators";
    default:
      return String(kind);
  }
}

function functionCategoryLabel(category: NonNullable<CompletionItem["category"]>): string {
  return `${category} Functions`;
}

export function buildCompletionRows(
  items: CompletionItem[],
  preferredIndices: number[],
): CompletionRenderRow[] {
  const rows: CompletionRenderRow[] = [];
  const categoryOrder: Array<NonNullable<CompletionItem["category"]>> = [
    "General",
    "Text",
    "Number",
    "Date",
    "People",
    "List",
    "Special",
  ];

  const recommendedIndices: number[] = [];
  const recommendedSet = new Set<number>();
  for (const idx of preferredIndices) {
    if (!Number.isInteger(idx)) continue;
    if (idx < 0 || idx >= items.length) continue;
    if (recommendedSet.has(idx)) continue;
    const item = items[idx];
    if (!item || item.is_disabled) continue;
    recommendedSet.add(idx);
    recommendedIndices.push(idx);
  }

  if (recommendedIndices.length > 0) {
    rows.push({ type: "section", section: "recommended", label: "Recommended" });
    for (const itemIndex of recommendedIndices) {
      const item = items[itemIndex];
      if (!item) continue;
      rows.push({ type: "item", item, itemIndex, isRecommended: true });
    }
  }

  let scanIdx = 0;
  while (scanIdx < items.length) {
    const kind = items[scanIdx].kind;

    if (kind === "Function") {
      const functionItems: Array<{ item: CompletionItem; itemIndex: number }> = [];
      while (scanIdx < items.length && items[scanIdx].kind === "Function") {
        if (!recommendedSet.has(scanIdx)) {
          functionItems.push({ item: items[scanIdx], itemIndex: scanIdx });
        }
        scanIdx += 1;
      }

      if (functionItems.length === 0) {
        continue;
      }

      rows.push({ type: "header", kind, label: completionGroupLabel(kind) });

      const groups = new Map<NonNullable<CompletionItem["category"]>, typeof functionItems>();
      for (const category of categoryOrder) groups.set(category, []);

      for (const row of functionItems) {
        const category = (row.item.category ?? "General") as NonNullable<
          CompletionItem["category"]
        >;
        const group = groups.get(category);
        if (group) group.push(row);
      }

      for (const category of categoryOrder) {
        const group = groups.get(category);
        if (!group || group.length === 0) continue;
        rows.push({
          type: "category",
          kind: "Function",
          category,
          label: functionCategoryLabel(category),
        });
        group.forEach(({ item, itemIndex }) => rows.push({ type: "item", item, itemIndex }));
      }

      continue;
    }

    const segmentItems: Array<{ item: CompletionItem; itemIndex: number }> = [];
    while (scanIdx < items.length && items[scanIdx].kind === kind) {
      if (!recommendedSet.has(scanIdx)) {
        segmentItems.push({ item: items[scanIdx], itemIndex: scanIdx });
      }
      scanIdx += 1;
    }

    if (segmentItems.length === 0) {
      continue;
    }

    rows.push({ type: "header", kind, label: completionGroupLabel(kind) });
    segmentItems.forEach(({ item, itemIndex }) => rows.push({ type: "item", item, itemIndex }));
  }

  return rows;
}

export function getSelectedItemIndex(
  rows: CompletionRenderRow[],
  selectedRowIndex: number,
): number | null {
  if (selectedRowIndex < 0 || selectedRowIndex >= rows.length) return null;
  const row = rows[selectedRowIndex];
  if (!row || row.type !== "item") return null;
  return row.itemIndex;
}

export function normalizeSelectedRowIndex(
  rows: CompletionRenderRow[],
  selectedRowIndex: number,
): number {
  if (rows.length === 0) {
    return -1;
  }
  if (selectedRowIndex < 0 || selectedRowIndex >= rows.length) {
    return rows.findIndex((row) => row.type === "item");
  }

  if (rows[selectedRowIndex]?.type === "item") {
    return selectedRowIndex;
  }

  const forward = rows.findIndex((row, idx) => idx > selectedRowIndex && row.type === "item");
  if (forward !== -1) return forward;

  const backward = [...rows]
    .map((row, idx) => ({ row, idx }))
    .reverse()
    .find(({ row, idx }) => idx < selectedRowIndex && row.type === "item")?.idx;

  return typeof backward === "number" ? backward : -1;
}

export function nextSelectedRowIndex(
  rows: CompletionRenderRow[],
  selectedRowIndex: number,
  delta: number,
): number {
  const itemRowIndices = rows
    .map((row, idx) => (row.type === "item" ? idx : -1))
    .filter((idx) => idx !== -1);
  if (itemRowIndices.length === 0) return -1;

  if (selectedRowIndex < 0) {
    return delta > 0 ? itemRowIndices[0] : itemRowIndices[itemRowIndices.length - 1];
  }

  const dir = delta >= 0 ? 1 : -1;
  let next = selectedRowIndex;
  for (let i = 0; i < rows.length; i += 1) {
    next = (next + dir + rows.length) % rows.length;
    if (rows[next]?.type === "item") {
      return next;
    }
  }

  return selectedRowIndex;
}
