import type { FormulaClientErrorData } from "@notion-formula/sdk";
import { describe, expect, it, vi } from "vitest";
import type { FormulaRequest } from "../../../../packages/notion-formula/dist/protocol.js";
import {
  FormulaWorkerRuntime,
  type FormulaSession,
} from "../../../../packages/notion-formula/dist/worker_runtime.js";

const definition = { id: "f", expression: "1" };
const state = {
  version: 0n,
  definition,
  output_type: "Number" as const,
  diagnostics: [],
  tokens: [],
  property_references: [],
};
const edit = { base_version: 0n, edits: [{ range: { start: 0, end: 1 }, new_text: "2" }] };

function session() {
  return {
    get_property: vi.fn(() => null),
    get_properties: vi.fn(() => []),
    get_state: vi.fn(() => "AllReady" as const),
    required_inputs: vi.fn(() => []),
    upsert: vi.fn(() => ({ affected_formulas: ["f"] })),
    remove: vi.fn(() => null),
    evaluate: vi.fn(() => ({ formulas: new Map() })),
    create_draft: vi.fn(() => 7),
    draft_state: vi.fn(() => state),
    draft_help: vi.fn(() => ({
      base_version: 0n,
      completion: { items: [], replace: { start: 1, end: 1 }, preferred_indices: [] },
      signature_help: null,
    })),
    draft_quick_fixes: vi.fn(() => []),
    draft_format_edits: vi.fn(() => edit),
    draft_update_expression: vi.fn(() => ({ state, cursor: 1 })),
    draft_into_definition: vi.fn(() => definition),
    draft_close: vi.fn(),
    close: vi.fn(),
    free: vi.fn(),
  } satisfies FormulaSession;
}

type Route = { request: FormulaRequest; target: keyof ReturnType<typeof session> };
const routes: Route[] = [
  { request: { id: 1, method: "engine.getProperty", args: ["f"] }, target: "get_property" },
  { request: { id: 1, method: "engine.getProperties", args: [] }, target: "get_properties" },
  { request: { id: 1, method: "engine.getState", args: [] }, target: "get_state" },
  {
    request: { id: 1, method: "engine.upsert", args: [{ Formula: definition }] },
    target: "upsert",
  },
  { request: { id: 1, method: "engine.remove", args: ["f"] }, target: "remove" },
  {
    request: {
      id: 1,
      method: "engine.evaluate",
      args: [
        {
          row_ids: [],
          columns: new Map(),
          runtime: { now: 0n, time_zone: "+00:00" },
          formula_ids: ["f"],
        },
      ],
    },
    target: "evaluate",
  },
  { request: { id: 1, method: "engine.createDraft", args: [definition] }, target: "create_draft" },
  { request: { id: 1, method: "draft.getState", args: [7] }, target: "draft_state" },
  {
    request: { id: 1, method: "draft.help", args: [7, 1, { preferred_limit: 5 }] },
    target: "draft_help",
  },
  {
    request: { id: 1, method: "draft.quickFixes", args: [7, "opaque:diagnostic"] },
    target: "draft_quick_fixes",
  },
  { request: { id: 1, method: "draft.formatEdits", args: [7] }, target: "draft_format_edits" },
  {
    request: { id: 1, method: "draft.updateExpression", args: [7, { Edits: { edit, cursor: 1 } }] },
    target: "draft_update_expression",
  },
  {
    request: { id: 1, method: "draft.intoDefinition", args: [7] },
    target: "draft_into_definition",
  },
  { request: { id: 1, method: "draft.close", args: [7] }, target: "draft_close" },
];

