import type {
  OperationCancellationCommand,
  OperationMutationCommand,
  OperationSnapshotQuery,
} from "./generated/protocol.ts";
import type { OperationPort, OperationUnlisten } from "./ports.ts";

export class SerializedOperationPort implements OperationPort {
  readonly #inner: OperationPort;

  constructor(inner: OperationPort) {
    this.#inner = inner;
  }

  async snapshot(query: OperationSnapshotQuery): Promise<unknown> {
    return clone(await this.#inner.snapshot(clone(query)));
  }

  async mutate(command: OperationMutationCommand): Promise<unknown> {
    return clone(await this.#inner.mutate(clone(command)));
  }

  async cancel(command: OperationCancellationCommand): Promise<unknown> {
    return clone(await this.#inner.cancel(clone(command)));
  }

  async listen(listener: (event: unknown) => void): Promise<OperationUnlisten> {
    if (this.#inner.listen === undefined) return () => {};
    return this.#inner.listen((event) => listener(clone(event)));
  }

  nextRequestId(): string {
    return this.#inner.nextRequestId();
  }
}

function clone<Value>(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}
