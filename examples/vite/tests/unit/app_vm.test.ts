import {
  FormulaClientError,
  type CompletionConfig,
  type CreateFormulaEngineClient,
  type CursorHelp,
  type EvaluateInput,
  type EvaluateResult,
  type ExpressionUpdate,
  type FormulaDefinition,
  type FormulaDraftClient,
  type FormulaDraftState,
  type FormulaEdit,
  type FormulaEngineClient,
  type FormulaSchema,
  type PropertyDefinition,
  type PropertyState,
  type QuickFix,
  type RuntimeContext,
  type ValueType,
} from "@notion-formula/sdk";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DEMO_SCHEMA, FORMULA_DEMOS } from "../../src/app/context";
import type { AppState } from "../../src/app/types";
import { AppVM } from "../../src/vm/app_vm";

function deferred<T = void>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

// This fake scripts native results and ownership; it does not parse or evaluate formulas.
class FakeDraft implements FormulaDraftClient {
  version = 0n;
  closed = false;

  constructor(
    readonly engine: FakeEngine,
    readonly definition: FormulaDefinition,
    readonly serial: number,
  ) {}

  private state(): FormulaDraftState {
    return {
      version: this.version,
      definition: { ...this.definition },
      output_type: this.engine.outputType,
      diagnostics: this.engine.invalidSources.has(this.definition.expression)
        ? [
            {
              id: `diagnostic:${this.serial}:${this.version}`,
              span: { start: 0, end: 1 },
              message: "native diagnostic",
            },
          ]
        : [],
      tokens: [
        {
          kind: "native",
          text: this.definition.expression,
          span: { start: 0, end: this.definition.expression.length },
        },
      ],
    };
  }

  private call(method: string): Promise<void> {
    if (this.closed) throw new Error(`Closed draft ${this.serial} used for ${method}`);
    return this.engine.call(`${this.definition.id}:${this.serial}:${method}`);
  }

  readonly getState = vi.fn(async () => {
    await this.call("state");
    return this.state();
  });

  readonly help = vi.fn(async (_cursor: number, _config: CompletionConfig): Promise<CursorHelp> => {
    await this.call("help");
    return {
      base_version: this.version,
      completion: {
        items: [
          {
            label: "native completion",
            kind: "Property",
            insert_text: "native completion",
            primary_edit: {
              range: { start: 0, end: this.definition.expression.length },
              new_text: "native completion",
            },
            additional_edits: [],
            cursor: 5,
            detail: null,
            is_disabled: false,
            disabled_reason: null,
          },
        ],
        replace: { start: 0, end: this.definition.expression.length },
        preferred_indices: [0],
      },
      signature_help: null,
    };
  });

  readonly quickFixes = vi.fn(async (diagnosticId: string): Promise<QuickFix[]> => {
    await this.call("fixes");
    return this.state().diagnostics.some((diagnostic) => diagnostic.id === diagnosticId)
      ? [{ title: "native fix", edit: this.edit("native fix") }]
      : [];
  });

  private edit(source: string): FormulaEdit {
    return {
      base_version: this.version,
      edits: [{ range: { start: 0, end: this.definition.expression.length }, new_text: source }],
    };
  }

  readonly formatEdits = vi.fn(async (): Promise<FormulaEdit> => {
    await this.call("format");
    return this.edit("native format");
  });

  readonly updateExpression = vi.fn(async (update: ExpressionUpdate) => {
    const snapshot = structuredClone(update);
    await this.call("Replace" in snapshot ? "replace" : "edit");
    if ("Replace" in snapshot) {
      this.definition.expression = snapshot.Replace;
    } else {
      if (snapshot.Edits.edit.base_version !== this.version) {
        throw new FormulaClientError({
          code: "UPDATE_EXPRESSION",
          message: "Stale native version",
          payload: { error: "VersionMismatch" },
        });
      }
      // Every scripted edit replaces the whole document; Rust edit validation is tested separately.
      this.definition.expression =
        snapshot.Edits.edit.edits[0]?.new_text ?? this.definition.expression;
    }
    this.version += 1n;
    return { state: this.state(), cursor: 5 };
  });

