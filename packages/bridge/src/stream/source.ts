import type { Unlisten } from "@inflatable-cookie/longhorn-core";

import {
  deserialize,
  serialize,
} from "../loopback.ts";

export interface BridgeStreamSource {
  listen(listener: (value: unknown) => void): Promise<Unlisten>;
  loadSnapshot(): Promise<unknown>;
}

export class DirectBridgeStreamSource implements BridgeStreamSource {
  readonly #loadSnapshot: () => unknown | Promise<unknown>;
  readonly #listeners = new Set<(value: unknown) => void>();

  constructor(loadSnapshot: () => unknown | Promise<unknown>) {
    this.#loadSnapshot = loadSnapshot;
  }

  async listen(listener: (value: unknown) => void): Promise<Unlisten> {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  }

  loadSnapshot(): Promise<unknown> {
    return Promise.resolve(this.#loadSnapshot());
  }

  emit(value: unknown): void {
    for (const listener of this.#listeners) {
      listener(value);
    }
  }

  listenerCount(): number {
    return this.#listeners.size;
  }
}

export class SerializedLoopbackBridgeStreamSource
  implements BridgeStreamSource {
  readonly #source: BridgeStreamSource;

  constructor(source: BridgeStreamSource) {
    this.#source = source;
  }

  listen(listener: (value: unknown) => void): Promise<Unlisten> {
    return this.#source.listen((value) => {
      listener(deserialize(serialize(value, "event"), "event"));
    });
  }

  async loadSnapshot(): Promise<unknown> {
    const value = await this.#source.loadSnapshot();
    return deserialize(serialize(value, "snapshot"), "snapshot");
  }
}
