import {
  createFormulaEngineClient,
  FormulaClientError,
  type Column,
  type EvaluateInput,
  type FormulaClientErrorData,
  type FormulaDraftState,
  type FormulaSchema,
} from "@notion-formula/sdk";
import init, { FormulaEngineSession } from "@notion-formula/sdk/wasm";
import FormulaWorker from "@notion-formula/sdk/worker?worker";
import type { FormulaSession } from "../../../../packages/notion-formula/dist/worker_runtime.js";

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function equal(actual: unknown, expected: unknown, label: string): void {
  if (Object.is(actual, expected)) return;
  if (actual instanceof Map && expected instanceof Map) {
    assert(actual.size === expected.size, `${label}: map size`);
    for (const [key, value] of actual as Map<unknown, unknown>) {
      assert(expected.has(key), `${label}: map key`);
      equal(value, expected.get(key), label);
    }
    return;
  }
  if (Array.isArray(actual) && Array.isArray(expected)) {
    assert(actual.length === expected.length, `${label}: array length`);
    for (let index = 0; index < actual.length; index++)
      equal(actual[index], expected[index], label);
    return;
  }
  if (
    typeof actual === "object" &&
    actual !== null &&
    typeof expected === "object" &&
    expected !== null
  ) {
    const actualFields = actual as Record<string, unknown>;
    const expectedFields = expected as Record<string, unknown>;
    const keys = Object.keys(actualFields).sort();
    const expectedKeys = Object.keys(expectedFields).sort();
    assert(keys.length === expectedKeys.length, `${label}: object fields`);
    keys.forEach((key, index) => {
      assert(key === expectedKeys[index], `${label}: object field name`);
      equal(actualFields[key], expectedFields[key], `${label}.${key}`);
    });
    return;
  }
  throw new Error(`${label}: values differ (${String(actual)}, ${String(expected)})`);
}

function comparableDraft(state: FormulaDraftState) {
  // Diagnostic IDs belong to a particular session and are intentionally opaque.
  return {
    ...state,
    diagnostics: state.diagnostics.map(({ message, span }) => ({ message, span })),
  };
}

function nativeError(action: () => unknown): FormulaClientErrorData {
  try {
    action();
  } catch (error) {
    assert(typeof error === "object" && error !== null && "code" in error, "Native typed error");
    return error as FormulaClientErrorData;
  }
  throw new Error("Expected a synchronous WASM error");
}

async function clientError(action: () => Promise<unknown>): Promise<FormulaClientErrorData> {
  try {
    await action();
  } catch (error) {
    assert(error instanceof FormulaClientError, "Client rejection must be FormulaClientError");
    assert(error instanceof Error, "Client rejection must retain Error prototype");
    return error.data;
  }
  throw new Error("Expected a Worker client rejection");
}

const schema: FormulaSchema = {
  properties: [
    { Input: { id: "n", ty: "Number" } },
    { Input: { id: "date", ty: "Date" } },
    { Input: { id: "text", ty: "String" } },
    { Input: { id: "items", ty: { List: "Unknown" } } },
    { Input: { id: "dynamic", ty: "Unknown" } },
    { Formula: { id: "copy", expression: 'prop("n")' } },
    { Formula: { id: "date_copy", expression: 'prop("date")' } },
    { Formula: { id: "list_copy", expression: 'prop("items")' } },
    { Formula: { id: "union_copy", expression: 'prop("dynamic")' } },
    { Formula: { id: "formatted", expression: 'repeat(prop("text"), 2)' } },
    { Formula: { id: "row_error", expression: 'test(prop("text"), "[")' } },
    { Formula: { id: "missing", expression: 'prop("absent") + 1' } },
    { Formula: { id: "editable", expression: "1 + 1" } },
  ],
};

