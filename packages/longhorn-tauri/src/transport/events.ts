import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

import type { EventTransport, Unlisten } from "@inflatable-cookie/longhorn/core";
import { TRANSFER_CLIENT_CHANGED_EVENT } from "@inflatable-cookie/longhorn/transfer";

import { TauriTransport } from "./index.ts";

/// Optional raw Tauri invoke/listen edge.
export class TauriEventTransport extends TauriTransport implements EventTransport {
  listen(event: string, listener: (payload: unknown) => void): Promise<Unlisten> {
    const options =
      event === TRANSFER_CLIENT_CHANGED_EVENT
        ? { target: getCurrentWebviewWindow().label }
        : undefined;
    return listen(event, ({ payload }) => listener(payload), options);
  }
}
