import {
  createFormulaEngineClient,
  decodeFormulaString,
  encodeFormulaString,
  FormulaClientError,
  type Column,
  type EvaluateInput,
  type FormulaClientErrorData,
  type FormulaDraftState,
  type FormulaSchema,
} from "@notion-formula/sdk";
import * as sdk from "@notion-formula/sdk";
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

    const range = {
      start: 9_007_199_254_740_993n,
      end: -9_007_199_254_740_993n,
      include_time: false,
    };
    const richDates: Column = {
      DateValue: {
        values: [
          range,
          { start: 1n, end: null, include_time: false },
          { start: -1n, end: null, include_time: true },
          range,
        ],
        validity: [true, true, true, false],
      },
    };
    const nestedDates: Column = {
      List: {
        values: [[{ DateValue: range }, null, { Date: 0n }], [], [], []],
        validity: [true, true, true, true],
      },
    };
    const unionDates: Column = {
      Union: {
        values: [
          { DateValue: range },
          { Date: 0n },
          { Number: -0 },
          { List: [{ DateValue: range }] },
        ],
        validity: [true, true, true, true],
      },
    };
    const richInput: EvaluateInput = {
      ...input,
      columns: new Map(input.columns),
      formula_ids: ["date_copy", "list_copy", "union_copy"],
    };
    richInput.columns.set("date", richDates);
    richInput.columns.set("items", nestedDates);
    richInput.columns.set("dynamic", unionDates);
    const richResult = await engine.evaluate(richInput);
    equal(richResult, direct.evaluate(richInput), "rich date evaluation");
    const richDate = richResult.formulas.get("date_copy");
    assert(richDate && "Ok" in richDate, "Expected rich date copy");
    assert("DateValue" in richDate.Ok.column, "Expected rich date column");
    equal(
      richDate.Ok.column,
      {
        DateValue: {
          ...richDates.DateValue,
          values: [
            ...richDates.DateValue.values.slice(0, 3),
            { start: 0n, end: null, include_time: false },
          ],
        },
      },
      "exact range, hidden time, mixed scalar row and null placeholder",
    );
    equal(richDate.Ok.output_type, "Date", "rich dates share Date semantic type");
    for (const [id, column] of [
      ["list_copy", nestedDates],
      ["union_copy", unionDates],
    ] as const) {
      const output = richResult.formulas.get(id);
      assert(output && "Ok" in output, "Expected nested date copy");
      equal(output.Ok.column, column, id);
    }
    verified.push("rich date metadata");

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
      richDates: true,
      richRange: {
        start: richDate.Ok.column.DateValue.values[0].start.toString(),
        end: richDate.Ok.column.DateValue.values[0].end?.toString() ?? null,
        include_time: richDate.Ok.column.DateValue.values[0].include_time,
      },
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