  readonly intoDefinition = vi.fn(async () => {
    const pending = this.call("consume");
    this.closed = true;
    this.engine.active.delete(this);
    await pending;
    return { ...this.definition };
  });

  readonly close = vi.fn(async () => {
    if (this.closed) return;
    const pending = this.call("close");
    this.closed = true;
    await pending;
    this.engine.active.delete(this);
  });
}

class FakeEngine implements FormulaEngineClient {
  readonly events: string[] = [];
  readonly active = new Set<FakeDraft>();
  readonly drafts: FakeDraft[] = [];
  readonly saved = new Map<string, string>();
  readonly invalidSources = new Set<string>();
  readonly terminate = vi.fn();
  outputType: ValueType = "Number";
  beforeCall: (event: string) => Promise<void> = () => Promise.resolve();
  nextUpsertError: Error | null = null;
  nextEvaluateError: Error | null = null;

  initialize(schema: FormulaSchema): void {
    for (const property of schema.properties) {
      if ("Formula" in property) this.saved.set(property.Formula.id, property.Formula.expression);
    }
  }

  call(event: string): Promise<void> {
    this.events.push(event);
    return this.beforeCall(event);
  }

  readonly createDraft = vi.fn(async (definition: FormulaDefinition) => {
    const snapshot = structuredClone(definition);
    await this.call(`create:${snapshot.id}`);
    const draft = new FakeDraft(this, snapshot, this.drafts.length);
    this.drafts.push(draft);
    this.active.add(draft);
    return draft;
  });

  readonly upsert = vi.fn(async (property: PropertyDefinition) => {
    await this.call("upsert");
    if (this.active.size)
      throw new FormulaClientError({
        code: "ACTIVE_DRAFTS",
        message: "Drafts are active",
        payload: { count: this.active.size },
      });
    if (this.nextUpsertError) {
      const error = this.nextUpsertError;
      this.nextUpsertError = null;
      throw error;
    }
    if ("Formula" in property) this.saved.set(property.Formula.id, property.Formula.expression);
    return { affected_formulas: ["Formula 1", "Formula 2"] };
  });

  readonly evaluate = vi.fn(async (_input: EvaluateInput): Promise<EvaluateResult> => {
    await this.call("evaluate");
    if (this.nextEvaluateError) {
      const error = this.nextEvaluateError;
      this.nextEvaluateError = null;
      throw error;
    }
    return {
      formulas: new Map(
        Array.from(this.saved, ([id, source]) => [
          id,
          this.invalidSources.has(source)
            ? { Err: "NotReady" as const }
            : {
                Ok: {
                  output_type: "String" as const,
                  column: { String: { values: [source], validity: [true] } },
                  errors: [],
                },
              },
        ]),
      ),
    };
  });

  readonly close = vi.fn(async () => {
    await this.call("engine.close");
    for (const draft of this.active) draft.closed = true;
    this.active.clear();
    this.terminate();
  });
  readonly getProperty = vi.fn(
    (_id: string): Promise<PropertyState | null> => Promise.resolve(null),
  );
  readonly getProperties = vi.fn((): Promise<PropertyState[]> => Promise.resolve([]));
  readonly getState = vi.fn((): Promise<"AllReady"> => Promise.resolve("AllReady"));
  readonly remove = vi.fn((_id: string) => Promise.resolve(null));
}

const owned: AppVM[] = [];
afterEach(async () => {
  await Promise.all(owned.splice(0).map((vm) => vm.dispose()));
  vi.useRealTimers();
});

function setup(
  opts: {
    createEngine?: CreateFormulaEngineClient;
    runtime?: RuntimeContext;
    schema?: FormulaSchema;
  } = {},
) {
  const engine = new FakeEngine();
  const states: AppState[] = [];
  const factory = vi.fn((schema: FormulaSchema) => {
    engine.initialize(schema);
    return Promise.resolve(engine);
  });
  const vm = new AppVM({
    onStateChange: (state) => states.push(state),
    createEngine: opts.createEngine ?? factory,
    runtime: opts.runtime ?? { now: 1_700_000_000_000n, time_zone: "+08:00" },
    schema: opts.schema,
  });
  owned.push(vm);
  return { vm, engine, factory, states, latest: () => states[states.length - 1] };
}

