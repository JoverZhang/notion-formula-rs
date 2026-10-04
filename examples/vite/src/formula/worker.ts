import init, { FormulaEngineSession } from "../pkg/analyzer_wasm.js";
import { formulaErrorData } from "./errors";
import type { FormulaRequest, FormulaResponse } from "./protocol";
import { FormulaWorkerRuntime, type FormulaSession } from "./worker_runtime";

const runtime = new FormulaWorkerRuntime(async (schema) => {
  await init();
  return new FormulaEngineSession(schema) as FormulaSession;
});

const scope = globalThis as unknown as {
  addEventListener(type: "message", listener: (event: MessageEvent<FormulaRequest>) => void): void;
  postMessage(response: FormulaResponse): void;
};

scope.addEventListener("message", (event) => {
  void runtime
    .handle(event.data)
    .then((response) => scope.postMessage(response))
    .catch((error: unknown) =>
      scope.postMessage({
        id: event.data.id,
        ok: false,
        error: formulaErrorData(error, "WORKER_FAILURE"),
      }),
    );
});