export async function runFormulaStringCodec() {
  const canonicalCases = [
    { value: "plain", source: '"plain"' },
    { value: "", source: '""' },
    { value: "\n", source: String.raw`"\n"` },
    { value: "\t", source: String.raw`"\t"` },
    { value: '"', source: String.raw`"\""` },
    { value: 'a"b', source: String.raw`"a\"b"` },
    { value: "\\", source: String.raw`"\\"` },
    { value: 'line\nnext\tquote"slash\\', source: String.raw`"line\nnext\tquote\"slash\\"` },
    { value: "中文😀", source: '"中文😀"' },
    { value: "raw\r\0\b\f\u0001", source: '"raw\r\0\b\f\u0001"' },
  ];
  // The codec is available synchronously before any Worker/WASM initialization.
  for (const entry of canonicalCases) {
    equal(decodeFormulaString(entry.source), entry.value, "Synchronous SDK decoding");
    equal(encodeFormulaString(entry.value), entry.source, "Canonical formula encoding");
    equal(decodeFormulaString(encodeFormulaString(entry.value)), entry.value, "Codec inverse");
  }
  assert(!("quoteFormulaString" in sdk), "SDK exports the encode name without a quote alias");
  const rawControls = '"raw\n\t\r\0\b\f\u0001"';
  equal(decodeFormulaString(rawControls), "raw\n\t\r\0\b\f\u0001", "Raw control decoding");
  const cases = [
    ...canonicalCases,
    { value: "raw\n\t\r\0\b\f\u0001", source: rawControls },
    { value: "q", source: String.raw`"\q"` },
    { value: "r", source: String.raw`"\r"` },
    { value: "x41", source: String.raw`"\x41"` },
    { value: "u0041", source: String.raw`"\u0041"` },
    { value: "b", source: String.raw`"\b"` },
    { value: "f", source: String.raw`"\f"` },
    { value: "0", source: String.raw`"\0"` },
    { value: "中文😀", source: String.raw`"\中\文\😀"` },
    { value: "\\q", source: String.raw`"\\q"` },
    { value: "raw\n\t\r", source: '"raw\\\n\\\t\\\r"' },
  ];
  for (const entry of cases)
    equal(decodeFormulaString(entry.source), entry.value, "Decode complete formula token text");
  const incompleteLiterals = [
    'missing"',
    '"missing',
    '"',
    '"trailing' + "\\",
    String.raw`"escaped\"`,
    String.raw`"identity\q`,
  ];
  const invalidLiterals = [
    "",
    "'single'",
    ...incompleteLiterals,
    '"unescaped"quote"',
    '"trailing"text',
  ];

  const engine = await createFormulaEngineClient({
    properties: [{ Input: { id: "q", ty: "Number" } }, { Input: { id: "\\q", ty: "Number" } }],
  });
  const columns = new Map<string, Column>([
    ["q", { Number: { values: [41], validity: [true] } }],
    ["\\q", { Number: { values: [73], validity: [true] } }],
  ]);
  const roundtrips: { source: string; value: string; utf16End: number }[] = [];
  const rejected: { source: string; diagnosticCount: number }[] = [];
  const propertyRoundtrips: { source: string; propertyId: string; value: number }[] = [];
  try {
    for (const [index, entry] of cases.entries()) {
      const source = entry.source;
      const id = `value-${index}`;
      const draft = await engine.createDraft({ id, expression: source });
      const state = await draft.getState();
      equal(state.diagnostics, [], "Quoted literal is accepted by the real parser");
      const token = state.tokens[0];
      equal(token.kind, "String", "Literal token kind");
      equal(token.text, source, "Literal raw text");
      equal(token.span, { start: 0, end: source.length }, "Literal UTF-16 span");
      for (const token of state.tokens)
        equal(Object.keys(token).sort(), ["kind", "span", "text"], "Raw Token DTO fields");
      const decoded: string = decodeFormulaString(token.text);
      equal(decoded, entry.value, "Decode actual Worker token text");
      equal(
        decodeFormulaString(encodeFormulaString(decoded)),
        entry.value,
        "Re-encode decoded value",
      );
      await draft.close();
      await engine.upsert({ Formula: { id, expression: source } });
      const result = await engine.evaluate({
        row_ids: ["row"],
        columns,
        runtime: { now: 0n, time_zone: "+00:00" },
        formula_ids: [id],
      });
      const formula = result.formulas.get(id);
      assert(formula && "Ok" in formula && "String" in formula.Ok.column, "String result");
      equal(formula.Ok.column.String.values, [entry.value], "Real parser evaluation roundtrip");
      equal(
        decodeFormulaString(source),
        formula.Ok.column.String.values[0],
        "SDK decoding matches native WASM evaluation",
      );
      roundtrips.push({ source, value: entry.value, utf16End: token.span.end });
    }
    for (const source of invalidLiterals) {
      const draft = await engine.createDraft({ id: "invalid", expression: source });
      const state = await draft.getState();
      for (const token of state.tokens)
        equal(Object.keys(token).sort(), ["kind", "span", "text"], "Invalid Token DTO fields");
      assert(state.diagnostics.length > 0, "Invalid syntax retains diagnostics");
      if (incompleteLiterals.includes(source)) {
        assert(
          state.tokens.every((token) => token.kind !== "String"),
          "Incomplete string source has no complete String token",
        );
        assert(
          state.diagnostics.some((diagnostic) =>
            diagnostic.message.includes("unterminated string"),
          ),
          "Incomplete strings retain lexer diagnostics",
        );
      }
      rejected.push({ source, diagnosticCount: state.diagnostics.length });
      await draft.close();
    }
    for (const [index, entry] of [
      { source: String.raw`prop("\q")`, propertyId: "q", value: 41 },
      { source: String.raw`prop("\\q")`, propertyId: "\\q", value: 73 },
      { source: 'prop("q")', propertyId: "q", value: 41 },
    ].entries()) {
      const id = `property-${index}`;
      const draft = await engine.createDraft({ id, expression: entry.source });
      const state = await draft.getState();
      equal(state.diagnostics, [], "Escaped property ID resolves through the real Worker");
      const token = state.tokens.find((token) => token.kind === "String");
      assert(token, "Property argument has a complete String token");
      equal(token.text, entry.source.slice(5, -1), "Property token preserves original spelling");
      equal(token.span, { start: 5, end: entry.source.length - 1 }, "Property argument span");
      equal(decodeFormulaString(token.text), entry.propertyId, "Property ID is decoded once");
      await draft.close();
      await engine.upsert({ Formula: { id, expression: entry.source } });
      const result = await engine.evaluate({
        row_ids: ["row"],
        columns,
        runtime: { now: 0n, time_zone: "+00:00" },
        formula_ids: [id],
      });
      const formula = result.formulas.get(id);
      assert(formula && "Ok" in formula && "Number" in formula.Ok.column, "Property number result");
      equal(
        formula.Ok.column.Number.values,
        [entry.value],
        "Distinct q and backslash-q property IDs",
      );
      propertyRoundtrips.push(entry);
    }
    return { roundtrips, rejected, propertyRoundtrips, synchronous: true, rawTokenFields: true };
  } finally {
    await engine.close();
  }
}

declare global {
  interface Window {
    __formula_worker_contract: typeof runFormulaWorkerContract;
    __formula_worker_failures: typeof runFormulaWorkerFailures;
    __formula_diagnostic_scopes: typeof runFormulaDiagnosticScopes;
    __formula_string_codec: typeof runFormulaStringCodec;
  }
}

window.__formula_worker_contract = runFormulaWorkerContract;
window.__formula_worker_failures = runFormulaWorkerFailures;
window.__formula_diagnostic_scopes = runFormulaDiagnosticScopes;
window.__formula_string_codec = runFormulaStringCodec;
