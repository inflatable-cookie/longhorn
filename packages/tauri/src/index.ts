import { invoke } from "@tauri-apps/api/core";

import type { InvokeTransport } from "@inflatable-cookie/longhorn-core";

export class TauriTransport implements InvokeTransport {
  invoke(
    command: string,
    arguments_: Record<string, unknown>,
  ): Promise<unknown> {
    return invoke(command, arguments_);
  }
}