function schemaWithFirstSource(source: string): FormulaSchema {
  const schema = structuredClone(DEMO_SCHEMA);
  const property = schema.properties.find(
    (property) => "Formula" in property && property.Formula.id === "Formula 1",
  );
  if (property && "Formula" in property) property.Formula.expression = source;
  return schema;
}

describe("AppVM source buffers and native commands", () => {
  it("starts one engine, two drafts and one evaluation with a captured runtime and all input columns", async () => {
    const runtime = { now: 9_007_199_254_740_993n, time_zone: "+08:00" };
    const { vm, engine, factory, states, latest } = setup({ runtime });
    runtime.now = 0n;
    runtime.time_zone = "+00:00";
    engine.outputType = { Union: ["Number", { List: "String" }] };
    const starting = vm.start();
    expect(vm.start()).toBe(starting);
    await starting;
    expect(factory).toHaveBeenCalledExactlyOnceWith(DEMO_SCHEMA);
    expect(engine.active.size).toBe(2);
    expect(engine.evaluate).toHaveBeenCalledTimes(1);
    const input = engine.evaluate.mock.calls[0][0];
    expect(input.runtime).toEqual({ now: 9_007_199_254_740_993n, time_zone: "+08:00" });
    expect([...input.columns.keys()]).toEqual([
      "Title",
      "Text",
      "Number",
      "Select",
      "Date",
      "Relation",
    ]);
    expect(input.formula_ids).toEqual(["Formula 1", "Formula 2"]);
    expect(latest().formulas["Formula 1"]).toMatchObject({
      source: FORMULA_DEMOS["Formula 1"].sample,
      savedSource: FORMULA_DEMOS["Formula 1"].sample,
      dirty: false,
      version: 0n,
      outputType: engine.outputType,
    });
    expect(latest().formulas["Formula 2"].source).toBe(FORMULA_DEMOS["Formula 2"].sample);
    expect(states[0].wasmReady).toBe(false);
    expect(latest().wasmReady).toBe(true);
  });

  it("publishes typing immediately and debounces only Draft replacement without changing saved results", async () => {
    vi.useFakeTimers();
    const { vm, engine, latest } = setup();
    await vm.start();
    const evaluation = latest().evaluation;
    vm.setSource("Formula 1", "first buffer");
    vm.setSource("Formula 1", "latest buffer");
    vm.setSource("Formula 2", "other buffer");
    expect(latest().formulas["Formula 1"]).toMatchObject({
      source: "latest buffer",
      dirty: true,
      version: null,
      status: "analyzing",
    });
    expect(engine.drafts[0].updateExpression).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(79);
    expect(engine.drafts[0].updateExpression).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    await vm.help("Formula 2", 0);
    expect(engine.drafts[0].updateExpression).toHaveBeenCalledExactlyOnceWith({
      Replace: "latest buffer",
    });
    expect(latest().formulas["Formula 1"].tokens[0].text).toBe("latest buffer");
    expect(latest().evaluation).toEqual(evaluation);
    expect(engine.upsert).not.toHaveBeenCalled();
    expect(engine.evaluate).toHaveBeenCalledTimes(1);
  });

  it("flushes the buffer before formatting and uses native text, version and mapped cursor", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    vm.setSource("Formula 1", "unformatted buffer");
    const result = await vm.format("Formula 1", 2);
    expect(result?.state.definition.expression).toBe("native format");
    expect(latest().formulas["Formula 1"]).toMatchObject({
      source: "native format",
      cursor: 5,
      version: 2n,
      dirty: true,
    });
    expect(engine.drafts[0].updateExpression.mock.calls.map(([update]) => update)).toEqual([
      { Replace: "unformatted buffer" },
      {
        Edits: {
          edit: {
            base_version: 1n,
            edits: [{ range: { start: 0, end: 18 }, new_text: "native format" }],
          },
          cursor: 2,
        },
      },
    ]);
    expect(engine.upsert).not.toHaveBeenCalled();
    expect(engine.evaluate).toHaveBeenCalledTimes(1);
  });

  it("uses native diagnostic IDs, quick fixes and original completion TextEdits", async () => {
    const { vm, engine, latest } = setup();
    engine.invalidSources.add("invalid buffer");
    await vm.start();
    vm.setSource("Formula 1", "invalid buffer");
    await vm.help("Formula 1", 0);
    expect(latest().formulas["Formula 1"].diagnostics).toEqual([
      { id: "diagnostic:0:1", span: { start: 0, end: 1 }, message: "native diagnostic" },
    ]);
    expect(await vm.quickFixes("Formula 1", "foreign ID")).toEqual([]);
    const fixes = await vm.quickFixes(
      "Formula 1",
      latest().formulas["Formula 1"].diagnostics[0].id,
    );
    expect(await vm.applyEdit("Formula 1", fixes[0].edit, 1)).not.toBeNull();
    expect(latest().formulas["Formula 1"].source).toBe("native fix");
    const help = await vm.help("Formula 1", 0);
    const item = help!.completion.items[0];
    const completion = {
      base_version: help!.base_version,
      edits: [item.primary_edit!, ...item.additional_edits],
    };
    expect(await vm.applyEdit("Formula 1", completion, 0)).not.toBeNull();
    expect(latest().formulas["Formula 1"].source).toBe("native completion");
    expect(engine.drafts[0].quickFixes).toHaveBeenCalledWith("foreign ID");
    expect(engine.upsert).not.toHaveBeenCalled();
  });

  it("drops old edits after typing and after Draft recreation even when bigint versions repeat", async () => {
    const { vm, engine, latest } = setup({ schema: schemaWithFirstSource("invalid buffer") });
    engine.invalidSources.add("invalid buffer");
    await vm.start();
    const oldDiagnostic = latest().formulas["Formula 1"].diagnostics[0].id;
    const fixes = await vm.quickFixes("Formula 1", oldDiagnostic);
    expect(fixes[0].edit.base_version).toBe(0n);
    await vm.save("Formula 2");
    expect(latest().formulas["Formula 1"].version).toBe(0n);
    expect(await vm.applyEdit("Formula 1", fixes[0].edit, 0)).toBeNull();
    expect(await vm.quickFixes("Formula 1", oldDiagnostic)).toEqual([]);
    const current = await vm.quickFixes(
      "Formula 1",
      latest().formulas["Formula 1"].diagnostics[0].id,
    );
    vm.setSource("Formula 1", "new typing");
    expect(await vm.applyEdit("Formula 1", current[0].edit, 0)).toBeNull();
    expect(latest().formulas["Formula 1"].source).toBe("new typing");
    expect(
      engine.drafts.every((draft) =>
        draft.updateExpression.mock.calls.every(([update]) => "Replace" in update),
      ),
    ).toBe(true);
  });

  it("does not return help for a source changed during a late native request", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    const gate = deferred();
    engine.beforeCall = (event) => (event.endsWith(":help") ? gate.promise : Promise.resolve());
    const pending = vm.help("Formula 1", 0);
    await vi.waitFor(() => expect(engine.drafts[0].help).toHaveBeenCalledTimes(1));
    vm.setSource("Formula 1", "later typing");
    gate.resolve();
    expect(await pending).toBeNull();
    expect(latest().formulas["Formula 1"].source).toBe("later typing");
    engine.beforeCall = () => Promise.resolve();
    expect(await vm.help("Formula 1", 0)).not.toBeNull();
    expect(latest().formulas["Formula 1"].tokens[0].text).toBe("later typing");
  });

  it("does not apply analysis from a replacement while the buffer keeps changing", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    const gate = deferred();
    engine.beforeCall = (event) => (event.endsWith(":replace") ? gate.promise : Promise.resolve());
    vm.setSource("Formula 1", "first typing");
    const pending = vm.help("Formula 1", 0);
    await vi.waitFor(() => expect(engine.drafts[0].updateExpression).toHaveBeenCalled());
    vm.setSource("Formula 1", "later typing");
    gate.resolve();
    expect(await pending).toBeNull();
    expect(latest().formulas["Formula 1"]).toMatchObject({
      source: "later typing",
      version: null,
      tokens: [],
      outputType: "Unknown",
    });
    engine.beforeCall = () => Promise.resolve();
    await vm.help("Formula 1", 0);
    expect(latest().formulas["Formula 1"].tokens[0].text).toBe("later typing");
  });

  it("preserves typing and cursor when a pending native edit finishes", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    const gate = deferred();
    engine.beforeCall = (event) => (event.endsWith(":edit") ? gate.promise : Promise.resolve());
    const pending = vm.format("Formula 1", 0);
    await vi.waitFor(() => expect(engine.drafts[0].updateExpression).toHaveBeenCalled());
    vm.setSource("Formula 1", "typing during edit");
    gate.resolve();
    expect(await pending).toBeNull();
    expect(latest().formulas["Formula 1"]).toMatchObject({
      source: "typing during edit",
      cursor: null,
      version: null,
    });
    engine.beforeCall = () => Promise.resolve();
    await vm.help("Formula 1", 0);
    expect(engine.drafts[0].definition.expression).toBe("typing during edit");
    expect(engine.evaluate).toHaveBeenCalledTimes(1);
  });

  it("reports a recoverable command failure and continues the command queue", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    engine.drafts[0].help.mockRejectedValueOnce(new Error("native help failed"));
    expect(await vm.help("Formula 1", 0)).toBeNull();
    expect(latest().formulas["Formula 1"].error).toContain("native help failed");
    expect(await vm.help("Formula 1", 0)).not.toBeNull();
    expect(latest().formulas["Formula 1"].error).toBeNull();
  });
});