describe("FormulaWorkerRuntime", () => {
  it.each(routes)(
    "routes $request.method directly with its original DTO arguments",
    async ({ request, target }) => {
      const wasm = session();
      const runtime = new FormulaWorkerRuntime(() => Promise.resolve(wasm));
      await runtime.handle({ id: 0, method: "initialize", args: [{ properties: [] }] });
      const response = await runtime.handle(request);
      expect(response.ok).toBe(true);
      expect(wasm[target]).toHaveBeenCalledExactlyOnceWith(...request.args);
      expect(wasm.close).not.toHaveBeenCalled();
      expect(wasm.free).not.toHaveBeenCalled();
    },
  );

  it("holds concurrent requests behind asynchronous initialization", async () => {
    let finish!: (wasm: FormulaSession) => void;
    const initialization = new Promise<FormulaSession>((resolve) => {
      finish = resolve;
    });
    const wasm = session();
    const runtime = new FormulaWorkerRuntime(() => initialization);
    const starting = runtime.handle({ id: 0, method: "initialize", args: [{ properties: [] }] });
    const reading = runtime.handle({ id: 1, method: "engine.getState", args: [] });
    const updating = runtime.handle({
      id: 2,
      method: "engine.upsert",
      args: [{ Formula: definition }],
    });
    await Promise.resolve();
    expect(wasm.get_state).not.toHaveBeenCalled();
    expect(wasm.upsert).not.toHaveBeenCalled();
    finish(wasm);
    await expect(Promise.all([starting, reading, updating])).resolves.toEqual([
      { id: 0, ok: true, value: undefined },
      { id: 1, ok: true, value: "AllReady" },
      { id: 2, ok: true, value: { affected_formulas: ["f"] } },
    ]);
    expect(wasm.get_state.mock.invocationCallOrder[0]).toBeLessThan(
      wasm.upsert.mock.invocationCallOrder[0],
    );
  });

  it("returns asynchronous initialization failures as plain typed data", async () => {
    const runtime = new FormulaWorkerRuntime(() =>
      Promise.reject(new Error("WASM download failed")),
    );
    await expect(
      runtime.handle({ id: 0, method: "initialize", args: [{ properties: [] }] }),
    ).resolves.toEqual({
      id: 0,
      ok: false,
      error: { code: "INITIALIZATION_ERROR", message: "WASM download failed", payload: null },
    });
  });

  it("copies the standard Error fields and native payload before the Worker clones them", async () => {
    const data: FormulaClientErrorData = {
      code: "UPDATE_EXPRESSION",
      message: "Edit is stale",
      payload: { error: "VersionMismatch" },
    };
    const error = Object.assign(new Error(data.message), {
      code: data.code,
      payload: data.payload,
    });
    const wasm = session();
    wasm.draft_update_expression.mockImplementationOnce(() => {
      throw error;
    });
    const runtime = new FormulaWorkerRuntime(() => Promise.resolve(wasm));
    await runtime.handle({ id: 0, method: "initialize", args: [{ properties: [] }] });
    const response = await runtime.handle({
      id: 1,
      method: "draft.updateExpression",
      args: [7, { Replace: "2" }],
    });
    expect(structuredClone(response)).toEqual({ id: 1, ok: false, error: data });
    await expect(runtime.handle({ id: 2, method: "draft.getState", args: [7] })).resolves.toEqual({
      id: 2,
      ok: true,
      value: state,
    });
  });

  it("closes the Rust session before freeing the WASM allocation", async () => {
    const wasm = session();
    const runtime = new FormulaWorkerRuntime(() => Promise.resolve(wasm));
    await runtime.handle({ id: 0, method: "initialize", args: [{ properties: [] }] });
    await expect(runtime.handle({ id: 1, method: "engine.close", args: [] })).resolves.toEqual({
      id: 1,
      ok: true,
      value: undefined,
    });
    expect(wasm.close).toHaveBeenCalledTimes(1);
    expect(wasm.free).toHaveBeenCalledTimes(1);
    expect(wasm.close.mock.invocationCallOrder[0]).toBeLessThan(
      wasm.free.mock.invocationCallOrder[0],
    );
  });

  it("frees the WASM allocation even if session cleanup throws", async () => {
    const wasm = session();
    wasm.close.mockImplementationOnce(() => {
      throw new Error("Cleanup failed");
    });
    const runtime = new FormulaWorkerRuntime(() => Promise.resolve(wasm));
    await runtime.handle({ id: 0, method: "initialize", args: [{ properties: [] }] });
    await expect(
      runtime.handle({ id: 1, method: "engine.close", args: [] }),
    ).resolves.toMatchObject({ id: 1, ok: false, error: { code: "WORKER_FAILURE" } });
    expect(wasm.free).toHaveBeenCalledTimes(1);
  });
});
