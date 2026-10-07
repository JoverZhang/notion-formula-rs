import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import type { CursorHelp } from "@notion-formula/sdk";
import init, { FormulaEngineSession } from "@notion-formula/sdk/wasm";
import { beforeAll, describe, expect, it } from "vitest";

function help(source: string, cursor: number): CursorHelp {
  const session = new FormulaEngineSession({ properties: [] });
  const handle = session.create_draft({ id: "signature-test", expression: source });
  try {
    return session.draft_help(handle, cursor, { preferred_limit: 30 }) as CursorHelp;
  } finally {
    session.draft_close(handle);
    session.close();
    session.free();
  }
}

function sigLabelAtCloseParen(source: string): string {
  const cursor = source.lastIndexOf(")");
  expect(cursor).toBeGreaterThanOrEqual(0);

  const out = help(source, cursor);
  expect(out.signature_help).not.toBeNull();
  const sigHelp = out.signature_help!;
  const activeSig = sigHelp.signatures[sigHelp.active_signature] ?? sigHelp.signatures[0];
  expect(activeSig).toBeTruthy();
  return activeSig.segments
    .map((s) => {
      switch (s.kind) {
        case "Ellipsis":
          return "...";
        case "Param":
          return `${s.name}: ${s.ty}`;
        default:
          return s.text;
      }
    })
    .join("");
}

function sigAtCloseParen(source: string) {
  const cursor = source.lastIndexOf(")");
  expect(cursor).toBeGreaterThanOrEqual(0);

  const out = help(source, cursor);
  expect(out.signature_help).not.toBeNull();
  return out.signature_help!;
}

beforeAll(async () => {
  const wasmBytes = await readFile(
    fileURLToPath(
      new URL(
        "../../node_modules/@notion-formula/sdk/dist/wasm/analyzer_wasm_bg.wasm",
        import.meta.url,
      ),
    ),
  );
  await init({ module_or_path: wasmBytes });
});

describe("WASM signature help (instantiated types)", () => {
  it("sum() shows variadic number signature and highlights first arg", () => {
    const sig = sigAtCloseParen("sum()");
    expect(sigLabelAtCloseParen("sum()")).toBe("sum(values1: number | number[], ...) -> number");
    expect(sig.active_parameter).toBe(0);
  });

  it("sum(42) highlights first arg", () => {
    const sig = sigAtCloseParen("sum(42)");
    expect(sigLabelAtCloseParen("sum(42)")).toBe("sum(values1: number, ...) -> number");
    expect(sig.active_parameter).toBe(0);
  });

  it("sum(42, <empty>) highlights second arg", () => {
    const sig = sigAtCloseParen("sum(42, )");
    expect(sigLabelAtCloseParen("sum(42, )")).toBe(
      "sum(values1: number, values2: number | number[], ...) -> number",
    );
    expect(sig.active_parameter).toBe(1);
  });

  it("sum(42, 42) highlights second arg", () => {
    const sig = sigAtCloseParen("sum(42, 42)");
    expect(sigLabelAtCloseParen("sum(42, 42)")).toBe(
      "sum(values1: number, values2: number, ...) -> number",
    );
    expect(sig.active_parameter).toBe(1);
  });

  it("if(true, unknown, 1) -> unknown", () => {
    expect(sigLabelAtCloseParen("if(true, x, 1)")).toBe(
      "if(condition: boolean, then: unknown, else: number) -> unknown",
    );
  });

  it('if(true, 1, "x") -> number | string', () => {
    expect(sigLabelAtCloseParen('if(true, 1, "x")')).toBe(
      "if(condition: boolean, then: number, else: string) -> number | string",
    );
  });

  it('if(true, 42, [42, "42"]) list-of-union is parenthesized', () => {
    const sig = sigAtCloseParen('if(true, 42, [42, "42"])');
    expect(sigLabelAtCloseParen('if(true, 42, [42, "42"])')).toBe(
      "if(condition: boolean, then: number, else: (number | string)[]) -> number | (number | string)[]",
    );
    expect(sig.active_parameter).toBe(2);
  });

  it('ifs(true, 1, false, 2, "a") -> number | string', () => {
    expect(sigLabelAtCloseParen('ifs(true, 1, false, 2, "a")')).toBe(
      "ifs(condition1: boolean, value1: number, condition2: boolean, value2: number, ..., else: string?) -> number | string",
    );
  });

  it("ifs(true, unknown, false, 1, 2) -> unknown", () => {
    expect(sigLabelAtCloseParen("ifs(true, x, false, 1, 2)")).toBe(
      "ifs(condition1: boolean, value1: unknown, condition2: boolean, value2: number, ..., else: number?) -> unknown",
    );
  });

  it("ifs(..., <empty>) highlights else (tail)", () => {
    const sig = sigAtCloseParen('ifs(true, "123", true, "123", )');
    expect(sigLabelAtCloseParen('ifs(true, "123", true, "123", )')).toBe(
      "ifs(condition1: boolean, value1: string, condition2: boolean, value2: string, ..., else: string?) -> string",
    );
    expect(sig.active_parameter).toBe(4);
  });

  it("ifs(..., true, <empty>) highlights value in repeat pair", () => {
    const sig = sigAtCloseParen('ifs(true, "123", true, "123", true, )');
    expect(sig.active_parameter).toBe(5);
  });
});