describe("AppVM save and discard coordination", () => {
  it("consumes one Draft, closes the other, evaluates saved definitions once and restores both buffers", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    vm.setSource("Formula 1", "save this source");
    vm.setSource("Formula 2", "keep this unsaved source");
    await vm.save("Formula 1");
    expect(engine.upsert).toHaveBeenCalledExactlyOnceWith({
      Formula: { id: "Formula 1", expression: "save this source" },
    });
    expect(engine.drafts[0].intoDefinition).toHaveBeenCalledTimes(1);
    expect(engine.drafts[1].close).toHaveBeenCalledTimes(1);
    expect(engine.events.indexOf("upsert")).toBeGreaterThan(
      engine.events.indexOf("Formula 2:1:close"),
    );
    expect(engine.evaluate).toHaveBeenCalledTimes(2);
    expect(latest().formulas["Formula 1"]).toMatchObject({
      source: "save this source",
      savedSource: "save this source",
      dirty: false,
      version: 0n,
    });
    expect(latest().formulas["Formula 2"]).toMatchObject({
      source: "keep this unsaved source",
      savedSource: FORMULA_DEMOS["Formula 2"].sample,
      dirty: true,
      version: 0n,
    });
    expect(latest().evaluation?.formulas.get("Formula 2")).toMatchObject({
      Ok: { column: { String: { values: [FORMULA_DEMOS["Formula 2"].sample] } } },
    });
    expect(engine.active.size).toBe(2);
    expect(latest()).toMatchObject({ saving: false, error: null });
  });

  it("captures the save click before earlier commands finish and preserves later typing in either buffer", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    const helpGate = deferred();
    engine.beforeCall = (event) => (event.endsWith(":help") ? helpGate.promise : Promise.resolve());
    const earlier = vm.help("Formula 2", 0);
    await vi.waitFor(() => expect(engine.drafts[1].help).toHaveBeenCalled());
    vm.setSource("Formula 1", "source at save click");
    const saving = vm.save("Formula 1");
    vm.setSource("Formula 1", "typing after save click");
    vm.setSource("Formula 2", "other later typing");
    helpGate.resolve();
    await earlier;
    await saving;
    expect(engine.saved.get("Formula 1")).toBe("source at save click");
    expect(latest().formulas["Formula 1"]).toMatchObject({
      source: "typing after save click",
      savedSource: "source at save click",
      dirty: true,
    });
    expect(latest().formulas["Formula 2"]).toMatchObject({
      source: "other later typing",
      savedSource: FORMULA_DEMOS["Formula 2"].sample,
      dirty: true,
    });
    expect([...engine.active].map((draft) => draft.definition.expression)).toEqual([
      "typing after save click",
      "other later typing",
    ]);
  });

  it("serializes help, discard and replacement queued during save onto reopened Drafts", async () => {
    vi.useFakeTimers();
    const { vm, engine, latest } = setup();
    await vm.start();
    const gate = deferred();
    const entered = deferred();
    engine.beforeCall = (event) => {
      if (event !== "upsert") return Promise.resolve();
      entered.resolve();
      return gate.promise;
    };
    vm.setSource("Formula 1", "saved click source");
    vm.setSource("Formula 2", "other unsaved source");
    const saving = vm.save("Formula 1");
    await entered.promise;
    expect(engine.active.size).toBe(0);
    vm.setSource("Formula 1", "later selected typing");
    const help = vm.help("Formula 2", 0);
    const discard = vm.discard("Formula 2");
    await vi.advanceTimersByTimeAsync(80);
    gate.resolve();
    await Promise.all([saving, help, discard]);
    expect(await help).not.toBeNull();
    expect(latest().formulas["Formula 1"]).toMatchObject({
      source: "later selected typing",
      savedSource: "saved click source",
      dirty: true,
    });
    expect(latest().formulas["Formula 2"]).toMatchObject({
      source: FORMULA_DEMOS["Formula 2"].sample,
      dirty: false,
    });
    expect(engine.drafts[3].help).toHaveBeenCalledTimes(1);
    expect(engine.drafts[3].close).toHaveBeenCalledTimes(1);
    expect(engine.evaluate).toHaveBeenCalledTimes(2);
    expect(engine.active.size).toBe(2);
    expect(latest().error).toBeNull();
  });

  it("saves native diagnostics and displays a native NotReady evaluation", async () => {
    const { vm, engine, latest } = setup();
    engine.invalidSources.add("invalid saved source");
    await vm.start();
    vm.setSource("Formula 1", "invalid saved source");
    await vm.save("Formula 1");
    expect(latest().formulas["Formula 1"]).toMatchObject({
      savedSource: "invalid saved source",
      dirty: false,
      diagnostics: [{ message: "native diagnostic" }],
    });
    expect(latest().evaluation?.formulas.get("Formula 1")).toEqual({ Err: "NotReady" });
    expect(latest().error).toBeNull();
  });

  it("discards only the selected buffer and leaves saved results and the other Draft unchanged", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    const evaluation = latest().evaluation;
    vm.setSource("Formula 1", "discard this");
    vm.setSource("Formula 2", "keep this");
    await vm.help("Formula 1", 0);
    await vm.help("Formula 2", 0);
    await vm.discard("Formula 1");
    expect(engine.drafts[0].closed).toBe(true);
    expect(engine.drafts[1].closed).toBe(false);
    expect(latest().formulas["Formula 1"]).toMatchObject({
      source: FORMULA_DEMOS["Formula 1"].sample,
      dirty: false,
      version: 0n,
    });
    expect(latest().formulas["Formula 2"]).toMatchObject({ source: "keep this", dirty: true });
    expect(latest().evaluation).toEqual(evaluation);
    expect(engine.upsert).not.toHaveBeenCalled();
    expect(engine.evaluate).toHaveBeenCalledTimes(1);
  });

  it("does not overwrite typing while discard closes and recreates its Draft", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    vm.setSource("Formula 1", "discard this");
    const gate = deferred();
    engine.beforeCall = (event) => (event.endsWith(":close") ? gate.promise : Promise.resolve());
    const discarding = vm.discard("Formula 1");
    await vi.waitFor(() => expect(engine.drafts[0].close).toHaveBeenCalled());
    vm.setSource("Formula 1", "typing during discard");
    gate.resolve();
    await discarding;
    engine.beforeCall = () => Promise.resolve();
    await vm.help("Formula 1", 0);
    expect(latest().formulas["Formula 1"]).toMatchObject({
      source: "typing during discard",
      dirty: true,
    });
    expect(engine.drafts[2].definition.expression).toBe("typing during discard");
  });

  it("keeps saved state after failed upsert, rebuilds buffers and permits another save", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    const evaluation = latest().evaluation;
    vm.setSource("Formula 1", "retry this save");
    vm.setSource("Formula 2", "other dirty buffer");
    engine.nextUpsertError = new Error("native upsert failed");
    await vm.save("Formula 1");
    expect(latest().formulas["Formula 1"]).toMatchObject({
      source: "retry this save",
      savedSource: FORMULA_DEMOS["Formula 1"].sample,
      dirty: true,
    });
    expect(latest().formulas["Formula 2"].source).toBe("other dirty buffer");
    expect(latest().error).toContain("native upsert failed");
    expect(latest().saving).toBe(false);
    expect(latest().evaluation).toEqual(evaluation);
    expect(engine.evaluate).toHaveBeenCalledTimes(1);
    expect(engine.active.size).toBe(2);
    expect(await vm.help("Formula 1", 0)).not.toBeNull();
    await vm.save("Formula 1");
    expect(latest().formulas["Formula 1"]).toMatchObject({
      savedSource: "retry this save",
      dirty: false,
    });
    expect(latest().error).toBeNull();
    expect(engine.evaluate).toHaveBeenCalledTimes(2);
  });

  it("clears old table results when evaluation fails after a successful commit", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    expect(latest().evaluation).not.toBeNull();
    vm.setSource("Formula 1", "committed source");
    engine.nextEvaluateError = new Error("native evaluation failed");
    await vm.save("Formula 1");
    expect(latest().formulas["Formula 1"]).toMatchObject({
      savedSource: "committed source",
      dirty: false,
    });
    expect(latest().evaluation).toBeNull();
    expect(latest().error).toContain("evaluate the saved formulas: native evaluation failed");
    expect(engine.active.size).toBe(2);
    expect(await vm.help("Formula 2", 0)).not.toBeNull();
    await vm.save("Formula 2");
    expect(latest().evaluation).not.toBeNull();
    expect(latest().error).toBeNull();
  });

  it("closes remaining old Drafts when consuming the selected Draft fails", async () => {
    const { vm, engine, latest } = setup();
    await vm.start();
    engine.beforeCall = (event) =>
      event.endsWith(":consume")
        ? Promise.reject(new Error("native serialization failed"))
        : Promise.resolve();
    vm.setSource("Formula 1", "still unsaved");
    await vm.save("Formula 1");
    expect(engine.drafts[1].close).toHaveBeenCalledTimes(1);
    expect(engine.active.size).toBe(2);
    expect(latest().formulas["Formula 1"]).toMatchObject({
      savedSource: FORMULA_DEMOS["Formula 1"].sample,
      source: "still unsaved",
      dirty: true,
    });
    expect(latest().error).toContain("native serialization failed");
    engine.beforeCall = () => Promise.resolve();
    await vm.save("Formula 1");
    expect(latest().error).toBeNull();
  });
});

