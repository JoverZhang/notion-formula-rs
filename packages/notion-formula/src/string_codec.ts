import type {
  decodeFormulaString as decodeFormulaStringDeclaration,
  encodeFormulaString as encodeFormulaStringDeclaration,
} from "./client.h.js";

export const decodeFormulaString: typeof decodeFormulaStringDeclaration = (literal) =>
  literal
    .slice(1, -1)
    .replace(/\\([\s\S])/gu, (_, character: string) =>
      character === "n" ? "\n" : character === "t" ? "\t" : character,
    );

export const encodeFormulaString: typeof encodeFormulaStringDeclaration = (value) =>
  `"${value.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n").replace(/\t/g, "\\t")}"`;
