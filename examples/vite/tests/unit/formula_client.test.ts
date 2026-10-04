import { describe, expect, it, vi } from "vitest";
import {
  createFormulaEngineClient,
  FormulaClientError,
  type Column,
  type CursorHelp,
  type EvaluateInput,
  type FormulaClientErrorData,
  type FormulaDraftState,
  type FormulaWorker,
} from "../../src/formula/client";
import type { FormulaRequest, FormulaResponse } from "../../src/formula/protocol";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function success(request: FormulaRequest, value: unknown = undefined): FormulaResponse {
  return { id: request.id, ok: true, value };
}

function failure(request: FormulaRequest, error: FormulaClientErrorData): FormulaResponse {
  return { id: request.id, ok: false, error };
}

// Both directions clone just as a real Worker does, including special numbers and bigint.
class ControlledWorker extends EventTarget {
  readonly requests: FormulaRequest[] = [];
  readonly terminate = vi.fn();

  constructor(
    private readonly respond: (
      request: FormulaRequest,
    ) => FormulaResponse | Promise<FormulaResponse>,
  ) {
    super();
  }

  postMessage(message: unknown): void {
    const request = structuredClone(message) as FormulaRequest;
    this.requests.push(request);
    void Promise.resolve()
      .then(() => this.respond(request))
      .then((response) => {
        this.dispatchEvent(new MessageEvent("message", { data: structuredClone(response) }));
      })
      .catch((error: unknown) =>
        this.crash(error instanceof Error ? error.message : String(error)),
      );
  }

  crash(message: string): void {
    const event = new Event("error");
    Object.defineProperty(event, "message", { value: message });
    this.dispatchEvent(event);
  }
}

async function setup(
  respond: (request: FormulaRequest) => FormulaResponse | Promise<FormulaResponse>,
) {
  const worker = new ControlledWorker((request) =>
    request.method === "initialize" || request.method === "engine.close"
      ? success(request)
      : respond(request),
  );
  const engine = await createFormulaEngineClient(
    { properties: [] },
    {
      workerFactory: () => worker as unknown as FormulaWorker,
    },
  );
  return { engine, worker };
}

const draftState: FormulaDraftState = {
  version: 0n,
  definition: { id: "f", expression: "1" },
  output_type: "Number",
  diagnostics: [],
  tokens: [],
};

const cursorHelp: CursorHelp = {
  base_version: 0n,
  completion: { items: [], replace: { start: 1, end: 1 }, preferred_indices: [] },
  signature_help: null,
};