describe("AppVM lifecycle", () => {
  it("closes an Engine created after disposal during initialization and suppresses callbacks", async () => {
    const engine = new FakeEngine();
    const gate = deferred<FormulaEngineClient>();
    const create = vi.fn((schema: FormulaSchema) => {
      engine.initialize(schema);
      return gate.promise;
    });
    const { vm, states } = setup({ createEngine: create });
    const starting = vm.start();
    await vi.waitFor(() => expect(create).toHaveBeenCalledTimes(1));
    const count = states.length;
    const disposing = vm.dispose();
    expect(vm.dispose()).toBe(disposing);
    vm.setSource("Formula 1", "ignored after dispose");
    gate.resolve(engine);
    await Promise.all([starting, disposing]);
    expect(engine.close).toHaveBeenCalledTimes(1);
    expect(engine.terminate).toHaveBeenCalledTimes(1);
    expect(engine.createDraft).not.toHaveBeenCalled();
    expect(engine.evaluate).not.toHaveBeenCalled();
    expect(states).toHaveLength(count);
    expect(await vm.help("Formula 1", 0)).toBeNull();
    await vm.save("Formula 1");
  });

  it("releases partially initialized Drafts when disposal races draft creation", async () => {
    const { vm, engine, states } = setup();
    const gate = deferred();
    engine.beforeCall = (event) =>
      event === "create:Formula 2" ? gate.promise : Promise.resolve();
    const starting = vm.start();
    await vi.waitFor(() => expect(engine.createDraft).toHaveBeenCalledTimes(2));
    const disposing = vm.dispose();
    const count = states.length;
    gate.resolve();
    await Promise.all([starting, disposing]);
    expect(engine.active.size).toBe(0);
    expect(engine.drafts.every((draft) => draft.closed)).toBe(true);
    expect(engine.terminate).toHaveBeenCalledTimes(1);
    expect(states).toHaveLength(count);
  });

  it("reports initialization rejection and keeps repeated start and dispose idempotent", async () => {
    const error = new FormulaClientError({
      code: "INITIALIZATION_ERROR",
      message: "WASM initialization failed",
      payload: null,
    });
    const create = vi.fn(() => Promise.reject(error));
    const { vm, latest } = setup({ createEngine: create });
    const starting = vm.start();
    await expect(starting).rejects.toBe(error);
    expect(vm.start()).toBe(starting);
    expect(create).toHaveBeenCalledTimes(1);
    expect(latest()).toMatchObject({
      wasmReady: false,
      evaluation: null,
      error: "Unable to start the formula engine: WASM initialization failed",
    });
    const disposing = vm.dispose();
    expect(vm.dispose()).toBe(disposing);
    await disposing;
  });

  it("closes the acquired Engine and every Draft after a later startup failure", async () => {
    const { vm, engine, latest } = setup();
    engine.beforeCall = (event) =>
      event === "Formula 2:1:state"
        ? Promise.reject(new Error("native draft initialization failed"))
        : Promise.resolve();
    await expect(vm.start()).rejects.toThrow("native draft initialization failed");
    expect(latest()).toMatchObject({ wasmReady: false, evaluation: null });
    expect(latest().error).toContain("native draft initialization failed");
    expect(engine.active.size).toBe(0);
    expect(engine.drafts.every((draft) => draft.closed)).toBe(true);
    expect(engine.terminate).toHaveBeenCalledTimes(1);
    await vm.dispose();
    expect(engine.close).toHaveBeenCalledTimes(1);
  });

  it("cancels debounce and queued commands while a late request drains during disposal", async () => {
    vi.useFakeTimers();
    const { vm, engine, states } = setup();
    await vm.start();
    const gate = deferred();
    const entered = deferred();
    engine.beforeCall = (event) => {
      if (!event.endsWith(":help")) return Promise.resolve();
      entered.resolve();
      return gate.promise;
    };
    const help = vm.help("Formula 1", 0);
    await entered.promise;
    vm.setSource("Formula 2", "cancel this debounce");
    const queued = vm.format("Formula 2", 0);
    const disposing = vm.dispose();
    const count = states.length;
    gate.resolve();
    await Promise.all([help, queued, disposing]);
    await vi.advanceTimersByTimeAsync(100);
    expect(await help).toBeNull();
    expect(await queued).toBeNull();
    expect(engine.drafts[1].updateExpression).not.toHaveBeenCalled();
    expect(engine.drafts[1].formatEdits).not.toHaveBeenCalled();
    expect(engine.active.size).toBe(0);
    expect(engine.terminate).toHaveBeenCalledTimes(1);
    expect(states).toHaveLength(count);
  });
});
