import type {
  decodeFormulaString as decodeFormulaStringDeclaration,
  quoteFormulaString as quoteFormulaStringDeclaration,
} from "./client.h.js";

export const decodeFormulaString: typeof decodeFormulaStringDeclaration = (literal) => {
  if (literal.length < 2 || literal[0] !== '"' || literal[literal.length - 1] !== '"')
    return null;

  let value = "";
  const end = literal.length - 1;
  for (let index = 1; index < end; index++) {
    const character = literal[index];
    if (character === '"') return null;
    if (character !== "\\") {
      value += character;
      continue;
    }
    if (++index >= end) return null;
    switch (literal[index]) {
      case "n":
        value += "\n";
        break;
      case "t":
        value += "\t";
        break;
      case '"':
        value += '"';
        break;
      case "\\":
        value += "\\";
        break;
      default:
        return null;
    }
  }
  return value;
};

export const quoteFormulaString: typeof quoteFormulaStringDeclaration = (value) =>
  `"${value.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n").replace(/\t/g, "\\t")}"`;