const input: EvaluateInput = {
  row_ids: ["a", "b", "c", "d"],
  columns: new Map<string, Column>([
    [
      "n",
      { Number: { values: [NaN, Infinity, -Infinity, -0], validity: [true, true, true, true] } },
    ],
    [
      "date",
      {
        Date: { values: [1n, -1n, 9_007_199_254_740_993n, 0n], validity: [true, true, true, true] },
      },
    ],
    ["text", { String: { values: ["😀漢", "x", "", "z"], validity: [true, false, true, true] } }],
    [
      "items",
      {
        List: {
          values: [
            [{ Number: NaN }, null, { Date: 9_007_199_254_740_993n }, { List: [{ String: "😀" }] }],
            [],
            [{ Boolean: false }],
            [{ Number: -0 }, { String: "漢" }],
          ],
          validity: [true, true, true, true],
        },
      },
    ],
    [
      "dynamic",
      {
        Union: {
          values: [
            { Number: -0 },
            { Date: 9_007_199_254_740_993n },
            { List: [null, { Boolean: false }] },
            { String: "😀" },
          ],
          validity: [true, true, true, true],
        },
      },
    ],
  ]),
  runtime: { now: 1_700_000_000_000n, time_zone: "+08:00" },
  formula_ids: [
    "copy",
    "date_copy",
    "list_copy",
    "union_copy",
    "formatted",
    "row_error",
    "missing",
  ],
};

