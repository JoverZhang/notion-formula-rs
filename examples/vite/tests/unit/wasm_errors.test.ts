import { beforeAll, describe, expect, it } from "vitest";
import { analyzeSource, helpSource, initWasm } from "../../src/analyzer/wasm_client";

beforeAll(async () => {
  await initWasm();
});

describe("WASM host contract errors", () => {
  it("analyzeSource throws an Error for invalid context JSON", () => {
    try {
      analyzeSource("1+2", "{");
      throw new Error("expected analyzeSource to throw");
    } catch (e) {
      expect(e).toBeInstanceOf(Error);
      expect((e as Error).message).toContain("Invalid context JSON");
    }
  });

  it("helpSource throws an Error for invalid context JSON", () => {
    try {
      helpSource("1+2", 0, "{");
      throw new Error("expected helpSource to throw");
    } catch (e) {
      expect(e).toBeInstanceOf(Error);
      expect((e as Error).message).toContain("Invalid context JSON");
    }
  });

  it("analyzeSource throws an Error when context JSON contains functions", () => {
    try {
      analyzeSource("1+2", JSON.stringify({ functions: [] }));
      throw new Error("expected analyzeSource to throw");
    } catch (e) {
      expect(e).toBeInstanceOf(Error);
      expect((e as Error).message).toContain("Invalid context JSON");
    }
  });
});
