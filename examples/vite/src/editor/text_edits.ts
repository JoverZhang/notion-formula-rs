import type { CompletionItemView, TextEditView, Utf16Span } from "../analyzer/generated/wasm_dto";

export type TextRange = Utf16Span;

export type TextEdit = TextEditView;
export type CompletionItem = CompletionItemView;

function compareEditsDesc(a: TextEdit, b: TextEdit): number {
  return b.range.start - a.range.start || b.range.end - a.range.end;
}

export function applyTextEdits(text: string, edits: TextEdit[]): string {
  if (!edits.length) return text;
  const sorted = [...edits].sort(compareEditsDesc);
  let updated = text;

  for (const edit of sorted) {
    const start = edit.range.start;
    const end = edit.range.end;
    if (start < 0 || end < start || end > updated.length) continue;
    updated = updated.slice(0, start) + edit.new_text + updated.slice(end);
  }

  return updated;
}

export function applyCompletion(
  text: string,
  item: Pick<CompletionItem, "primary_edit" | "additional_edits" | "cursor">,
): { newText: string; newCursor: number } {
  const primary = item.primary_edit;
  if (!primary) return { newText: text, newCursor: 0 };

  const edits = [primary, ...(item.additional_edits ?? [])];
  const newText = applyTextEdits(text, edits);
  const fallbackCursor = primary.range.start + primary.new_text.length;
  const newCursor = Math.max(0, Math.min(item.cursor ?? fallbackCursor, newText.length));
  return { newText, newCursor };
}
