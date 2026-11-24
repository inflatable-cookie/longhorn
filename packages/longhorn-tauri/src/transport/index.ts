import { Channel, invoke } from "@tauri-apps/api/core";

import type { InvokeTransport } from "@inflatable-cookie/longhorn/core";

export class TauriTransport implements InvokeTransport {
  invoke(command: string, arguments_: Record<string, unknown>): Promise<unknown> {
    return invoke(command, arguments_);
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
      // An older host ignores the argument, so there may be no Rust channel
      // to close the callback. The invocation owns its renderer lifetime.
      const internals: unknown = "__TAURI_INTERNALS__" in window && window.__TAURI_INTERNALS__;
      if (hasCallbackCleanup(internals)) {
        internals.unregisterCallback(onProgress.id);
      }
    }
  }
}

function hasCallbackCleanup(value: unknown): value is { unregisterCallback(id: number): void } {
  return (
    typeof value === "object" &&
    value !== null &&
    "unregisterCallback" in value &&
    typeof value.unregisterCallback === "function"
  );
}