export async function runFormulaWorkerContract() {
  await init();
  const direct = new FormulaEngineSession(schema) as FormulaSession;
  const engine = await createFormulaEngineClient(schema);
  const verified: string[] = [];
  try {
    equal(await engine.getProperties(), direct.get_properties(), "properties");
    equal(await engine.getProperty("copy"), direct.get_property("copy"), "property");
    equal(await engine.getProperty("absent"), null, "missing property null");
    equal(await engine.getState(), direct.get_state(), "state");
    verified.push("engine snapshots");

    const transitive = { Formula: { id: "twice", expression: 'prop("copy") + prop("copy")' } };
    await engine.upsert(transitive);
    direct.upsert(transitive);
    const targets = ["twice", "formatted"];
    const dependencies = engine.requiredInputs(targets);
    targets[0] = "absent";
    equal(await dependencies, ["n", "text"], "transitive input snapshot");
    equal(
      await engine.requiredInputs(["twice", "formatted"]),
      direct.required_inputs(["twice", "formatted"]),
      "native dependency query",
    );
    equal(
      await clientError(() => engine.requiredInputs(["n"])),
      nativeError(() => direct.required_inputs(["n"])),
      "dependency selection error",
    );
    verified.push("transitive input dependencies");

    const result = await engine.evaluate(input);
    equal(result, direct.evaluate(input), "evaluation");
    assert(result.formulas instanceof Map, "Result formulas must be Map");
    const numeric = result.formulas.get("copy");
    assert(numeric && "Ok" in numeric && "Number" in numeric.Ok.column, "Expected numeric copy");
    const numbers = numeric.Ok.column.Number.values;
    assert(
      Number.isNaN(numbers[0]) &&
        numbers[1] === Infinity &&
        numbers[2] === -Infinity &&
        Object.is(numbers[3], -0),
      "Special numbers must survive both boundaries",
    );
    const date = result.formulas.get("date_copy");
    assert(date && "Ok" in date && "Date" in date.Ok.column, "Expected Date copy");
    assert(date.Ok.column.Date.values[2] === 9_007_199_254_740_993n, "Date bigint must be exact");
    equal(result.formulas.get("missing"), { Err: "NotReady" }, "Formula error is result data");
    const rowError = result.formulas.get("row_error");
    assert(
      rowError && "Ok" in rowError && rowError.Ok.errors.length > 0,
      "Row errors must remain result data",
    );
    assert(
      rowError.Ok.errors.every((error) => typeof error.row_index === "number"),
      "Row indexes must remain JS numbers",
    );
    verified.push("lossless evaluation DTOs");

    const invalidInput = {
      ...input,
      runtime: { now: -9_223_372_036_854_775_808n, time_zone: "+00:00" },
    };
    equal(
      await clientError(() => engine.evaluate(invalidInput)),
      nativeError(() => direct.evaluate(invalidInput)),
      "evaluation input error",
    );
    const invalidDefinition = { Formula: { id: "", expression: "1" } };
    equal(
      await clientError(() => engine.upsert(invalidDefinition)),
      nativeError(() => direct.upsert(invalidDefinition)),
      "engine change error",
    );
    verified.push("typed native errors");

    const definition = { id: "editable", expression: '"😀" +' };
    const [first, second] = await Promise.all([
      engine.createDraft(definition),
      engine.createDraft({ id: "other", expression: "3" }),
    ]);
    const firstHandle = direct.create_draft(definition);
    const secondHandle = direct.create_draft({ id: "other", expression: "3" });
    equal(await engine.requiredInputs(["twice"]), ["n"], "dependency query during editing");
    const clientState = await first.getState();
    const directState = direct.draft_state(firstHandle);
    equal(comparableDraft(clientState), comparableDraft(directState), "draft state");
    assert(typeof clientState.version === "bigint", "Draft version must be bigint");
    equal(
      await first.help(2, { preferred_limit: 0 }),
      direct.draft_help(firstHandle, 2, { preferred_limit: 0 }),
      "UTF-16 help",
    );
    if (clientState.diagnostics[0] && directState.diagnostics[0]) {
      equal(
        await first.quickFixes(clientState.diagnostics[0].id),
        direct.draft_quick_fixes(firstHandle, directState.diagnostics[0].id),
        "quick fixes",
      );
    }
    equal(
      await clientError(() => first.formatEdits()),
      nativeError(() => direct.draft_format_edits(firstHandle)),
      "format failure",
    );
    const blockedMutation = await clientError(() => engine.remove("editable"));
    equal(
      blockedMutation,
      nativeError(() => direct.remove("editable")),
      "active drafts",
    );
    assert(
      blockedMutation.code === "ACTIVE_DRAFTS" && blockedMutation.payload.count === 2,
      "Rust must reject both active Draft borrows",
    );
    verified.push("multiple drafts and borrow errors");

    const replacement = await first.updateExpression({ Replace: '"😀"' });
    const directReplacement = direct.draft_update_expression(firstHandle, { Replace: '"😀"' });
    equal(
      comparableDraft(replacement.state),
      comparableDraft(directReplacement.state),
      "replace state",
    );
    equal(replacement.cursor, directReplacement.cursor, "replace cursor");
    assert(
      replacement.cursor === 4 && replacement.state.version === 1n,
      "Replacement cursor is UTF-16 and version is bigint",
    );
    equal(await first.formatEdits(), direct.draft_format_edits(firstHandle), "format edit");
    const stale = { Edits: { edit: { base_version: 0n, edits: [] }, cursor: 0 } };
    equal(
      await clientError(() => first.updateExpression(stale)),
      nativeError(() => direct.draft_update_expression(firstHandle, stale)),
      "stale edit",
    );
    const update = {
      Edits: {
        edit: { base_version: 1n, edits: [{ range: { start: 2, end: 3 }, new_text: "x" }] },
        cursor: 3,
      },
    };
    const changed = await first.updateExpression(update);
    const directChanged = direct.draft_update_expression(firstHandle, update);
    equal(
      comparableDraft(changed.state),
      comparableDraft(directChanged.state),
      "UTF-16 edit state",
    );
    equal(changed.cursor, directChanged.cursor, "UTF-16 edit cursor");
    assert(
      changed.state.definition.expression === '"x"' && changed.cursor === 2,
      "Surrogate endpoints must floor in Rust",
    );
    const help = await first.help(3, { preferred_limit: 5 });
    equal(help.signature_help, null, "Absent signature is explicit null");
    const functionUpdate = { Replace: 'repeat("😀", 2)' };
    const functionState = await first.updateExpression(functionUpdate);
    equal(
      comparableDraft(functionState.state),
      comparableDraft(direct.draft_update_expression(firstHandle, functionUpdate).state),
      "Function update",
    );
    const signature = await first.help(13, { preferred_limit: 5 });
    equal(signature, direct.draft_help(firstHandle, 13, { preferred_limit: 5 }), "Signature help");
    assert(signature.signature_help !== null, "Function call must have signature help");
    assert(
      typeof signature.signature_help.active_signature === "number" &&
        typeof signature.signature_help.active_parameter === "number",
      "Signature indexes must remain JS numbers",
    );
    assert(
      signature.completion.preferred_indices.every((index) => typeof index === "number"),
      "Completion indexes must remain JS numbers",
    );
    verified.push("UTF-16 edits and versions");

    const extracted = await first.intoDefinition();
    equal(extracted, direct.draft_into_definition(firstHandle), "extraction");
    const consumed = await clientError(() => first.getState());
    assert(
      consumed.code === "DRAFT_CLOSED" && typeof consumed.payload.handle === "number",
      "Consumed draft must reject with typed handle",
    );
    equal(
      await engine.getProperty("editable"),
      direct.get_property("editable"),
      "extraction leaves saved definition",
    );
    const saved = await engine.getProperty("editable");
    assert(
      saved && "Formula" in saved && saved.Formula.definition.expression === "1 + 1",
      "Extraction must not commit",
    );
    equal(
      await clientError(() => engine.upsert({ Formula: extracted })),
      nativeError(() => direct.upsert({ Formula: extracted })),
      "remaining active draft",
    );
    await second.close();
    await second.close();
    direct.draft_close(secondHandle);
    equal(
      await engine.upsert({ Formula: extracted }),
      direct.upsert({ Formula: extracted }),
      "explicit commit",
    );
    equal(await engine.remove("absent"), direct.remove("absent"), "missing remove null");
    verified.push("consume discard and explicit commit");

    const property = { Formula: { id: "snapshot", expression: "1" } };
    const original = engine.upsert(property);
    const between = engine.getProperty("snapshot");
    property.Formula.expression = "2";
    const changedProperty = engine.upsert(property);
    equal(
      await original,
      direct.upsert({ Formula: { id: "snapshot", expression: "1" } }),
      "first request snapshot",
    );
    equal(await between, direct.get_property("snapshot"), "snapshot between mutations");
    equal(await changedProperty, direct.upsert(property), "second request snapshot");
    verified.push("request snapshots");

    const queued = engine.getProperties();
    const closing = engine.close();
    assert(engine.close() === closing, "Close must return the same Promise");
    const late = await clientError(() => engine.getState());
    assert(
      late.code === "ENGINE_CLOSED" && late.payload === null,
      "Late calls must reject immediately",
    );
    equal(await queued, direct.get_properties(), "read queued before close");
    await closing;
    await first.close();
    verified.push("queued close and idempotence");
    return {
      verified,
      rows: input.row_ids.length,
      specialNumbers: true,
      exactDate: true,
      maps: true,
      utf16Cursor: changed.cursor,
    };
  } finally {
    await engine.close();
    direct.close();
    direct.free();
  }
}

