import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type { EventTransport, Unlisten } from "@longhorn/core";

export class TauriTransport implements EventTransport {
  invoke(
    command: string,
    arguments_: Record<string, unknown>,
  ): Promise<unknown> {
    return invoke(command, arguments_);
  }

  listen(
    event: string,
    listener: (payload: unknown) => void,
  ): Promise<Unlisten> {
    return listen(event, ({ payload }) => listener(payload));
  }
}
