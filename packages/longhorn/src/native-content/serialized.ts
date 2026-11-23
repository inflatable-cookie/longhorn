import { wireCopy } from "../core/wire-copy.ts";
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
    return wireCopy(await this.#inner.connect(wireCopy(request)));
  }

  async snapshot(request: NativeContentSnapshotRequest): Promise<unknown> {
    return wireCopy(await this.#inner.snapshot(wireCopy(request)));
  }

  async updateDesired(request: NativeContentDesiredUpdateRequest): Promise<unknown> {
    return wireCopy(await this.#inner.updateDesired(wireCopy(request)));
  }

  async decideContentSize(request: NativeContentContentSizeDecisionRequest): Promise<unknown> {
    return wireCopy(await this.#inner.decideContentSize(wireCopy(request)));
  }

  async listen(listener: (event: unknown) => void): Promise<NativeContentUnlisten> {
    if (this.#inner.listen === undefined) {
      return () => {};
    }
    return this.#inner.listen((event) => listener(wireCopy(event)));
  }

  nextRequestId(): string {
    return this.#inner.nextRequestId();
  }
}
