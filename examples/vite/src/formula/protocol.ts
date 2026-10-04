import type {
  CompletionConfig,
  CursorHelp,
  DiagnosticId,
  EvaluateInput,
  EvaluateResult,
  ExpressionUpdate,
  FormulaDefinition,
  FormulaClientErrorData,
  FormulaDraftState,
  FormulaEdit,
  FormulaEngineChangeResult,
  FormulaEngineState,
  FormulaSchema,
  PropertyDefinition,
  PropertyId,
  PropertyState,
  QuickFix,
  UpdateExpressionResult,
} from "../engine/generated/wasm_dto";

type Operation<Args extends unknown[], Result> = { args: Args; result: Result };

export type FormulaOperations = {
  initialize: Operation<[FormulaSchema], void>;
  "engine.getProperty": Operation<[PropertyId], PropertyState | null>;
  "engine.getProperties": Operation<[], PropertyState[]>;
  "engine.getState": Operation<[], FormulaEngineState>;
  "engine.upsert": Operation<[PropertyDefinition], FormulaEngineChangeResult>;
  "engine.remove": Operation<[PropertyId], FormulaEngineChangeResult | null>;
  "engine.evaluate": Operation<[EvaluateInput], EvaluateResult>;
  "engine.createDraft": Operation<[FormulaDefinition], number>;
  "engine.close": Operation<[], void>;
  "draft.getState": Operation<[number], FormulaDraftState>;
  "draft.help": Operation<[number, number, CompletionConfig], CursorHelp>;
  "draft.quickFixes": Operation<[number, DiagnosticId], QuickFix[]>;
  "draft.formatEdits": Operation<[number], FormulaEdit>;
  "draft.updateExpression": Operation<[number, ExpressionUpdate], UpdateExpressionResult>;
  "draft.intoDefinition": Operation<[number], FormulaDefinition>;
  "draft.close": Operation<[number], void>;
};

export type FormulaMethod = keyof FormulaOperations;

export type FormulaRequest = {
  [Method in FormulaMethod]: {
    id: number;
    method: Method;
    args: FormulaOperations[Method]["args"];
  };
}[FormulaMethod];

export type FormulaResponse =
  | { id: number; ok: true; value: unknown }
  | { id: number; ok: false; error: FormulaClientErrorData };