describe("FormulaEngineClient queue and lifecycle", () => {
  it("preserves each upsert's definition when the caller reuses and mutates it", async () => {
    let expression: string | null = null;
    const { engine, worker } = await setup((request) => {
      if (request.method === "engine.upsert" && "Formula" in request.args[0]) {
        const next = request.args[0].Formula.expression;
        const changed = next !== expression;
        expression = next;
        return success(request, { affected_formulas: changed ? ["f"] : [] });
      }
      if (request.method === "engine.getProperty")
        return success(request, {
          Formula: {
            definition: { id: "f", expression },
            status: { Ready: { output_type: "Number" } },
          },
        });
      throw new Error("Unexpected request");
    });
    const property = { Formula: { id: "f", expression: "1" } };
    const first = engine.upsert(property);
    const between = engine.getProperty("f");
    property.Formula.expression = "2";
    const second = engine.upsert(property);
    property.Formula.expression = "3";
    await expect(Promise.all([first, between, second])).resolves.toEqual([
      { affected_formulas: ["f"] },
      {
        Formula: {
          definition: { id: "f", expression: "1" },
          status: { Ready: { output_type: "Number" } },
        },
      },
      { affected_formulas: ["f"] },
    ]);
    expect(expression).toBe("2");
    expect(worker.requests.filter((request) => request.method === "engine.upsert")).toEqual([
      { id: 1, method: "engine.upsert", args: [{ Formula: { id: "f", expression: "1" } }] },
      { id: 3, method: "engine.upsert", args: [{ Formula: { id: "f", expression: "2" } }] },
    ]);
    await engine.close();
  });

  it("snapshots the schema before asynchronous initialization starts", async () => {
    const worker = new ControlledWorker((request) => success(request));
    const schema = { properties: [{ Formula: { id: "f", expression: "1" } }] };
    const starting = createFormulaEngineClient(schema, {
      workerFactory: () => worker as unknown as FormulaWorker,
    });
    schema.properties[0].Formula.expression = "2";
    schema.properties.push({ Formula: { id: "g", expression: "3" } });
    const engine = await starting;
    expect(worker.requests[0]?.args).toEqual([
      { properties: [{ Formula: { id: "f", expression: "1" } }] },
    ]);
    await engine.close();
  });

  it("snapshots draft definitions, completion config and nested versioned edits", async () => {
    const { engine, worker } = await setup((request) => {
      if (request.method === "engine.createDraft") return success(request, 7);
      if (request.method === "draft.help") return success(request, cursorHelp);
      if (request.method === "draft.updateExpression")
        return success(request, { state: draftState, cursor: 1 });
      throw new Error("Unexpected request");
    });
    const definition = { id: "f", expression: "1" };
    const creating = engine.createDraft(definition);
    definition.expression = "2";
    const draft = await creating;
    const config = { preferred_limit: 5 };
    const help = draft.help(1, config);
    config.preferred_limit = 0;
    const update = {
      Edits: {
        edit: {
          base_version: 0n,
          edits: [{ range: { start: 0, end: 1 }, new_text: "2" }],
        },
        cursor: 1,
      },
    };
    const first = draft.updateExpression(update);
    update.Edits.edit.base_version = 1n;
    update.Edits.edit.edits[0].range.start = 1;
    update.Edits.edit.edits[0].range.end = 2;
    update.Edits.edit.edits[0].new_text = "3";
    update.Edits.cursor = 2;
    const second = draft.updateExpression(update);
    update.Edits.edit.edits.length = 0;
    await Promise.all([help, first, second]);
    expect(worker.requests.slice(1).map((request) => request.args)).toEqual([
      [{ id: "f", expression: "1" }],
      [7, 1, { preferred_limit: 5 }],
      [
        7,
        {
          Edits: {
            edit: {
              base_version: 0n,
              edits: [{ range: { start: 0, end: 1 }, new_text: "2" }],
            },
            cursor: 1,
          },
        },
      ],
      [
        7,
        {
          Edits: {
            edit: {
              base_version: 1n,
              edits: [{ range: { start: 1, end: 2 }, new_text: "3" }],
            },
            cursor: 2,
          },
        },
      ],
    ]);
    await engine.close();
  });

  it("serializes engine and draft calls in one FIFO queue", async () => {
    const blocked = deferred<FormulaResponse>();
    const { engine, worker } = await setup((request) => {
      if (request.method === "engine.createDraft") return success(request, 7);
      if (request.method === "engine.getState") return blocked.promise;
      if (request.method === "draft.help") return success(request, cursorHelp);
      if (request.method === "engine.getProperty") return success(request, null);
      throw new Error("Unexpected request");
    });
    const draft = await engine.createDraft(draftState.definition);
    const first = engine.getState();
    const second = draft.help(1, { preferred_limit: 5 });
    const third = engine.getProperty("absent");
    await vi.waitFor(() => expect(worker.requests).toHaveLength(3));
    expect(worker.requests[2]?.method).toBe("engine.getState");
    blocked.resolve(success(worker.requests[2], "AllReady"));
    await expect(Promise.all([first, second, third])).resolves.toEqual([
      "AllReady",
      cursorHelp,
      null,
    ]);
    expect(worker.requests.map((request) => request.method)).toEqual([
      "initialize",
      "engine.createDraft",
      "engine.getState",
      "draft.help",
      "engine.getProperty",
    ]);
    expect(worker.requests[3]?.args).toEqual([7, 1, { preferred_limit: 5 }]);
    await engine.close();
  });

  it("drains queued calls before releasing and terminating, rejecting late calls immediately", async () => {
    const read = deferred<FormulaResponse>();
    const close = deferred<FormulaResponse>();
    const worker = new ControlledWorker((request) => {
      if (request.method === "initialize") return success(request);
      if (request.method === "engine.createDraft") return success(request, 2);
      if (request.method === "draft.getState") return read.promise;
      if (request.method === "engine.close") return close.promise;
      throw new Error("Unexpected request");
    });
    const engine = await createFormulaEngineClient(
      { properties: [] },
      {
        workerFactory: () => worker as unknown as FormulaWorker,
      },
    );
    const draft = await engine.createDraft(draftState.definition);
    const earlier = draft.getState();
    const closing = engine.close();
    expect(engine.close()).toBe(closing);
    await expect(engine.getState()).rejects.toMatchObject({ code: "ENGINE_CLOSED", payload: null });
    await expect(draft.getState()).rejects.toMatchObject({ code: "ENGINE_CLOSED", payload: null });
    expect(draft.close()).toBe(closing);
    await vi.waitFor(() => expect(worker.requests).toHaveLength(3));
    expect(worker.terminate).not.toHaveBeenCalled();
    read.resolve(success(worker.requests[2], draftState));
    await expect(earlier).resolves.toEqual(draftState);
    await vi.waitFor(() => expect(worker.requests).toHaveLength(4));
    expect(worker.requests[3]?.method).toBe("engine.close");
    expect(worker.terminate).not.toHaveBeenCalled();
    close.resolve(success(worker.requests[3]));
    await expect(closing).resolves.toBeUndefined();
    expect(worker.terminate).toHaveBeenCalledTimes(1);
  });

  it("releases drafts created by calls queued before engine.close", async () => {
    const created = deferred<FormulaResponse>();
    const { engine, worker } = await setup(() => created.promise);
    const draftPromise = engine.createDraft(draftState.definition);
    const closing = engine.close();
    await vi.waitFor(() => expect(worker.requests).toHaveLength(2));
    created.resolve(success(worker.requests[1], 9));
    const draft = await draftPromise;
    await expect(draft.getState()).rejects.toMatchObject({ code: "ENGINE_CLOSED" });
    await closing;
    expect(worker.requests.map((request) => request.method)).toEqual([
      "initialize",
      "engine.createDraft",
      "engine.close",
    ]);
    await draft.close();
    expect(worker.terminate).toHaveBeenCalledTimes(1);
  });

  it("discards a draft once, while earlier reads finish", async () => {
    const read = deferred<FormulaResponse>();
    const { engine, worker } = await setup((request) => {
      if (request.method === "engine.createDraft") return success(request, 17);
      if (request.method === "draft.getState") return read.promise;
      if (request.method === "draft.close") return success(request);
      throw new Error("Unexpected request");
    });
    const draft = await engine.createDraft(draftState.definition);
    const earlier = draft.getState();
    const closing = draft.close();
    expect(draft.close()).toBe(closing);
    await expect(draft.intoDefinition()).rejects.toMatchObject({
      code: "DRAFT_CLOSED",
      data: { code: "DRAFT_CLOSED", payload: { handle: 17 } },
    });
    await vi.waitFor(() => expect(worker.requests).toHaveLength(3));
    read.resolve(success(worker.requests[2], draftState));
    await expect(earlier).resolves.toEqual(draftState);
    await closing;
    expect(worker.requests.filter((request) => request.method === "draft.close")).toHaveLength(1);
    await engine.close();
  });

  it("consumes a draft upon extraction and leaves committing to an explicit upsert", async () => {
    const definition = deferred<FormulaResponse>();
    const { engine, worker } = await setup((request) => {
      if (request.method === "engine.createDraft") return success(request, 18);
      if (request.method === "draft.intoDefinition") return definition.promise;
      if (request.method === "engine.upsert") return success(request, { affected_formulas: ["f"] });
      throw new Error("Unexpected request");
    });
    const draft = await engine.createDraft(draftState.definition);
    const extracted = draft.intoDefinition();
    await expect(draft.getState()).rejects.toMatchObject({
      code: "DRAFT_CLOSED",
      payload: { handle: 18 },
    });
    await expect(draft.intoDefinition()).rejects.toMatchObject({ code: "DRAFT_CLOSED" });
    const closing = draft.close();
    await vi.waitFor(() => expect(worker.requests).toHaveLength(3));
    definition.resolve(success(worker.requests[2], draftState.definition));
    await expect(extracted).resolves.toEqual(draftState.definition);
    await closing;
    expect(worker.requests.some((request) => request.method === "engine.upsert")).toBe(false);
    await engine.upsert({ Formula: await extracted });
    expect(worker.requests[3]?.args).toEqual([{ Formula: draftState.definition }]);
    expect(worker.requests.some((request) => request.method === "draft.close")).toBe(false);
    await engine.close();
  });

  it("forwards mutations with multiple active drafts and continues after a Rust rejection", async () => {
    let nextHandle = 0;
    const { engine, worker } = await setup((request) => {
      if (request.method === "engine.createDraft") return success(request, nextHandle++);
      if (request.method === "engine.upsert")
        return failure(request, {
          code: "ACTIVE_DRAFTS",
          message: "Active drafts prevent engine changes",
          payload: { count: 2 },
        });
      if (request.method === "engine.getState") return success(request, "AllReady");
      throw new Error("Unexpected request");
    });
    await Promise.all([
      engine.createDraft(draftState.definition),
      engine.createDraft({ id: "g", expression: "2" }),
    ]);
    const rejected = engine.upsert({ Formula: draftState.definition });
    const next = engine.getState();
    await expect(rejected).rejects.toBeInstanceOf(FormulaClientError);
    await expect(rejected).rejects.toMatchObject({ code: "ACTIVE_DRAFTS", payload: { count: 2 } });
    await expect(next).resolves.toBe("AllReady");
    expect(worker.requests.map((request) => request.method)).toEqual([
      "initialize",
      "engine.createDraft",
      "engine.createDraft",
      "engine.upsert",
      "engine.getState",
    ]);
    await engine.close();
  });
});

