import { StateEffect, StateField } from "@codemirror/state";
import { Decoration, DecorationSet, EditorView } from "@codemirror/view";
import type { TokenView } from "./analyzer/generated/wasm_dto";

export type Token = TokenView;

export type Chip = {
  spanStart: number;
  spanEnd: number;
  argContentStart: number;
  argContentEnd: number;
  argValue: string;
};

export const setTokenDecoListEffect = StateEffect.define<DecorationSet>();

export const tokenDecoStateField = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },
  update(value, tr) {
    let token = value;
    for (const effect of tr.effects) {
      if (effect.is(setTokenDecoListEffect)) {
        token = effect.value;
      }
    }

    if (tr.docChanged) {
      token = token.map(tr.changes);
    }

    return token;
  },
  provide: (field) => EditorView.decorations.from(field),
});

function isTriviaKind(kind: string): boolean {
  return (
    kind === "DocComment" || kind === "LineComment" || kind === "BlockComment" || kind === "Newline"
  );
}

export function sortTokens(tokens: Token[]): Token[] {
  return [...tokens].sort((a, b) => {
    const aStart = a.span?.range?.start ?? Number.MAX_SAFE_INTEGER;
    const bStart = b.span?.range?.start ?? Number.MAX_SAFE_INTEGER;
    if (aStart !== bStart) return aStart - bStart;
    const aEnd = a.span?.range?.end ?? Number.MAX_SAFE_INTEGER;
    const bEnd = b.span?.range?.end ?? Number.MAX_SAFE_INTEGER;
    return aEnd - bEnd;
  });
}

function nextNonTrivia(tokens: Token[], from: number): number {
  let index = from;
  while (index < tokens.length && isTriviaKind(tokens[index].kind ?? "")) index += 1;
  return index;
}

export function computePropChips(source: string, tokens: Token[]): Chip[] {
  if (!tokens || tokens.length === 0) return [];
  const chips: Chip[] = [];
  const sortedTokens = sortTokens(tokens);
  for (let i = 0; i < sortedTokens.length; i += 1) {
    const ident = sortedTokens[i];
    if (!ident || ident.kind !== "Ident" || ident.text !== "prop") continue;
    const identStart = ident.span?.range?.start;
    if (typeof identStart !== "number") continue;

    const openIndex = nextNonTrivia(sortedTokens, i + 1);
    const openParen = sortedTokens[openIndex];
    if (!openParen || openParen.kind !== "OpenParen") continue;

    const stringIndex = nextNonTrivia(sortedTokens, openIndex + 1);
    const stringToken = sortedTokens[stringIndex];
    if (!stringToken || stringToken.kind !== "String") continue;

    const closeIndex = nextNonTrivia(sortedTokens, stringIndex + 1);
    const closeParen = sortedTokens[closeIndex];
    if (!closeParen || closeParen.kind !== "CloseParen") continue;

    const stringStart = stringToken.span?.range?.start;
    const stringEnd = stringToken.span?.range?.end;
    const closeEnd = closeParen.span?.range?.end;
    if (typeof stringStart !== "number" || typeof stringEnd !== "number") continue;
    if (typeof closeEnd !== "number" || closeEnd <= stringEnd) continue;

    const rawText = stringToken.text ?? source.slice(stringStart, stringEnd);
    const hasQuotes = rawText.startsWith('"') && rawText.endsWith('"');
    const argContentStart = stringStart + (hasQuotes ? 1 : 0);
    const argContentEnd = stringEnd - (hasQuotes ? 1 : 0);
    const argValue = source.slice(argContentStart, argContentEnd);

    chips.push({
      spanStart: identStart,
      spanEnd: closeEnd,
      argContentStart,
      argContentEnd,
      argValue,
    });

    i = closeIndex;
  }
  return chips;
}

export type TokenDecorationRange = {
  from: number;
  to: number;
  className: string;
};

type TokenSpan = { start: number; end: number; kind: string };

function scanTokenSpans(
  docLen: number,
  tokens: Token[],
): { spans: TokenSpan[]; outOfBounds: boolean } {
  const spans: TokenSpan[] = [];
  let outOfBounds = false;
  for (const token of tokens) {
    if (!token || token.kind === "Eof") continue;
    const start = token.span?.range?.start;
    const end = token.span?.range?.end;
    if (typeof start !== "number" || typeof end !== "number") continue;
    if (start === end) continue;
    if (start < 0 || end < 0 || end < start || start > docLen || end > docLen) {
      outOfBounds = true;
      continue;
    }
    spans.push({ start, end, kind: token.kind });
  }
  return { spans, outOfBounds };
}

export function computeTokenDecorationRanges(
  docLen: number,
  tokens: Token[],
): TokenDecorationRange[] {
  if (!tokens || tokens.length === 0) return [];
  const { spans } = scanTokenSpans(docLen, tokens);
  return spans.map((span) => ({
    from: span.start,
    to: span.end,
    className: `tok tok-${span.kind}`,
  }));
}

export type TokenSpanIssues = {
  outOfBounds: boolean;
  overlap: boolean;
};

export function getTokenSpanIssues(docLen: number, tokens: Token[]): TokenSpanIssues {
  if (!tokens || tokens.length === 0) return { outOfBounds: false, overlap: false };
  const { spans, outOfBounds } = scanTokenSpans(docLen, tokens);
  spans.sort((a, b) => a.start - b.start || a.end - b.end);
  for (let i = 1; i < spans.length; i += 1) {
    if (spans[i].start < spans[i - 1].end) {
      return { outOfBounds, overlap: true };
    }
  }
  return { outOfBounds, overlap: false };
}
