import { wireCopy } from "../core/wire-copy.ts";
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
    return wireCopy(await this.#inner.snapshot(wireCopy(query)));
  }

  async mutate(command: OperationMutationCommand): Promise<unknown> {
    return wireCopy(await this.#inner.mutate(wireCopy(command)));
  }

  async cancel(command: OperationCancellationCommand): Promise<unknown> {
    return wireCopy(await this.#inner.cancel(wireCopy(command)));
  }

  async listen(listener: (event: unknown) => void): Promise<OperationUnlisten> {
    if (this.#inner.listen === undefined) {
      return () => {};
    }
    return this.#inner.listen((event) => listener(wireCopy(event)));
  }

  nextRequestId(): string {
    return this.#inner.nextRequestId();
  }
}