export async function runFormulaWorkerFailures() {
  let initTerminations = 0;
  const error = await clientError(() =>
    createFormulaEngineClient(
      {
        properties: [
          { Formula: { id: "same", expression: "1" } },
          { Formula: { id: "same", expression: "2" } },
        ],
      },
      {
        workerFactory: () => {
          const worker = new FormulaWorker();
          const terminate = worker.terminate.bind(worker);
          worker.terminate = () => {
            initTerminations++;
            terminate();
          };
          return worker;
        },
      },
    ),
  );
  assert(error.code === "ENGINE_INIT", "Schema initialization must retain the native error code");
  equal(error.payload, { error: { DuplicateId: "same" } }, "Initialization error payload");
  assert(initTerminations === 1, "Initialization failure must terminate once");

  const url = URL.createObjectURL(
    new Blob(
      [
        `self.addEventListener("message", ({data}) => {
      if (data.method === "initialize") self.postMessage({id: data.id, ok: true, value: undefined});
      else throw new Error("Contract worker failed");
    });`,
      ],
      { type: "text/javascript" },
    ),
  );
  let failureTerminations = 0;
  try {
    const engine = await createFormulaEngineClient(
      { properties: [] },
      {
        workerFactory: () => {
          const worker = new Worker(url, { type: "module" });
          const terminate = worker.terminate.bind(worker);
          worker.terminate = () => {
            failureTerminations++;
            terminate();
          };
          return worker;
        },
      },
    );
    const settled = await Promise.allSettled([engine.getState(), engine.getProperties()]);
    for (const result of settled) {
      assert(result.status === "rejected", "Worker failure must settle every pending Promise");
      const reason: unknown = result.reason;
      assert(
        reason instanceof FormulaClientError && reason.code === "WORKER_FAILURE",
        "Worker failure rejection must be typed",
      );
    }
    const closeError = await clientError(() => engine.close());
    assert(
      closeError.code === "WORKER_FAILURE" && failureTerminations === 1,
      "Failed worker close must terminate once",
    );
    return { initTerminations, failureTerminations, settled: settled.length };
  } finally {
    URL.revokeObjectURL(url);
  }
}

