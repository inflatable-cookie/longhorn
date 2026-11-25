import { Channel, invoke } from "@tauri-apps/api/core";

import type { InvokeTransport } from "@inflatable-cookie/longhorn/core";

/** An operational error returned by a Tauri host adapter. */
export class TauriHostError extends Error {
  constructor(
    readonly code: string,
    message: string,
    readonly retryable: boolean,
  ) {
    super(message);
    this.name = "TauriHostError";
  }
}

export class TauriTransport implements InvokeTransport {
  async invoke(command: string, arguments_: Record<string, unknown>): Promise<unknown> {
    try {
      return await invoke(command, arguments_);
    } catch (error) {
      if (isHostErrorEnvelope(error)) {
        throw new TauriHostError(error.code, error.message, error.retryable);
      }
      throw error;
    }
  }

  /** Invokes a command with a channel for its live progress. */
  async invokeWithProgress(
    command: string,
    arguments_: Record<string, unknown>,
    listener: (event: unknown) => void,
  ): Promise<unknown> {
    const onProgress = new Channel(listener);
    try {
      return await this.invoke(command, { ...arguments_, onProgress });
    } finally {
      // A rejected invoke may never create a Rust channel to close the callback.
      const internals: unknown = "__TAURI_INTERNALS__" in window && window.__TAURI_INTERNALS__;
      if (hasCallbackCleanup(internals)) {
        internals.unregisterCallback(onProgress.id);
      }
    }
  }
}

function isHostErrorEnvelope(
  value: unknown,
): value is Pick<TauriHostError, "code" | "message" | "retryable"> {
  return (
    typeof value === "object" &&
    value !== null &&
    Object.keys(value).length === 3 &&
    Object.hasOwn(value, "code") &&
    Object.hasOwn(value, "message") &&
    Object.hasOwn(value, "retryable") &&
    "code" in value &&
    typeof value.code === "string" &&
    "message" in value &&
    typeof value.message === "string" &&
    "retryable" in value &&
    typeof value.retryable === "boolean"
  );
}

function hasCallbackCleanup(value: unknown): value is { unregisterCallback(id: number): void } {
  return (
    typeof value === "object" &&
    value !== null &&
    "unregisterCallback" in value &&
    typeof value.unregisterCallback === "function"
  );
}
