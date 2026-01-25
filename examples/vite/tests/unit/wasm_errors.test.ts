import { beforeAll, describe, expect, it } from "vitest";
import { analyzeSource, completeSource, initWasm } from "../../src/analyzer/wasm_client";

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

  it("completeSource throws an Error for invalid context JSON", () => {
    try {
      completeSource("1+2", 0, "{");
      throw new Error("expected completeSource to throw");
    } catch (e) {
      expect(e).toBeInstanceOf(Error);
      expect((e as Error).message).toContain("Invalid context JSON");
    }
  });
});
