import type { FormulaClientErrorData } from "./generated/wasm_dto.js";

export type FormulaTransportErrorCode =
  | "WORKER_FAILURE"
  | "INVALID_RESPONSE"
  | "INVALID_REQUEST"
  | "INITIALIZATION_ERROR";

export class FormulaClientError extends Error {
  readonly data: FormulaClientErrorData;

  constructor(error: FormulaClientErrorData) {
    super(error.message);
    this.name = "FormulaClientError";
    this.data = error;
  }

  get code(): FormulaClientErrorData["code"] {
    return this.data.code;
  }

  get payload(): FormulaClientErrorData["payload"] {
    return this.data.payload;
  }
}

const errorCodes: Record<FormulaClientErrorData["code"], true> = {
  INVALID_DTO: true,
  ENGINE_CLOSED: true,
  DRAFT_CLOSED: true,
  ACTIVE_DRAFTS: true,
  ENGINE_INIT: true,
  ENGINE_CHANGE: true,
  CREATE_DRAFT: true,
  EVALUATE_INPUT: true,
  UPDATE_EXPRESSION: true,
  FORMAT_ERROR: true,
  SERIALIZE_ERROR: true,
  WORKER_FAILURE: true,
  INVALID_RESPONSE: true,
  INVALID_REQUEST: true,
  INITIALIZATION_ERROR: true,
};

export function isFormulaErrorData(value: unknown): value is FormulaClientErrorData {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof value.code === "string" &&
    Object.prototype.hasOwnProperty.call(errorCodes, value.code) &&
    "message" in value &&
    typeof value.message === "string" &&
    "payload" in value
  );
}

export function formulaErrorData(
  error: unknown,
  fallbackCode: FormulaTransportErrorCode,
): FormulaClientErrorData {
  if (isFormulaErrorData(error)) {
    // Error's standard fields are not reliably retained by structured cloning.
    return {
      code: error.code,
      message: error.message,
      payload: error.payload,
    } as FormulaClientErrorData;
  }
  return {
    code: fallbackCode,
    message: error instanceof Error ? error.message : String(error),
    payload: null,
  };
}

export function engineClosedError(): FormulaClientError {
  return new FormulaClientError({
    code: "ENGINE_CLOSED",
    message: "Formula engine is closed",
    payload: null,
  });
}

export function draftClosedError(handle: number): FormulaClientError {
  return new FormulaClientError({
    code: "DRAFT_CLOSED",
    message: "Formula draft is closed",
    payload: { handle },
  });
}
