import { listen } from "@tauri-apps/api/event";
import type { Options } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

import type {
  EventListenOptions,
  EventTransport,
  Unlisten,
} from "@inflatable-cookie/longhorn/core";

import { TauriTransport } from "./index.ts";

/// Optional raw Tauri invoke/listen edge.
export class TauriEventTransport extends TauriTransport implements EventTransport {
  async listen(
    event: string,
    listener: (payload: unknown) => void,
    options?: EventListenOptions,
  ): Promise<Unlisten> {
    if (options?.scope === "renderer-local") {
      const webview = window;
      const receive = (message: Event): void => {
        if ("detail" in message) {
          listener(message.detail);
        }
      };
      webview.addEventListener(event, receive);
      return () => webview.removeEventListener(event, receive);
    }
    const tauriOptions: Options | undefined =
      options?.scope === "current-webview"
        ? { target: { kind: "WebviewWindow", label: getCurrentWebviewWindow().label } }
        : undefined;
    return listen(event, ({ payload }) => listener(payload), tauriOptions);
  }
}
