import type { SignatureHelp } from "../analyzer/wasm_client";

export type SignatureSide = "left" | "right";
export type SignatureWrapMode = "unwrapped" | "wrapped";
export type SignatureToken = { text: string; active: boolean };

type SignatureSegment = SignatureHelp["signatures"][number]["segments"][number];

const segText = (seg: SignatureSegment) =>
  seg.kind === "Ellipsis" ? "..." : seg.kind === "Param" ? `${seg.name}: ${seg.ty}` : seg.text;

const isComma = (seg: SignatureSegment) =>
  (seg.kind === "Punct" && seg.text === ",") ||
  (seg.kind === "Separator" && seg.text.includes(","));

const toToken = (seg: SignatureSegment, activeParameter: number): SignatureToken => ({
  text: segText(seg),
  active: seg.kind === "Param" && seg.param_index === activeParameter,
});

function activeSignature(sig: SignatureHelp) {
  return sig.signatures[sig.active_signature] ?? sig.signatures[0] ?? null;
}

function pushTokens(out: SignatureToken[], segments: SignatureSegment[], activeParameter: number) {
  for (const seg of segments) out.push(toToken(seg, activeParameter));
}

function trimLine(line: SignatureSegment[]): SignatureSegment[] {
  while (line.length && segText(line[0]).trim() === "") line.shift();
  while (line.length && segText(line[line.length - 1]).trim() === "") line.pop();
  return line;
}

function buildUnwrapped(sig: SignatureHelp): SignatureToken[] {
  const current = activeSignature(sig);
  if (!current) return [];
  return current.segments.map((seg) => toToken(seg, sig.active_parameter));
}

function buildWrapped(sig: SignatureHelp): SignatureToken[] {
  const current = activeSignature(sig);
  if (!current) return [];

  const segments = current.segments;
  const arrow = segments.findIndex((seg) => seg.kind === "Arrow");
  const beforeArrow = arrow === -1 ? segments.length : arrow;

  let open = -1;
  for (let i = beforeArrow - 1; i >= 0; i -= 1) {
    const seg = segments[i];
    if (seg.kind === "Punct" && seg.text === "(") {
      open = i;
      break;
    }
  }
  if (open === -1) return buildUnwrapped(sig);

  let close = -1;
  for (let i = open + 1; i < beforeArrow; i += 1) {
    const seg = segments[i];
    if (seg.kind === "Punct" && seg.text === ")") {
      close = i;
      break;
    }
  }
  if (close === -1) return buildUnwrapped(sig);

  const out: SignatureToken[] = [];
  pushTokens(out, segments.slice(0, open + 1), sig.active_parameter);
  out.push({ text: "\n", active: false });

  let line: SignatureSegment[] = [];
  for (const seg of segments.slice(open + 1, close)) {
    line.push(seg);
    if (!isComma(seg)) continue;
    const next = trimLine(line);
    if (next.length) {
      out.push({ text: "  ", active: false });
      pushTokens(out, next, sig.active_parameter);
      out.push({ text: "\n", active: false });
    }
    line = [];
  }

  const tail = trimLine(line);
  if (tail.length) {
    out.push({ text: "  ", active: false });
    pushTokens(out, tail, sig.active_parameter);
    out.push({ text: "\n", active: false });
  }

  pushTokens(out, segments.slice(close), sig.active_parameter);
  return out;
}

export function planSignatureTokens(sig: SignatureHelp, mode: SignatureWrapMode) {
  return { mode, tokens: mode === "wrapped" ? buildWrapped(sig) : buildUnwrapped(sig) };
}

export function computePopoverWidthPx(viewportWidth: number): number {
  return Math.max(240, Math.min(360, viewportWidth * 0.28));
}

export function pickPopoverSide(args: {
  viewportWidth: number;
  wrapLeft: number;
  wrapRight: number;
  gapPx?: number;
  popoverWidthPx: number;
}): SignatureSide {
  const gap = args.gapPx ?? 12;
  const leftSpace = args.wrapLeft - gap;
  const rightSpace = args.viewportWidth - args.wrapRight - gap;
  if (leftSpace >= args.popoverWidthPx) return "left";
  if (rightSpace >= args.popoverWidthPx) return "right";
  return leftSpace >= rightSpace ? "left" : "right";
}

export function shouldUseWrappedSignature(args: {
  scrollWidth: number;
  clientWidth: number;
  epsilon?: number;
}): boolean {
  return args.scrollWidth > args.clientWidth + (args.epsilon ?? 1);
}