export async function runFormulaDiagnosticScopes() {
  const [first, second] = await Promise.all([
    createFormulaEngineClient({ properties: [] }),
    createFormulaEngineClient({ properties: [] }),
  ]);
  try {
    const [draft, other] = await Promise.all([
      first.createDraft({ id: "f", expression: "[1,2,]" }),
      second.createDraft({ id: "f", expression: "[1,2,]" }),
    ]);
    const state = await draft.getState();
    const otherState = await other.getState();
    const id = state.diagnostics[0]?.id;
    const otherId = otherState.diagnostics[0]?.id;
    assert(id && otherId && id !== otherId, "Diagnostic IDs must differ between Worker clients");
    equal(await draft.getState(), state, "Repeated state keeps opaque IDs");
    assert(
      (await draft.quickFixes(id)).length > 0,
      "Own diagnostic must route to its native quick fix",
    );
    equal(await draft.quickFixes(otherId), [], "Foreign client diagnostic");
    equal(await other.quickFixes(id), [], "Reverse foreign client diagnostic");
    equal(await draft.quickFixes(`forged:${id}`), [], "Forged diagnostic scope");
    await draft.updateExpression({ Replace: "[1,2]" });
    equal(await draft.quickFixes(id), [], "Obsolete diagnostic");
    const changed = await draft.updateExpression({ Replace: "[1,2,]" });
    equal(changed.state, await draft.getState(), "Update result uses the same scoped IDs as state");
    assert(changed.state.diagnostics[0]?.id !== id, "Recreated diagnostics must have new IDs");
    equal(await draft.quickFixes(id), [], "Obsolete diagnostic after restoring text");
    return {
      distinct: true,
      stable: true,
      crossClientEmpty: true,
      staleEmpty: true,
      updateStateScoped: true,
    };
  } finally {
    await Promise.all([first.close(), second.close()]);
  }
}

declare global {
  interface Window {
    __formula_worker_contract: typeof runFormulaWorkerContract;
    __formula_worker_failures: typeof runFormulaWorkerFailures;
    __formula_diagnostic_scopes: typeof runFormulaDiagnosticScopes;
  }
}

window.__formula_worker_contract = runFormulaWorkerContract;
window.__formula_worker_failures = runFormulaWorkerFailures;
window.__formula_diagnostic_scopes = runFormulaDiagnosticScopes;
