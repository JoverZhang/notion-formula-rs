import { describe, expect, it } from "vitest";
import type { SignatureHelpView } from "../../src/analyzer/generated/wasm_dto";
import { planSignatureTokens } from "../../src/model/signature";

function makeSignatureHelp(): SignatureHelpView {
  return {
    signatures: [
      {
        segments: [
          { kind: "Name", text: "if" },
          { kind: "Punct", text: "(" },
          { kind: "Param", name: "condition", ty: "boolean", param_index: 0 },
          { kind: "Punct", text: "," },
          { kind: "Separator", text: " " },
          { kind: "Param", name: "then", ty: "number", param_index: 1 },
          { kind: "Punct", text: "," },
          { kind: "Separator", text: " " },
          { kind: "Param", name: "else", ty: "string", param_index: 2 },
          { kind: "Punct", text: ")" },
          { kind: "Separator", text: " " },
          { kind: "Arrow", text: "->" },
          { kind: "Separator", text: " " },
          { kind: "ReturnType", text: "number | string" },
        ],
      },
    ],
    active_signature: 0,
    active_parameter: 1,
  };
}

describe("signature layout planning", () => {
  it("planSignatureTokens preserves active parameter marks in unwrapped mode", () => {
    const plan = planSignatureTokens(makeSignatureHelp(), "unwrapped");
    expect(plan.mode).toBe("unwrapped");
    expect(plan.tokens.some((token) => token.text === "\n")).toBe(false);

    const activeParams = plan.tokens.filter((token) => token.active);
    expect(activeParams).toHaveLength(1);
    expect(activeParams[0].text).toBe("then: number");
  });

  it("planSignatureTokens emits text markers for wrapped mode", () => {
    const plan = planSignatureTokens(makeSignatureHelp(), "wrapped");
    expect(plan.mode).toBe("wrapped");

    const lineBreaks = plan.tokens.filter((token) => token.text === "\n");
    const indents = plan.tokens.filter((token) => token.text === "  ");
    expect(lineBreaks.length).toBeGreaterThan(0);
    expect(indents.length).toBe(3);

    const commas = plan.tokens.filter((token) => token.text === ",");
    expect(commas).toHaveLength(2);
  });
});
