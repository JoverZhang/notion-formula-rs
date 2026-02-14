import { beforeAll, describe, expect, it } from "vitest";
import { analyzeSource, formatSource, initWasm } from "../../src/analyzer/wasm_client";
import { ANALYZER_CONFIG } from "../../src/app/context";
import * as wasm from "../../src/pkg/analyzer_wasm.js";

beforeAll(async () => {
  await initWasm(ANALYZER_CONFIG);
});

describe("WASM host contract errors", () => {
  it("Analyzer constructor throws on invalid config shape", () => {
    expect(() => new wasm.Analyzer({ functions: [] })).toThrowError(/Invalid analyzer config/);
  });

  it("analyzeSource still succeeds after initialization", () => {
    const out = analyzeSource("1+2");
    expect(out.output_type).toBe("number");
  });

  it("formatSource throws analyzer IDE errors", () => {
    expect(() => formatSource("1 +", 0)).toThrowError(/Format error/);
  });
});
