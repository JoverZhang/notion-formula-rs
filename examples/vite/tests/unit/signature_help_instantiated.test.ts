import { beforeAll, describe, expect, it } from "vitest";
import { completeSource, initWasm } from "../../src/analyzer/wasm_client";

const contextJson = JSON.stringify({ properties: [] });

function sigLabelAtCloseParen(source: string): string {
  const cursor = source.lastIndexOf(")");
  expect(cursor).toBeGreaterThanOrEqual(0);

  const out = completeSource(source, cursor, contextJson);
  expect(out.signature_help).not.toBeNull();
  return out.signature_help!.label;
}

function sigAtCloseParen(source: string) {
  const cursor = source.lastIndexOf(")");
  expect(cursor).toBeGreaterThanOrEqual(0);

  const out = completeSource(source, cursor, contextJson);
  expect(out.signature_help).not.toBeNull();
  return out.signature_help!;
}

beforeAll(async () => {
  await initWasm();
});

describe("WASM signature help (instantiated types)", () => {
  // TODO: restore `number[]` support for `sum` once list literals or an equivalent array expression exists.
  it("sum() shows variadic number signature and highlights first arg", () => {
    const sig = sigAtCloseParen("sum()");
    expect(sig.label).toBe("sum(values1: number | number[], ...) -> number");
    expect(sig.active_param).toBe(0);
  });

  it("sum(42) highlights first arg", () => {
    const sig = sigAtCloseParen("sum(42)");
    expect(sig.label).toBe("sum(values1: number | number[], ...) -> number");
    expect(sig.active_param).toBe(0);
  });

  it("sum(42, <empty>) highlights second arg", () => {
    const sig = sigAtCloseParen("sum(42, )");
    expect(sig.label).toBe(
      "sum(values1: number | number[], values2: number | number[], ...) -> number",
    );
    expect(sig.active_param).toBe(1);
  });

  it("sum(42, 42) highlights second arg", () => {
    const sig = sigAtCloseParen("sum(42, 42)");
    expect(sig.label).toBe(
      "sum(values1: number | number[], values2: number | number[], ...) -> number",
    );
    expect(sig.active_param).toBe(1);
  });

  it("if(true, unknown, 1) -> unknown", () => {
    expect(sigLabelAtCloseParen("if(true, x, 1)")).toBe(
      "if(condition: boolean, then: unknown, else: number) -> unknown",
    );
  });

  it("if(true, 1, \"x\") -> number | string", () => {
    expect(sigLabelAtCloseParen('if(true, 1, "x")')).toBe(
      "if(condition: boolean, then: number, else: string) -> number | string",
    );
  });

  it("ifs(true, 1, false, 2, \"a\") -> number | string", () => {
    expect(sigLabelAtCloseParen('ifs(true, 1, false, 2, "a")')).toBe(
      "ifs(condition1: boolean, value1: number, condition2: boolean, value2: number, ..., default: string) -> number | string",
    );
  });

  it("ifs(true, unknown, false, 1, 2) -> unknown", () => {
    expect(sigLabelAtCloseParen("ifs(true, x, false, 1, 2)")).toBe(
      "ifs(condition1: boolean, value1: unknown, condition2: boolean, value2: number, ..., default: number) -> unknown",
    );
  });

  it("ifs(..., <empty>) highlights default (tail)", () => {
    const sig = sigAtCloseParen('ifs(true, "123", true, "123", )');
    expect(sig.label).toBe(
      "ifs(condition1: boolean, value1: string, condition2: boolean, value2: string, ..., default: string) -> string",
    );
    expect(sig.active_param).toBe(5);
  });

  it("ifs(..., true, <empty>) highlights value in repeat pair", () => {
    const sig = sigAtCloseParen('ifs(true, "123", true, "123", true, )');
    expect(sig.active_param).toBe(3);
  });
});