describe("FormulaEngineClient transport", () => {
  it("isolates queued input Maps, columns, arrays and caller-provided runtime values", async () => {
    const { engine, worker } = await setup((request) => success(request, { formulas: new Map() }));
    const numbers = {
      values: [NaN, Infinity, -Infinity, -0],
      validity: [true, true, true, true],
    };
    const dates = {
      values: [9_007_199_254_740_993n, 0n, -1n, 100n],
      validity: [true, true, true, true],
    };
    const input: EvaluateInput = {
      row_ids: ["a", "b", "c", "d"],
      columns: new Map<string, Column>([
        ["n", { Number: numbers }],
        ["date", { Date: dates }],
      ]),
      runtime: { now: 1_700_000_000_000n, time_zone: "+00:00" },
      formula_ids: ["f"],
    };
    const first = engine.evaluate(input);
    numbers.values[0] = 5;
    numbers.validity[0] = false;
    dates.values[0] = 1n;
    input.row_ids[0] = "changed";
    input.formula_ids.push("g");
    input.runtime.now = 1_700_000_000_001n;
    input.runtime.time_zone = "+08:00";
    input.columns.delete("date");
    const second = engine.evaluate(input);
    input.columns.clear();
    await Promise.all([first, second]);
    const firstSent = worker.requests[1];
    const secondSent = worker.requests[2];
    if (firstSent.method !== "engine.evaluate" || secondSent.method !== "engine.evaluate")
      throw new Error("Unexpected request");
    expect(firstSent.args[0]).toEqual({
      row_ids: ["a", "b", "c", "d"],
      columns: new Map<string, Column>([
        [
          "n",
          {
            Number: {
              values: [NaN, Infinity, -Infinity, -0],
              validity: [true, true, true, true],
            },
          },
        ],
        [
          "date",
          {
            Date: {
              values: [9_007_199_254_740_993n, 0n, -1n, 100n],
              validity: [true, true, true, true],
            },
          },
        ],
      ]),
      runtime: { now: 1_700_000_000_000n, time_zone: "+00:00" },
      formula_ids: ["f"],
    });
    expect(secondSent.args[0]).toEqual({
      row_ids: ["changed", "b", "c", "d"],
      columns: new Map<string, Column>([["n", { Number: numbers }]]),
      runtime: { now: 1_700_000_000_001n, time_zone: "+08:00" },
      formula_ids: ["f", "g"],
    });
    await engine.close();
  });

  it("scopes opaque diagnostic IDs per client, preserving stable state and native routing", async () => {
    function response(request: FormulaRequest): FormulaResponse {
      if (request.method === "engine.createDraft") return success(request, 1);
      if (request.method === "draft.getState")
        return success(request, {
          ...draftState,
          diagnostics: [{ id: "native:0", span: { start: 0, end: 1 }, message: "Error" }],
        });
      if (request.method === "draft.updateExpression")
        return success(request, {
          state: {
            ...draftState,
            version: 1n,
            diagnostics: [{ id: "native:1", span: { start: 0, end: 1 }, message: "Changed" }],
          },
          cursor: 1,
        });
      if (request.method === "draft.quickFixes") return success(request, []);
      throw new Error("Unexpected request");
    }
    const first = await setup(response);
    const second = await setup(response);
    const draft = await first.engine.createDraft(draftState.definition);
    const other = await second.engine.createDraft(draftState.definition);
    const state = await draft.getState();
    const foreign = await other.getState();
    expect(await draft.getState()).toEqual(state);
    const id = state.diagnostics[0].id;
    expect(id).not.toBe(foreign.diagnostics[0].id);
    expect(id).toMatch(/^formula:[0-9a-f]{32}:native:0$/);
    await draft.quickFixes(id);
    expect(first.worker.requests.slice(-1)[0]?.args).toEqual([1, "native:0"]);
    await draft.quickFixes(foreign.diagnostics[0].id);
    expect(first.worker.requests.slice(-1)[0]?.args).toEqual([
      1,
      `foreign:${foreign.diagnostics[0].id}`,
    ]);
    await draft.quickFixes("native:0");
    expect(first.worker.requests.slice(-1)[0]?.args).toEqual([1, "foreign:native:0"]);
    const updated = await draft.updateExpression({ Replace: "2" });
    expect(updated.state.diagnostics[0].id).toBe(id.replace(/native:0$/, "native:1"));
    await expect(draft.quickFixes(id)).resolves.toEqual([]);
    expect(first.worker.requests.slice(-1)[0]?.args).toEqual([1, "native:0"]);
    await Promise.all([first.engine.close(), second.engine.close()]);
  });

  it("preserves Map, bigint, null, and all JS number values in both directions", async () => {
    const column: Column = {
      Number: { values: [NaN, Infinity, -Infinity, -0], validity: [true, true, false, true] },
    };
    const input: EvaluateInput = {
      row_ids: ["a", "b", "c", "d"],
      columns: new Map<string, Column>([
        ["n", column],
        ["date", { Date: { values: [1n, 2n, 3n, 4n], validity: [true, true, true, true] } }],
      ]),
      runtime: { now: 1_700_000_000_000n, time_zone: "+00:00" },
      formula_ids: ["f"],
    };
    const { engine, worker } = await setup((request) =>
      success(request, {
        formulas: new Map([["f", { Ok: { output_type: "Number", column, errors: [] } }]]),
      }),
    );
    const result = await engine.evaluate(input);
    const sent = worker.requests[1];
    expect(sent.method).toBe("engine.evaluate");
    if (sent.method !== "engine.evaluate") throw new Error("Unexpected request");
    expect(sent.args[0]).toEqual(input);
    expect(sent.args[0].columns).toBeInstanceOf(Map);
    expect(sent.args[0].runtime.now).toBe(1_700_000_000_000n);
    expect(result.formulas).toBeInstanceOf(Map);
    expect(result.formulas.get("f")).toEqual({ Ok: { output_type: "Number", column, errors: [] } });
    await engine.close();
  });

  it("reconstructs typed Errors with discriminated, lossless native payloads", async () => {
    const errorData: FormulaClientErrorData = {
      code: "EVALUATE_INPUT",
      message: "Invalid runtime timestamp",
      payload: { error: { InvalidNow: { now: -9_223_372_036_854_775_808n } } },
    };
    const { engine } = await setup((request) => failure(request, errorData));
    try {
      await engine.getState();
      throw new Error("Expected a rejection");
    } catch (error) {
      expect(error).toBeInstanceOf(Error);
      expect(error).toBeInstanceOf(FormulaClientError);
      if (!(error instanceof FormulaClientError)) throw error;
      expect(error.message).toBe(errorData.message);
      expect(error.data).toEqual(errorData);
      if (error.data.code !== "EVALUATE_INPUT") throw new Error("Wrong error code");
      expect(error.data.payload.error).toEqual({
        InvalidNow: { now: -9_223_372_036_854_775_808n },
      });
    }
    await engine.close();
  });

  it("rejects asynchronous initialization and terminates the owned worker", async () => {
    const worker = new ControlledWorker((request) =>
      failure(request, {
        code: "ENGINE_INIT",
        message: "Duplicate property",
        payload: { error: { DuplicateId: "f" } },
      }),
    );
    await expect(
      createFormulaEngineClient(
        { properties: [] },
        {
          workerFactory: () => worker as unknown as FormulaWorker,
        },
      ),
    ).rejects.toMatchObject({ code: "ENGINE_INIT", payload: { error: { DuplicateId: "f" } } });
    expect(worker.terminate).toHaveBeenCalledTimes(1);
  });

  it("rejects worker construction failures with a typed initialization error", async () => {
    await expect(
      createFormulaEngineClient(
        { properties: [] },
        {
          workerFactory: () => {
            throw new Error("Cannot construct module worker");
          },
        },
      ),
    ).rejects.toMatchObject({
      code: "INITIALIZATION_ERROR",
      payload: null,
      message: "Cannot construct module worker",
    });
  });

  it("settles in-flight and queued Promises after worker failure and terminates once", async () => {
    const unanswered = deferred<FormulaResponse>();
    const { engine, worker } = await setup(() => unanswered.promise);
    const active = engine.getState();
    const queued = engine.getProperties();
    const settled = Promise.allSettled([active, queued]);
    await vi.waitFor(() => expect(worker.requests).toHaveLength(2));
    worker.crash("WASM worker crashed");
    const results = await settled;
    for (const result of results) {
      expect(result.status).toBe("rejected");
      if (result.status === "rejected")
        expect(result.reason).toMatchObject({ code: "WORKER_FAILURE", payload: null });
    }
    await expect(engine.getProperty("f")).rejects.toMatchObject({ code: "WORKER_FAILURE" });
    await expect(engine.close()).rejects.toMatchObject({ code: "WORKER_FAILURE" });
    expect(worker.terminate).toHaveBeenCalledTimes(1);
    expect(worker.requests).toHaveLength(2);
  });

  it("settles pending calls if the worker response cannot be deserialized", async () => {
    const unanswered = deferred<FormulaResponse>();
    const { engine, worker } = await setup(() => unanswered.promise);
    const reading = engine.getState();
    await vi.waitFor(() => expect(worker.requests).toHaveLength(2));
    worker.dispatchEvent(new MessageEvent("messageerror"));
    await expect(reading).rejects.toMatchObject({ code: "WORKER_FAILURE" });
    await expect(engine.close()).rejects.toMatchObject({ code: "WORKER_FAILURE" });
    expect(worker.terminate).toHaveBeenCalledTimes(1);
  });

  it("treats malformed worker responses as terminal typed failures", async () => {
    const { engine, worker } = await setup(
      (request) =>
        ({ id: request.id, ok: false, error: { code: "WORKER_FAILURE" } }) as FormulaResponse,
    );
    await expect(engine.getState()).rejects.toMatchObject({ code: "INVALID_RESPONSE" });
    await expect(engine.close()).rejects.toMatchObject({ code: "INVALID_RESPONSE" });
    expect(worker.terminate).toHaveBeenCalledTimes(1);
  });

  it.each([NaN, Infinity, -1, 1.5, Number.MAX_SAFE_INTEGER + 1, 999])(
    "settles in-flight and queued calls for an invalid response ID (%s)",
    async (id) => {
      const { engine, worker } = await setup(() => ({ id, ok: true, value: "AllReady" }));
      const results = await Promise.allSettled([engine.getState(), engine.getProperties()]);
      for (const result of results) {
        expect(result.status).toBe("rejected");
        if (result.status === "rejected")
          expect(result.reason).toMatchObject({ code: "INVALID_RESPONSE", payload: null });
      }
      await expect(engine.close()).rejects.toMatchObject({ code: "INVALID_RESPONSE" });
      expect(worker.terminate).toHaveBeenCalledTimes(1);
      expect(worker.requests.map((request) => request.method)).toEqual([
        "initialize",
        "engine.getState",
      ]);
    },
  );

  it("queues clone failures behind earlier calls and continues with later calls", async () => {
    const blocked = deferred<FormulaResponse>();
    const events: string[] = [];
    const { engine, worker } = await setup((request) =>
      request.method === "engine.getState" ? blocked.promise : success(request, null),
    );
    const earlier = engine.getState().then((state) => {
      events.push("earlier");
      return state;
    });
    let invalid!: ReturnType<typeof engine.upsert>;
    expect(() => {
      invalid = engine.upsert({
        Formula: { ...draftState.definition, extra: () => undefined },
      } as Parameters<typeof engine.upsert>[0]);
    }).not.toThrow();
    const rejected = invalid.catch((error: unknown) => {
      events.push("invalid");
      throw error;
    });
    const next = engine.getProperty("f").then((property) => {
      events.push("next");
      return property;
    });
    const settled = Promise.allSettled([earlier, rejected, next]);
    await vi.waitFor(() => expect(worker.requests).toHaveLength(2));
    expect(events).toEqual([]);
    blocked.resolve(success(worker.requests[1], "AllReady"));
    const results = await settled;
    expect(results[0]).toEqual({ status: "fulfilled", value: "AllReady" });
    expect(results[1]).toMatchObject({
      status: "rejected",
      reason: { code: "INVALID_REQUEST", payload: null },
    });
    expect(results[2]).toEqual({ status: "fulfilled", value: null });
    expect(events).toEqual(["earlier", "invalid", "next"]);
    expect(worker.requests.map((request) => request.method)).toEqual([
      "initialize",
      "engine.getState",
      "engine.getProperty",
    ]);
    await engine.close();
  });
});
