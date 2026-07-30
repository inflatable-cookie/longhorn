import { listen } from "@tauri-apps/api/event";

import type { EventTransport, Unlisten } from "@longhorn/core";

import { TauriTransport } from "./index.ts";

/// Optional raw Tauri invoke/listen edge.
export class TauriEventTransport
  extends TauriTransport
  implements EventTransport {
  listen(
    event: string,
    listener: (payload: unknown) => void,
  ): Promise<Unlisten> {
    return listen(event, ({ payload }) => listener(payload));
  }
}
