import type {
  decodeFormulaString as decodeFormulaStringDeclaration,
  encodeFormulaString as encodeFormulaStringDeclaration,
} from "./client.h.js";

const escapes = { n: "\n", t: "\t", '"': '"', "\\": "\\" };

export const decodeFormulaString: typeof decodeFormulaStringDeclaration = (literal) =>
  literal
    .slice(1, -1)
    .replace(/\\([nt"\\])/g, (_, escape: keyof typeof escapes) => escapes[escape]);

export const encodeFormulaString: typeof encodeFormulaStringDeclaration = (value) =>
  `"${value.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n").replace(/\t/g, "\\t")}"`;
