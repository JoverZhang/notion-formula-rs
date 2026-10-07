import { FormulaClientError, formulaErrorData, isFormulaErrorData } from "./errors.js";
import type { FormulaTransportErrorCode } from "./errors.js";
import type { FormulaMethod, FormulaOperations, FormulaRequest } from "./protocol.js";

export type FormulaWorker = Pick<
  Worker,
  "postMessage" | "addEventListener" | "removeEventListener" | "terminate"
>;

type Pending = { resolve: (value: unknown) => void; reject: (error: FormulaClientError) => void };

export class FormulaRpc {
  private nextId = 0;
  private readonly pending = new Map<number, Pending>();
  private failure: FormulaClientError | null = null;
  private disposed = false;

  constructor(private readonly worker: FormulaWorker) {
    worker.addEventListener("message", this.onMessage);
    worker.addEventListener("error", this.onError);
    worker.addEventListener("messageerror", this.onMessageError);
  }

  request<Method extends FormulaMethod>(
    method: Method,
    args: FormulaOperations[Method]["args"],
  ): Promise<FormulaOperations[Method]["result"]> {
    if (this.failure) return Promise.reject(this.failure);
    if (this.disposed) {
      return Promise.reject(
        new FormulaClientError({
          code: "WORKER_FAILURE",
          message: "Worker is terminated",
          payload: null,
        }),
      );
    }
    const id = this.nextId++;
    const request = { id, method, args } as FormulaRequest;
    return new Promise<unknown>((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      try {
        this.worker.postMessage(request);
      } catch (error) {
        this.pending.delete(id);
        reject(new FormulaClientError(formulaErrorData(error, "INVALID_REQUEST")));
      }
    }) as Promise<FormulaOperations[Method]["result"]>;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.worker.removeEventListener("message", this.onMessage);
    this.worker.removeEventListener("error", this.onError);
    this.worker.removeEventListener("messageerror", this.onMessageError);
    this.worker.terminate();
  }

  private readonly onMessage = (event: MessageEvent<unknown>): void => {
    const response = event.data;
    if (
      typeof response !== "object" ||
      response === null ||
      !("id" in response) ||
      typeof response.id !== "number" ||
      !Number.isSafeInteger(response.id) ||
      response.id < 0 ||
      !("ok" in response) ||
      typeof response.ok !== "boolean"
    ) {
      this.fail("INVALID_RESPONSE", "Worker returned an invalid response");
      return;
    }
    const pending = this.pending.get(response.id);
    if (!pending) {
      this.fail("INVALID_RESPONSE", "Worker returned an unknown response ID");
      return;
    }
    if (response.ok && "value" in response) {
      this.pending.delete(response.id);
      pending.resolve(response.value);
    } else if (!response.ok && "error" in response && isFormulaErrorData(response.error)) {
      this.pending.delete(response.id);
      pending.reject(new FormulaClientError(response.error));
    } else {
      this.fail("INVALID_RESPONSE", "Worker returned an invalid response");
    }
  };

  private readonly onError = (event: ErrorEvent): void => {
    this.fail("WORKER_FAILURE", event.message || "Worker execution failed");
  };

  private readonly onMessageError = (): void => {
    this.fail("WORKER_FAILURE", "Worker response could not be deserialized");
  };

  private fail(code: FormulaTransportErrorCode, message: string): void {
    if (this.failure) return;
    this.failure = new FormulaClientError({ code, message, payload: null });
    for (const pending of this.pending.values()) pending.reject(this.failure);
    this.pending.clear();
    this.dispose();
  }
}
