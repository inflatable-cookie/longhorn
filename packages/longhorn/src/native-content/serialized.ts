import type {
  NativeContentConnectRequest,
  NativeContentContentSizeDecisionRequest,
  NativeContentDesiredUpdateRequest,
  NativeContentSnapshotRequest,
} from "./generated/protocol.ts";
import type { NativeContentPort, NativeContentUnlisten } from "./ports.ts";

export class SerializedNativeContentPort implements NativeContentPort {
  readonly #inner: NativeContentPort;

  constructor(inner: NativeContentPort) {
    this.#inner = inner;
  }

  async connect(request: NativeContentConnectRequest): Promise<unknown> {
    return clone(await this.#inner.connect(clone(request)));
  }

  async snapshot(request: NativeContentSnapshotRequest): Promise<unknown> {
    return clone(await this.#inner.snapshot(clone(request)));
  }

  async updateDesired(
    request: NativeContentDesiredUpdateRequest,
  ): Promise<unknown> {
    return clone(await this.#inner.updateDesired(clone(request)));
  }

  async decideContentSize(
    request: NativeContentContentSizeDecisionRequest,
  ): Promise<unknown> {
    return clone(await this.#inner.decideContentSize(clone(request)));
  }

  async listen(
    listener: (event: unknown) => void,
  ): Promise<NativeContentUnlisten> {
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
