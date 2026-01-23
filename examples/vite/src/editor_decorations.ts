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
    const aStart = a.span?.start ?? Number.MAX_SAFE_INTEGER;
    const bStart = b.span?.start ?? Number.MAX_SAFE_INTEGER;
    if (aStart !== bStart) return aStart - bStart;
    const aEnd = a.span?.end ?? Number.MAX_SAFE_INTEGER;
    const bEnd = b.span?.end ?? Number.MAX_SAFE_INTEGER;
    return aEnd - bEnd;
  });
}

export function computePropChips(source: string, tokens: Token[]): Chip[] {
  const chips: Chip[] = [];
  if (!tokens || tokens.length === 0) {
    return chips;
  }

  const sortedTokens = sortTokens(tokens);
  for (let i = 0; i < sortedTokens.length; i += 1) {
    const ident = sortedTokens[i];
    if (!ident || ident.kind !== "Ident" || ident.text !== "prop") {
      continue;
    }
    const identStart = ident.span?.start;
    const identEnd = ident.span?.end;
    if (typeof identStart !== "number" || typeof identEnd !== "number") {
      continue;
    }

    let j = i + 1;
    while (j < sortedTokens.length && isTriviaKind(sortedTokens[j].kind ?? "")) {
      j += 1;
    }
    const openParen = sortedTokens[j];
    if (!openParen || openParen.kind !== "OpenParen") {
      continue;
    }

    let k = j + 1;
    while (k < sortedTokens.length && isTriviaKind(sortedTokens[k].kind ?? "")) {
      k += 1;
    }
    const stringToken = sortedTokens[k];
    if (!stringToken || stringToken.kind !== "String") {
      continue;
    }

    let l = k + 1;
    while (l < sortedTokens.length && isTriviaKind(sortedTokens[l].kind ?? "")) {
      l += 1;
    }
    const closeParen = sortedTokens[l];
    if (!closeParen || closeParen.kind !== "CloseParen") {
      continue;
    }

    const stringStart = stringToken.span?.start;
    const stringEnd = stringToken.span?.end;
    const closeEnd = closeParen.span?.end;
    if (
      typeof stringStart !== "number" ||
      typeof stringEnd !== "number" ||
      typeof closeEnd !== "number"
    ) {
      continue;
    }
    if (closeEnd <= stringEnd) {
      continue;
    }

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

    i = l;
  }

  return chips;
}

export type TokenDecorationRange = {
  from: number;
  to: number;
  className: string;
};

export function computeTokenDecorationRanges(
  docLen: number,
  tokens: Token[],
): TokenDecorationRange[] {
  if (!tokens || tokens.length === 0) {
    return [];
  }

  const ranges: TokenDecorationRange[] = [];

  for (const token of tokens) {
    if (!token || token.kind === "Eof") {
      continue;
    }
    const start = token.span?.start;
    const end = token.span?.end;
    if (typeof start !== "number" || typeof end !== "number") {
      continue;
    }
    if (start === end) {
      continue;
    }
    if (start < 0 || end < 0 || end < start || start > docLen || end > docLen) {
      continue;
    }

    ranges.push({
      from: start,
      to: end,
      className: `tok tok-${token.kind}`,
    });
  }

  return ranges;
}

export type TokenSpanIssues = {
  outOfBounds: boolean;
  overlap: boolean;
};

export function getTokenSpanIssues(docLen: number, tokens: Token[]): TokenSpanIssues {
  if (!tokens || tokens.length === 0) {
    return { outOfBounds: false, overlap: false };
  }
  let outOfBounds = false;
  const spans: { start: number; end: number }[] = [];
  for (const token of tokens) {
    if (!token || token.kind === "Eof") {
      continue;
    }
    const start = token.span?.start;
    const end = token.span?.end;
    if (typeof start !== "number" || typeof end !== "number") {
      continue;
    }
    if (start < 0 || end < 0 || end < start || start > docLen || end > docLen) {
      outOfBounds = true;
      continue;
    }
    if (start === end) {
      continue;
    }
    spans.push({ start, end });
  }

  spans.sort((a, b) => a.start - b.start || a.end - b.end);
  let overlap = false;
  for (let i = 1; i < spans.length; i += 1) {
    if (spans[i].start < spans[i - 1].end) {
      overlap = true;
      break;
    }
  }

  return { outOfBounds, overlap };
}
