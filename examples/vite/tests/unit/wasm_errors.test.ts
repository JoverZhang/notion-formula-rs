import { beforeAll, describe, expect, it } from "vitest";
import { analyze, format, initWasm } from "../../src/analyzer/wasm_client";
import { ANALYZER_CONFIG } from "../../src/app/context";
import * as wasm from "../../src/pkg/analyzer_wasm.js";

beforeAll(async () => {
  await initWasm(ANALYZER_CONFIG);
});

describe("WASM host contract errors", () => {
  it("Analyzer constructor throws on invalid config shape", () => {
    expect(() => new wasm.Analyzer({ functions: [] })).toThrowError(/Invalid analyzer config/);
  });

  it("analyze still succeeds after initialization", () => {
    const out = analyze("1+2");
    expect(out.output_type).toBe("number");
  });

  it("format throws analyzer IDE errors", () => {
    expect(() => format("1 +", 0)).toThrowError(/Format error/);
  });
});
